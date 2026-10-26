#!/usr/bin/env python3
"""Regression tests for the single-file safety report runner (no compiler needed)."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
import safety_report as report


class SafetyReportTests(unittest.TestCase):
    def test_excerpt_preserves_both_ends(self):
        raw = "FIRST ERROR\n" + "x" * 50000 + "\nFINAL SUMMARY"
        trimmed = report.excerpt(raw)
        self.assertTrue(trimmed["truncated"])
        self.assertIn("FIRST ERROR", trimmed["text"])
        self.assertIn("FINAL SUMMARY", trimmed["text"])
        self.assertEqual(trimmed["sha256"], report.hashlib.sha256(raw.encode()).hexdigest())

    def test_success_only_means_probe_pass(self):
        with tempfile.TemporaryDirectory() as d:
            probe = report.safety.probe("mock-probe", [1], [sys.executable, "-c", "print('OK')"])
            result = report.run_probe(probe, Path(d), False)
            self.assertEqual(result["status"], "PROBE_PASS")
            self.assertEqual(result["actual"]["exit_code"], 0)
            self.assertIn("OK", result["actual"]["stdout"]["text"])

    def test_failure_includes_diagnostic_and_exit(self):
        with tempfile.TemporaryDirectory() as d:
            p = report.safety.probe("mock-probe", [8],
                                    [sys.executable, "-c", "import sys; sys.stderr.write('error: test\\n'); sys.exit(1)"])
            result = report.run_probe(p, Path(d), False)
            self.assertEqual(result["status"], "FAIL")
            self.assertEqual(result["actual"]["exit_code"], 1)
            self.assertIn("error: test", result["actual"]["stderr"]["text"])

    def test_nested_case_failure_is_not_green(self):
        with tempfile.TemporaryDirectory() as d:
            location = Path(d)
            (location / "edges-8-9.json").write_text(json.dumps(
                {"results": [{"item": 8, "case_status": "fail",
                              "observed": {"exit": 2, "stderr": "internal error"}}]}), encoding="utf-8")
            p = report.safety.probe("edges-8-9", [8, 9], [sys.executable, "-c", "pass"])
            result = report.run_probe(p, location, False)
            self.assertEqual(result["status"], "FAIL")
            self.assertEqual(result["case_report"]["results"][0]["observed"]["exit"], 2)

    def test_unavailable_is_not_passed(self):
        with tempfile.TemporaryDirectory() as d:
            p = report.safety.probe("absent-test", [12], ["__nonexistent_safety_executable__"])
            result = report.run_probe(p, Path(d), False)
            self.assertEqual(result["status"], "NOT_RUN")


if __name__ == "__main__":
    unittest.main()
