"""Changed-input and failure receiving for the standalone paired report reader."""
from __future__ import annotations

import base64
import copy
import hashlib
from html.parser import HTMLParser
import importlib.util
import json
import os
from pathlib import Path
import statistics
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / "render_paired_comparison.py"
spec = importlib.util.spec_from_file_location("paired_reader", SCRIPT)
reader = importlib.util.module_from_spec(spec)
spec.loader.exec_module(reader)


def fixture():
    # Independent statistics library supplies this authored report's oracle.
    rows = []
    for seed, baseline, current in [(-(1 << 63), 5, 0), (-2, 7, 7),
                                     (0, -2, 3), ((1 << 63) - 1, -(1 << 31), (1 << 31) - 1)]:
        rows.append(dict(scenario="ura", trainee="雪 <&> \"quoted\"\r\nline\t\x00",
                         seed=seed, baseline_score=baseline, input_score=current,
                         score_delta=current-baseline, baseline_grade="F & <tag>", input_grade="=A\x7f"))
    deltas = [r["score_delta"] for r in rows]
    return dict(format="uma-sim.paired-batches.v1", direction="input_minus_baseline",
                identity_fields=["scenario", "trainee", "seed"], comparisons=rows,
                summary=dict(count=4, wins=2, ties=1, losses=1,
                             mean_score_delta=statistics.mean(deltas), median_score_delta=2.5,
                             min_score_delta=-5, max_score_delta=(1 << 32)-1,
                             population_stddev_score_delta=statistics.pstdev(deltas)))


def raw(report):
    return (json.dumps(report, ensure_ascii=False, indent=2) + "\n").encode()


