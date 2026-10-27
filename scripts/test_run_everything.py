"""Unit controls for the single-command Windows/Linux verification driver.

These test stage contracts without executing the actual expensive suite.
"""
from pathlib import Path
import sys
import tempfile
import json
import hashlib
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))
import run_everything as full


class FullRunnerTests(unittest.TestCase):
    def test_inventory_is_not_an_execution_failure(self):
        with tempfile.TemporaryDirectory() as d:
            output = Path(d)
            inventory = {"status": "NOT_PROVEN", "total_obligations": 1,
                         "proved_obligations": 0,
                         "components": [{"id": "jwc:semantics:jwc/main.jw",
                                         "status": "REFINEMENT_NOT_PROVEN"}]}
            (output / "whole-proof-inventory.json").write_text(json.dumps(inventory))
            stages = [{"name": name, "status": "EXIT_ZERO", "exit_code": 0}
                      for name, _ in full.plans(output)]
            summary = full.summarize(stages, output, quick=False)
            self.assertEqual(summary["status"], "FAIL")
            self.assertEqual(summary["evidence"]["implementation_inventory"]["proved"], 0)
            self.assertNotEqual(summary["reason"], "one or more verification stages failed")

    def test_default_plan_runs_every_required_stage(self):
        output = Path("dummy")
        tasks = full.plans(output)
        self.assertEqual([x[0] for x in tasks], [
            "python-regressions", "cargo-tests", "cargo-doctests", "cargo-clippy", "practical-safety", "whole-implementation-obligations", "refinement-transition", "build-dependency-audit", "emitter-proof-binding", "formal-proofs",
            "acceptance-34", "benchmark-four-way", "llvm-concrete-ir-audit", "legacy-optimization-counts"])
        self.assertIn("test_*.py", tasks[0][1])
        self.assertEqual(dict(tasks)["cargo-tests"], ["cargo", "test", "--offline", "--all-targets"])
        self.assertEqual(dict(tasks)["cargo-doctests"], ["cargo", "test", "--offline", "--doc"])
        self.assertEqual(dict(tasks)["cargo-clippy"], ["cargo", "clippy", "--offline", "--all-targets", "--", "-D", "warnings"])
        self.assertIn("--require-full", dict(tasks)["formal-proofs"])
        self.assertIn("--benchmark-report", dict(tasks)["llvm-concrete-ir-audit"])
        self.assertIn("refinement-transition", dict(tasks))
        self.assertIn("--full", dict(tasks)["benchmark-four-way"])
        self.assertNotIn("--full", dict(full.plans(output, quick=True))["benchmark-four-way"])

    def test_windows_launches_batches_and_uses_python_for_formal(self):
        with mock.patch.object(full, "WINDOWS", True):
            steps = dict(full.plans(Path("out")))
            self.assertEqual(steps["practical-safety"][:3], ["cmd", "/d", "/c"])
            self.assertTrue(steps["practical-safety"][3].endswith("safety_windows.cmd"))
            self.assertTrue(steps["benchmark-four-way"][3].endswith("benchmark_windows.cmd"))
            self.assertEqual(steps["formal-proofs"][0], sys.executable)

    def test_missing_or_failed_stage_never_passes(self):
        with tempfile.TemporaryDirectory() as d:
            folder = Path(d)
            names = [n for n, _ in full.plans(folder)]
            stages = [{"name": n, "status": "EXIT_ZERO", "exit_code": 0} for n in names]
            self.assertEqual(full.summarize(stages, folder, quick=False)["status"], "FAIL")
            self.assertEqual(full.summarize(stages[:-1], folder, quick=False)["status"], "FAIL")
            stages[1]["status"] = "EXIT_NONZERO"
            self.assertEqual(full.summarize(stages, folder, quick=False)["status"], "FAIL")

    def test_unproved_implementation_links_appear_even_on_nonzero_stage(self):
        with tempfile.TemporaryDirectory() as d:
            output = Path(d)
            report = {"status": "NOT_PROVEN", "total_obligations": 2,
                      "proved_obligations": 0,
                      "components": [
                          {"id": "jwc:types:jwc/typeck.jw", "status": "REFINEMENT_NOT_PROVEN"},
                          {"id": "rust:memory:runtime/jw_rt.c", "status": "REFINEMENT_NOT_PROVEN"}
                      ]}
            (output / "whole-proof-inventory.json").write_text(json.dumps(report))
            names = [name for name, _ in full.plans(output)]
            failed = [{"name": name, "status": "EXIT_NONZERO", "exit_code": 1}
                      for name in names]
            result = full.summarize(failed, output, quick=False)
            self.assertEqual(result["status"], "FAIL")
            evidence = result["evidence"]["implementation_inventory"]
            self.assertEqual(evidence["required"], 2)
            self.assertEqual(len(evidence["uncovered"]), 2)
            self.assertEqual(set(result["failed_stages"]), set(names))

    def test_no_summary_pass_with_duplicate_stage(self):
        with tempfile.TemporaryDirectory() as d:
            output = Path(d)
            names = [name for name, _ in full.plans(output)]
            stages = [{"name": name, "status": "EXIT_ZERO", "exit_code": 0}
                      for name in names]
            stages[0] = stages[1].copy()
            self.assertEqual(full.summarize(stages, output, quick=False)["status"], "FAIL")

    def test_actual_failure_is_not_masked_by_unproved_refinement(self):
        with tempfile.TemporaryDirectory() as d:
            folder = Path(d)
            (folder / "whole-proof-inventory.json").write_text(json.dumps({
                "status": "NOT_PROVEN", "total_obligations": 1,
                "proved_obligations": 0, "components": [{
                    "id": "jwc:semantics:jwc/main.jw",
                    "status": "REFINEMENT_NOT_PROVEN"}]}), encoding="utf-8")
            stages = [{"name": name, "status": "EXIT_ZERO", "exit_code": 0}
                      for name, _ in full.plans(folder)]
            stages[0]["status"] = "EXIT_NONZERO"
            stages[0]["exit_code"] = 7
            result = full.summarize(stages, folder, quick=False)
            self.assertEqual(result["status"], "FAIL")
            self.assertEqual(result["failed_stages"], ["python-regressions"])
            self.assertIn("python-regressions", result["reason"])

    def test_cargo_failure_blocks_full_verdict(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            stages = [{"name": name, "status": "EXIT_ZERO", "exit_code": 0}
                      for name, _ in full.plans(output)]
            cargo = next(stage for stage in stages if stage["name"] == "cargo-tests")
            cargo["status"] = "EXIT_NONZERO"
            cargo["exit_code"] = 101
            report = full.summarize(stages, output, quick=False)
            self.assertEqual(report["status"], "FAIL")
            self.assertIn("cargo-tests", report["failed_stages"])
            self.assertIn("cargo-tests", report["reason"])

    def test_every_cargo_stage_failure_is_blocking(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            for target in ("cargo-tests", "cargo-doctests", "cargo-clippy"):
                stages = [{"name": name, "status": "EXIT_ZERO", "exit_code": 0}
                          for name, _ in full.plans(output)]
                broken = next(stage for stage in stages if stage["name"] == target)
                broken["status"] = "EXIT_NONZERO"
                broken["exit_code"] = 101
                with self.subTest(stage=target):
                    result = full.summarize(stages, output, quick=False)
                    self.assertEqual(result["status"], "FAIL")
                    self.assertIn(target, result["failed_stages"])

    def test_stage_log_integrity_detects_tampering_and_missing_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            stage = full.execute_stage(
                "sealed-stage", [sys.executable, "-c", "print('original')"],
                folder, timeout=10)
            self.assertTrue(full.verify_stage_log_integrity(stage))
            path = Path(stage["stdout_log"])
            path.write_text("changed", encoding="utf-8")
            self.assertFalse(full.verify_stage_log_integrity(stage))
            path.unlink()
            self.assertFalse(full.verify_stage_log_integrity(stage))
            self.assertFalse(full.verify_stage_log_integrity(
                {"name": "forged", "status": "EXIT_ZERO", "exit_code": 0}))

    def test_all_exit_zero_without_sealed_logs_cannot_pass(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            stages = [{"name": name, "status": "EXIT_ZERO", "exit_code": 0}
                      for name, _ in full.plans(folder)]
            verdict = full.summarize(stages, folder, quick=False)
            self.assertEqual(verdict["status"], "FAIL")
            self.assertEqual(set(verdict["invalid_stage_logs"]),
                             {name for name, _ in full.plans(folder)})

    def test_false_success_with_nonzero_or_missing_exit_code_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            for bad_code in (None, 1, 101):
                stages = [{"name": name, "status": "EXIT_ZERO", "exit_code": 0}
                          for name, _ in full.plans(output)]
                forged = next(x for x in stages if x["name"] == "cargo-tests")
                if bad_code is None:
                    del forged["exit_code"]
                else:
                    forged["exit_code"] = bad_code
                with self.subTest(exit_code=bad_code):
                    result = full.summarize(stages, output, quick=False)
                    self.assertEqual(result["status"], "FAIL")
                    self.assertIn("cargo-tests", result["failed_stages"])

    def test_sealed_logs_must_belong_to_the_named_stage(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            good = full.execute_stage(
                "authentic", [sys.executable, "-c", "print('ok')"],
                output, timeout=10)
            self.assertTrue(full.verify_stage_log_integrity(good, output))
            forged = dict(good)
            forged["name"] = "other"
            self.assertFalse(full.verify_stage_log_integrity(forged, output))
            forged = dict(good)
            forged["stdout_log"] = str(output / "missing.log")
            self.assertFalse(full.verify_stage_log_integrity(forged, output))

    def test_runtime_stage_failure_is_retained_with_logs(self):
        with tempfile.TemporaryDirectory() as d:
            folder = Path(d)
            cmd = [sys.executable, "-c", "import sys; print('evidence'); sys.exit(17)"]
            stage = full.execute_stage("controlled-failure", cmd, folder, timeout=10)
            self.assertEqual(stage["status"], "EXIT_NONZERO")
            self.assertEqual(stage["exit_code"], 17)
            self.assertIn("evidence", Path(stage["stdout_log"]).read_text())
            self.assertTrue(Path(stage["stderr_log"]).is_file())
            self.assertEqual(stage["stdout_sha256"], hashlib.sha256(Path(stage["stdout_log"]).read_bytes()).hexdigest())
            self.assertEqual(stage["stderr_sha256"], hashlib.sha256(Path(stage["stderr_log"]).read_bytes()).hexdigest())
            self.assertEqual(stage["stdout_bytes"], Path(stage["stdout_log"]).stat().st_size)
            self.assertEqual(stage["stderr_bytes"], Path(stage["stderr_log"]).stat().st_size)


if __name__ == "__main__":
    unittest.main()
