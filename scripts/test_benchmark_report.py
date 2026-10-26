#!/usr/bin/env python3
"""Unit tests for structured benchmark reports; no compiler or timing run required."""
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))
import benchmark_report as bench


class BenchmarkReportTests(unittest.TestCase):
    def test_parses_existing_markdown_tables(self):
        with tempfile.TemporaryDirectory() as d:
            out = Path(d)
            (out / "time.md").write_text(
                "| 名前 | jabberwock | 検査を全部残す | 検査を全部消す | Go | C++ | C++（境界チェック） | Rust |\n"
                "|---|---|---|---|---|---|---|---|\n"
                "| fib | 0.200 [0.190–0.230] | 0.210 [0.200–0.240] | 0.190 [0.185–0.220] | 0.400 [0.390–0.440] | 0.180 [0.170–0.200] | 0.190 [0.180–0.210] | 0.170 [0.160–0.190] |\n",
                encoding="utf-8")
            (out / "rss.md").write_text(
                "| 名前 | jabberwock | Go | C++ | Rust |\n"
                "|---|---|---|---|---|\n"
                "| fib | 1.7 | 1.8 | 3.7 | 2.2 |\n", encoding="utf-8")
            (out / "checks.md").write_text(
                "| 名前 | a | b | c | d | e | f |\n"
                "|---|---|---|---|---|---|---|\n"
                "| fib | 0 | 0 | 0 | 0 | 0 | 0 |\n", encoding="utf-8")
            case = bench.parse_case(out, "fib")
            self.assertEqual(case["timings"]["jabberwock"]["median_s"], 0.2)
            self.assertEqual(case["checks"]["sites_generation"], 0)
            self.assertAlmostEqual(case["ratios"]["jw_vs_cpp"], 0.2 / 0.18, places=3)

    def test_missing_case_is_error(self):
        with tempfile.TemporaryDirectory() as d:
            with self.assertRaises(ValueError):
                bench.parse_case(Path(d), "missing")

    def test_comparison_flags_regression(self):
        env = {"machine": "same"}
        old = {"environment": env, "runs": 5,
               "cases": {"fib": {"status": "PASS",
                                  "metrics": {"timings": {"jabberwock": {"median_s": 1.0}}}}}}
        now = json.loads(json.dumps(old))
        now["cases"]["fib"]["metrics"]["timings"]["jabberwock"]["median_s"] = 1.2
        result = bench.compare(now, old, 15.0)
        self.assertEqual(result["regressions"], ["fib"])

    def test_different_environments_are_not_comparable(self):
        prior = {"environment": {"machine": "other"}, "runs": 5}
        current = {"environment": {"machine": "current"}, "runs": 5}
        self.assertEqual(bench.compare(current, prior, 15)["status"], "NOT_COMPARABLE")


if __name__ == "__main__":
    unittest.main()
