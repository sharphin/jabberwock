#!/usr/bin/env python3
"""Regression tests for the single-file safety report runner (no compiler needed)."""
import json
import os
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

    def test_retirement_without_clang_is_not_run(self):
        with tempfile.TemporaryDirectory() as d:
            p = next(p for p in report.safety.catalog(1, Path(d)) if p["name"] == "retirement")
            with mock.patch.object(report.shutil, "which", return_value=None), \
                 mock.patch.object(report.subprocess, "run") as run:
                result = report.run_probe(p, Path(d), False)
            self.assertEqual(result["status"], "NOT_RUN")
            self.assertEqual(result["reason"], "clang unavailable")
            run.assert_not_called()

    def test_native_retirement_requires_execution_and_does_not_regenerate_sources(self):
        with tempfile.TemporaryDirectory() as d:
            p = next(p for p in report.safety.catalog(1, Path(d)) if p["name"] == "retirement")
            completed = subprocess.CompletedProcess(p["cmd"], 0, "", "")
            with mock.patch.object(report.shutil, "which", return_value="/tools/clang"), \
                 mock.patch.dict(os.environ, {"JWC_REGEN": "1"}), \
                 mock.patch.object(report.subprocess, "run", return_value=completed) as run:
                result = report.run_probe(p, Path(d), False)
            self.assertEqual(result["status"], "PROBE_PASS")
            child_env = run.call_args.kwargs["env"]
            self.assertEqual(child_env["JW_REQUIRE_RETIREMENT_TESTS"], "1")
            self.assertNotIn("JWC_REGEN", child_env)

    def test_rust_profiles_run_without_clang_but_disclose_missing_native_coverage(self):
        with tempfile.TemporaryDirectory() as d:
            for p in report.safety.catalog(1, Path(d)):
                if p["name"] not in ("cargo-test", "release-test"):
                    continue
                completed = subprocess.CompletedProcess(p["cmd"], 0, "", "")
                def which(name):
                    return None if name == "clang" else "/tools/" + name
                with mock.patch.dict(os.environ, {"JW_CLANG": "clang"}), \
                     mock.patch.object(report.shutil, "which", side_effect=which), \
                     mock.patch.object(report.subprocess, "run", return_value=completed) as run:
                    result = report.run_probe(p, Path(d), False)
                self.assertEqual(result["status"], "PROBE_PASS")
                self.assertIn("coverage_notes", result)
                self.assertIn("native/jwc/sanitizer", result["coverage_notes"][0])
                run.assert_called_once()


if __name__ == "__main__":
    unittest.main()
