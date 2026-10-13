"""Public CLI regression checks for top-level calibration record admission."""
from __future__ import annotations

import hashlib
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EMPTY = {"count": 0, "median_error": None, "pct_within_2": 0.0, "pass": False}
PASS = {"count": 2, "median_error": 1.0, "pct_within_2": 1.0, "pass": True}
WARNING = "WARN: no training records with main_gain found\n"


def row(gain):
    return {
        "type": "turn",
        "decision": {"action": "TRAIN", "trainingStat": "speed"},
        "pre": {"facilityLevels": {"speed": 1}, "mood": "NORMAL",
                "supportsOnFacility": 0},
        "post": {"delta": {"stats": {"speed": gain}}},
    }


class CalibrationRecordShapeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="uma record shape ")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.script = self.root / "source" / "scripts" / "calibrate_sim.py"
        self.script.parent.mkdir(parents=True)
        shutil.copyfile(ROOT / "scripts" / "calibrate_sim.py", self.script)
        table = self.script.parents[1] / "research" / "training_gain_tables.json"
        table.parent.mkdir()
        shutil.copyfile(ROOT / "research" / "training_gain_tables.json", table)
        self.work = self.root / "unrelated working directory"
        self.work.mkdir()

    def write(self, name, lines):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(("\n".join(lines) + "\n").encode("utf-8"))
        return path

    def invoke(self, *args):
        def snapshot():
            return {
                str(p.relative_to(self.root)):
                    (hashlib.sha256(p.read_bytes()).hexdigest(), p.stat().st_mtime_ns)
                for p in self.root.rglob("*") if p.is_file()
            }
        before = snapshot()
        command = [sys.executable, "-I", "-B"]
        if sys.flags.optimize:
            command.append("-O")
        result = subprocess.run(
            command + [str(self.script), *map(str, args)],
            cwd=self.work, capture_output=True, timeout=10, check=False,
        )
        self.assertEqual(snapshot(), before, "source, table or input changed")
        return result

    def selected(self, name, noise, gains=(10, 12)):
        lines = [json.dumps(row(gains[0])), *noise, json.dumps(row(gains[1]))]
        return self.invoke("--telemetry", self.write(name, lines))

    def assert_report(self, result, expected, exit_code=0, stderr=""):
        self.assertEqual(result.returncode, exit_code,
                         result.stderr.decode("utf-8", "backslashreplace"))
        self.assertEqual(json.loads(result.stdout), expected)
        self.assertEqual(result.stderr.decode().replace("\r\n", "\n"), stderr)

    def test_arrays_do_not_become_records_or_unpack_into_samples(self):
        result = self.selected("array values.jsonl",
                               ["[]", "[1]", json.dumps([row(100)]), "[[1],{}]"])
        self.assert_report(result, PASS)

    def test_every_scalar_class_is_skipped(self):
        values = [None, False, True, 0, 4, -3, 0.5, "", "not a record"]
        result = self.selected("scalar values.jsonl", list(map(json.dumps, values)))
        self.assert_report(result, PASS)

    def test_legacy_skips_keep_successful_output_bytes(self):
        healthy = self.selected("healthy.jsonl", [])
        noisy = self.selected("legacy skips.jsonl",
                              ["", "  ", "{", "{}", "null", "false", "0",
                               '""', "[]", '{"type":"run_start"}'])
        self.assert_report(healthy, PASS)
        self.assertEqual((noisy.returncode, noisy.stdout, noisy.stderr),
                         (healthy.returncode, healthy.stdout, healthy.stderr))

    def test_nonobjects_do_not_hide_a_genuine_calibration_failure(self):
        result = self.selected("failing gains.jsonl", ["true", "[1]"], gains=(10, 16))
        self.assert_report(result, {
            "count": 2, "median_error": 3.0, "pct_within_2": 0.5, "pass": False,
        }, exit_code=2)

    def test_selected_only_skipped_values_keeps_empty_report_policy(self):
        path = self.write("only skipped.jsonl",
                          ["[1]", '"noise"', "1", "true", "null", "{}", "{", ""])
        result = self.invoke("--telemetry", path)
        self.assert_report(result, EMPTY, stderr=WARNING)

    def test_implicit_scan_keeps_all_selected_files_and_samples(self):
        self.write("runs/first.jsonl", [json.dumps(row(10)), "[1]"])
        self.write("runs/second.jsonl", ['"noise"', json.dumps(row(12))])
        result = self.invoke("--runs-dir", self.root / "runs")
        self.assert_report(result, dict(PASS, files_scanned=2))

    def test_implicit_scan_with_no_loaded_records_retains_refusal(self):
        self.write("empty runs/only.jsonl", ["[1]", "true", "{}", "{"])
        result = self.invoke("--runs-dir", self.root / "empty runs")
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stdout, b"")
        self.assertIn(b"No telemetry JSONL found under", result.stderr)
        self.assertNotIn(b"Traceback", result.stderr)

    def test_nested_dictionary_errors_are_not_silently_discarded(self):
        result = self.selected("nested invalid dictionary.jsonl",
                               ['{"type":"turn","decision":[1]}'])
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stdout, b"")
        self.assertIn(b"AttributeError", result.stderr)


if __name__ == "__main__":
    unittest.main()
