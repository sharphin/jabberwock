"""Contract tests for the local toolchain preflight; no toolchains are needed."""
import subprocess
import sys
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))
import toolchain_preflight as preflight


class PreflightContractTests(unittest.TestCase):
    def test_missing_executable_reports_missing_without_executing(self):
        with mock.patch.object(preflight.shutil, "which", return_value=None):
            with mock.patch.object(preflight.subprocess, "run") as run:
                actual = preflight.check("lean", ["lake", "env", "lean", "--version"])
        self.assertEqual(actual["status"], "MISSING")
        run.assert_not_called()

    def test_wrong_lean_version_fails_closed(self):
        completed = subprocess.CompletedProcess(
            ["lake"], 0, stdout="Lean (version 4.25.0, x86_64)\n", stderr="")
        with mock.patch.object(preflight.shutil, "which", return_value="/test/lake"):
            with mock.patch.object(preflight.subprocess, "run", return_value=completed):
                actual = preflight.check("lean", ["lake", "env", "lean", "--version"])
        self.assertEqual(actual["status"], "WRONG_VERSION")

    def test_pinned_lean_version_is_accepted(self):
        completed = subprocess.CompletedProcess(
            ["lake"], 0, stdout="Lean (version 4.24.0, x86_64)\n", stderr="")
        with mock.patch.object(preflight.shutil, "which", return_value="/test/lake"):
            with mock.patch.object(preflight.subprocess, "run", return_value=completed) as run:
                actual = preflight.check("lean", ["lake", "env", "lean", "--version"])
        self.assertEqual(actual["status"], "OK")
        self.assertEqual(run.call_args.kwargs["cwd"], preflight.ROOT / "formal")

    def test_nonzero_exit_never_reports_ready(self):
        completed = subprocess.CompletedProcess(["clang"], 1, stdout="", stderr="bad")
        with mock.patch.object(preflight.shutil, "which", return_value="/test/clang"):
            with mock.patch.object(preflight.subprocess, "run", return_value=completed):
                actual = preflight.check("clang", ["clang", "--version"])
        self.assertEqual(actual["status"], "FAILED")
        self.assertEqual(actual["exit_code"], 1)

    def test_timeout_is_not_ignored(self):
        with mock.patch.object(preflight.shutil, "which", return_value="/test/opt"):
            with mock.patch.object(preflight.subprocess, "run",
                                   side_effect=subprocess.TimeoutExpired(["opt"], 30)):
                actual = preflight.check("opt", ["opt", "--version"])
        self.assertEqual(actual["status"], "FAILED")
        self.assertIn("reason", actual)


if __name__ == "__main__":
    unittest.main()
