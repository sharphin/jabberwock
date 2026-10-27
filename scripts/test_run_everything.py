"""Unit controls for the single-command Windows/Linux verification driver.

These test stage contracts without executing the actual expensive suite.
"""
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))
import run_everything as full


class FullRunnerTests(unittest.TestCase):
    def test_default_plan_runs_every_required_stage(self):
        output = Path("dummy")
        tasks = full.plans(output)
        self.assertEqual([x[0] for x in tasks], [
            "python-regressions", "practical-safety", "formal-proofs",
            "acceptance-34", "benchmark-four-way", "legacy-optimization-counts"])
        self.assertIn("test_*.py", tasks[0][1])
        self.assertIn("--require-full", tasks[2][1])
        self.assertIn("--full", tasks[4][1])
        self.assertNotIn("--full", full.plans(output, quick=True)[4][1])

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

    def test_runtime_stage_failure_is_retained_with_logs(self):
        with tempfile.TemporaryDirectory() as d:
            folder = Path(d)
            cmd = [sys.executable, "-c", "import sys; print('evidence'); sys.exit(17)"]
            stage = full.execute_stage("controlled-failure", cmd, folder, timeout=10)
            self.assertEqual(stage["status"], "EXIT_NONZERO")
            self.assertEqual(stage["exit_code"], 17)
            self.assertIn("evidence", Path(stage["stdout_log"]).read_text())
            self.assertTrue(Path(stage["stderr_log"]).is_file())


if __name__ == "__main__":
    unittest.main()
