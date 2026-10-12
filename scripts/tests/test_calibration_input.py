"""Exercise calibration input selection through the real command-line entry."""
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


class CalibrationInputTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="uma-calibration-input-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.script = self.root / "scripts" / "calibrate_sim.py"
        self.script.parent.mkdir()
        shutil.copyfile(ROOT / "scripts" / "calibrate_sim.py", self.script)
        tables = self.root / "research" / "training_gain_tables.json"
        tables.parent.mkdir()
        shutil.copyfile(ROOT / "research" / "training_gain_tables.json", tables)
        self.runs = self.root / "runs"
        self.runs.mkdir()
        self.write_records(self.runs / "unrelated.jsonl", gain=10, count=3)

    @staticmethod
    def write_records(path, *, gain, count):
        record = {
            "type": "turn",
            "decision": {"action": "TRAIN", "trainingStat": "speed"},
            "pre": {"mood": "NORMAL", "facilityLevels": {"speed": 1},
                    "supportsOnFacility": 0},
            "post": {"delta": {"stats": {"speed": gain}}},
        }
        path.write_text((json.dumps(record) + "\n") * count, encoding="utf-8")

    def invoke(self, *args):
        def snapshot():
            return {
                str(path.relative_to(self.root)): hashlib.sha256(path.read_bytes()).hexdigest()
                for path in self.root.rglob("*") if path.is_file()
            }

        before = snapshot()
        result = subprocess.run(
            [sys.executable, "-I", "-B", str(self.script), *map(str, args)],
            cwd=self.root, capture_output=True, text=True, check=False, timeout=10,
        )
        self.assertEqual(snapshot(), before, "calibration changed a source, input or saved file")
        return result

    def assert_read_refusal(self, result, selected):
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertEqual(result.stdout, "")
        self.assertIn("Cannot read telemetry", result.stderr)
        self.assertIn(str(selected), result.stderr)
        self.assertNotIn("Traceback", result.stderr)

    def test_missing_explicit_file_does_not_use_default_runs(self):
        selected = self.root / "misspelled.jsonl"
        self.assert_read_refusal(self.invoke("--telemetry", selected), selected)

    def test_missing_explicit_file_does_not_use_supplied_runs(self):
        selected = self.root / "missing" / "wanted.jsonl"
        (self.root / "previous-calibration.json").write_text("previous result\n", encoding="utf-8")
        self.assert_read_refusal(
            self.invoke("--telemetry", selected, "--runs-dir", self.runs), selected,
        )

    def test_directory_selection_has_a_targeted_read_error(self):
        self.assert_read_refusal(self.invoke("--telemetry", self.runs), self.runs)

    def test_non_utf8_selection_has_a_targeted_read_error(self):
        selected = self.root / "invalid-utf8.jsonl"
        selected.write_bytes(b"\xff\n")
        self.assert_read_refusal(self.invoke("--telemetry", selected), selected)

    def test_valid_explicit_file_takes_priority_over_passing_runs(self):
        selected = self.root / "selected.jsonl"
        self.write_records(selected, gain=25, count=2)
        result = self.invoke("--telemetry", selected, "--runs-dir", self.runs)
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertEqual(result.stderr, "")
        self.assertEqual(json.loads(result.stdout), {
            "count": 2, "median_error": 15.0, "pct_within_2": 0.0, "pass": False,
        })

    def test_implicit_runs_scan_retains_its_result_and_file_count(self):
        result = self.invoke()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, "")
        self.assertEqual(json.loads(result.stdout), {
            "count": 3, "median_error": 0.0, "pct_within_2": 1.0,
            "pass": True, "files_scanned": 1,
        })

    def test_empty_selected_file_does_not_use_passing_runs(self):
        selected = self.root / "empty.jsonl"
        selected.write_bytes(b"")
        result = self.invoke("--telemetry", selected)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout), {
            "count": 0, "median_error": None, "pct_within_2": 0.0, "pass": False,
        })
        self.assertIn("no training records", result.stderr)

    def test_stub_retains_precedence_over_both_file_selectors(self):
        result = self.invoke("--stub", "--telemetry", self.root / "missing.jsonl",
                             "--runs-dir", self.runs)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, "")
        document = json.loads(result.stdout)
        self.assertEqual(document["count"], 5)
        self.assertTrue(document["pass"])
        self.assertNotIn("files_scanned", document)


if __name__ == "__main__":
    unittest.main()