class Table(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.rows = []
        self.cell = None
        self.row = None
        self.download = None
        self.scripts = []
        self.remote = []

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == "tr" and "data-index" in attrs:
            self.row = []
        if tag == "td" and self.row is not None:
            self.cell = ""
        if tag == "a" and "download" in attrs:
            self.download = attrs["href"]
        if tag == "script":
            self.scripts.append(attrs)
        if any(v.startswith(("http:", "https:", "//")) for k, v in attrs.items() if k in ("src", "href")):
            self.remote.append(attrs)

    def handle_data(self, data):
        if self.cell is not None:
            self.cell += data

    def handle_endtag(self, tag):
        if tag == "td" and self.cell is not None:
            self.row.append(self.cell)
            self.cell = None
        if tag == "tr" and self.row is not None:
            self.rows.append(self.row)
            self.row = None


class TestPairedComparisonReader(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source = self.root / "comparison.json"
        self.source.write_bytes(raw(fixture()))
        self.output = self.root / "comparison.html"

    def invoke(self, *extra, digest=None, source=None, output=None):
        path = source or self.source
        return subprocess.run([sys.executable, "-B", str(SCRIPT), "--input", str(path),
                               "--input-sha256", digest or hashlib.sha256(path.read_bytes()).hexdigest(),
                               "--output", str(output or self.output), *extra],
                              capture_output=True, timeout=10)

    def assert_refused(self, result):
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertEqual(result.stdout, b"")
        self.assertEqual(json.loads(result.stderr)["status"], "error")
        self.assertFalse(self.output.exists())

    def test_actual_cli_all_rows_i64_endpoints_literal_fields_and_raw_download(self):
        before = (self.source.read_bytes(), self.source.stat().st_mtime_ns)
        result = self.invoke("--baseline-label", "Earlier <&>", "--input-label", "Later 雪")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, b"")
        receipt = json.loads(result.stdout)
        content = self.output.read_bytes()
        self.assertEqual(receipt["output_sha256"], hashlib.sha256(content).hexdigest())
        self.assertEqual(receipt["pairs"], 4)
        page = Table(); page.feed(content.decode())
        self.assertEqual(len(page.rows), 4)
        self.assertEqual([r[2] for r in page.rows], [str(x) for x in [-(1 << 63), -2, 0, (1 << 63)-1]])
        self.assertEqual([r[5] for r in page.rows], ["-5", "+0", "+5", "+4294967295"])
        self.assertEqual(page.rows[0][1], '雪 <&> "quoted"\r\nline\t⟦U+0000⟧')
        self.assertEqual(page.rows[0][6:], ["F & <tag>", "=A⟦U+007F⟧"])
        self.assertEqual(base64.b64decode(page.download.split(",", 1)[1]), before[0])
        self.assertEqual(page.remote, [])
        self.assertEqual(page.scripts, [{}])
        self.assertIn(b"<noscript>", content)
        self.assertEqual((self.source.read_bytes(), self.source.stat().st_mtime_ns), before)

    def test_complete_statistics_are_reconciled_not_trusted(self):
        for field in fixture()["summary"]:
            with self.subTest(field=field):
                report = fixture(); report["summary"][field] += 1
                with self.assertRaises(ValueError): reader.validate(raw(report))
        report = fixture()
        report["summary"]["population_stddev_score_delta"] *= 1 + 1e-13
        reader.validate(raw(report))
        report["summary"]["population_stddev_score_delta"] *= 1 + 1e-8
        with self.assertRaises(ValueError): reader.validate(raw(report))
        report = fixture()
        for row in report["comparisons"]:
            row["input_score"] = row["baseline_score"]; row["score_delta"] = 0
        report["summary"] = dict(count=4, wins=0, ties=4, losses=0,
                                  min_score_delta=0, max_score_delta=0,
                                  mean_score_delta=0.0, median_score_delta=0.0,
                                  population_stddev_score_delta=-5e-10)
        with self.assertRaisesRegex(ValueError, "nonnegative"):
            reader.validate(raw(report))

    def test_malformed_shape_duplicate_keys_nonfinite_and_bad_utf8_refuse(self):
        examples = [b"[]", b'{"format":1,"format":2}', b'{"a":NaN}', b"\xff", b"{}", b""]
        report = fixture(); report["extra"] = 1; examples.append(raw(report))
        report = fixture(); report["comparisons"][0]["extra"] = 1; examples.append(raw(report))
        report = fixture(); report["summary"]["extra"] = 1; examples.append(raw(report))
        report = fixture(); report["direction"] = "baseline_minus_input"; examples.append(raw(report))
        report = fixture(); report["identity_fields"] = ["seed"]; examples.append(raw(report))
        for value in examples:
            with self.subTest(value=value[:50]):
                self.source.write_bytes(value); self.assert_refused(self.invoke())

    def test_identity_order_duplicates_integer_types_ranges_and_delta(self):
        cases = []
        for field, value in [("seed", True), ("seed", 1.0), ("seed", 1 << 63),
                             ("baseline_score", -(1 << 31)-1), ("input_score", 1 << 31),
                             ("score_delta", 5), ("score_delta", -5.0)]:
            report = fixture(); report["comparisons"][0][field] = value; cases.append(report)
        report = fixture(); report["comparisons"].reverse(); cases.append(report)
        report = fixture(); report["comparisons"][1] = copy.deepcopy(report["comparisons"][0]); cases.append(report)
        report = fixture(); report["comparisons"] = []; cases.append(report)
        for report in cases:
            with self.assertRaises(ValueError): reader.validate(raw(report))

    def test_native_string_byte_bounds_and_invalid_surrogates(self):
        for field, value in [("scenario", ""), ("trainee", "雪" * 86),
                             ("baseline_grade", "a" * 65), ("input_grade", 3),
                             ("trainee", "\ud800")]:
            report = fixture(); report["comparisons"][0][field] = value
            with self.assertRaises(ValueError): reader.validate(json.dumps(report).encode())
        report = fixture()
        for row in report["comparisons"]:
            row["scenario"] = " " * 256; row["baseline_grade"] = " " * 64
        reader.validate(raw(report))

    def test_wrong_digest_existing_output_alias_and_symlink_preserve_inputs(self):
        before = (self.source.read_bytes(), self.source.stat().st_mtime_ns)
        self.assert_refused(self.invoke(digest="0" * 64))
        self.assert_refused(self.invoke(digest="not-a-hash"))
        self.output.write_bytes(b"existing output")
        out_before = (self.output.read_bytes(), self.output.stat().st_mtime_ns)
        result = self.invoke(); self.assertEqual(result.returncode, 2)
        self.assertEqual((self.output.read_bytes(), self.output.stat().st_mtime_ns), out_before)
        result = self.invoke(output=self.source); self.assertEqual(result.returncode, 2)
        self.assertEqual((self.source.read_bytes(), self.source.stat().st_mtime_ns), before)
        self.output.unlink()
        if hasattr(os, "symlink"):
            link = self.root / "source-link.json"
            try: link.symlink_to(self.source)
            except OSError: return  # Windows may require privilege for symlink creation.
            self.assert_refused(self.invoke(source=link))
            self.output.symlink_to(self.root / "absent.html")
            self.assertEqual(self.invoke().returncode, 2)
            self.assertTrue(self.output.is_symlink())
            self.assertFalse((self.root / "absent.html").exists())

    def test_changed_input_and_failed_output_cleanup(self):
        digest = hashlib.sha256(self.source.read_bytes()).hexdigest()
        original = reader.read_regular
        calls = 0
        def changed(path):
            nonlocal calls
            calls += 1
            data, identity = original(path)
            if calls == 2: return data + b" ", identity
            return data, identity
        with patch.object(reader, "read_regular", side_effect=changed):
            with self.assertRaisesRegex(ValueError, "changed before"):
                reader.create_report(self.source, digest, self.output)
        self.assertFalse(self.output.exists())
        calls = 0
        def changed_after(path):
            nonlocal calls
            calls += 1
            data, identity = original(path)
            if calls == 3: return data + b" ", identity
            return data, identity
        with patch.object(reader, "read_regular", side_effect=changed_after):
            with self.assertRaisesRegex(ValueError, "changed during"):
                reader.create_report(self.source, digest, self.output)
        self.assertFalse(self.output.exists())
        with patch.object(reader.os, "fsync", side_effect=OSError("authored write failure")):
            with self.assertRaises(OSError): reader.create_report(self.source, digest, self.output)
        self.assertFalse(self.output.exists())
        self.assertEqual(hashlib.sha256(self.source.read_bytes()).hexdigest(), digest)

    def test_input_limit_labels_and_cli_argument_refusal(self):
        with patch.object(reader, "MAX_BYTES", 32):
            with self.assertRaisesRegex(ValueError, "64 MiB"): reader.read_regular(self.source)
        self.assert_refused(self.invoke("--baseline-label", "x\nchanged"))
        self.assert_refused(self.invoke("--input-label", "雪" * 43))
        self.assert_refused(self.invoke("--unknown-option"))


if __name__ == "__main__":
    unittest.main()
