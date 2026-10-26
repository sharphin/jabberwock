#!/usr/bin/env python3
"""Tests for the self-hosted jwc / Rust / C++ report. No benchmark executed."""
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))
import benchmark_report as bench


class BenchmarkReportTests(unittest.TestCase):
    def test_targets_exactly_three(self):
        self.assertEqual(bench.TARGETS, ("jwc", "rust", "cpp"))

    def test_measure_parser(self):
        result = bench.parse_measure("0.12 0.15 0.20 4.5")
        self.assertEqual(result["median_s"], 0.15)
        self.assertEqual(result["peak_rss_mb"], 4.5)

    def test_invalid_measure(self):
        with self.assertRaises(ValueError):
            bench.parse_measure("0.2 0.1 0.3 1")

    def test_regression(self):
        metadata = {"schema_version": 2, "environment": {"os": "linux"},
                    "runs": 5, "compiler_hashes": {"jwc": "same"},
                    "flags": {"rust": "-O3"}}
        baseline = {**metadata, "cases": {"fib": {"status": "PASS", "metrics": {
            "jwc": {"median_s": 1.0}}}}}
        current = {**metadata, "cases": {"fib": {"status": "PASS", "metrics": {
            "jwc": {"median_s": 1.2}}}}}
        self.assertEqual(bench.compare(current, baseline, 15)["regressions"], ["fib"])

    def test_different_compiler_is_not_comparable(self):
        a = {"schema_version": 2, "environment": {}, "runs": 5,
             "compiler_hashes": {"jwc": "new"}, "flags": {}, "cases": {}}
        b = {**a, "compiler_hashes": {"jwc": "old"}}
        self.assertEqual(bench.compare(a, b, 15)["status"], "NOT_COMPARABLE")

    def test_old_format_is_not_comparable(self):
        a = {"schema_version": 2}
        b = {"schema_version": 1}
        self.assertEqual(bench.compare(a, b, 15)["status"], "NOT_COMPARABLE")


if __name__ == "__main__":
    unittest.main()
