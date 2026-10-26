"""Compiler proof attribution regressions; no real Lean processes in these cases."""
import contextlib
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock
import formal_report as formal

class FormalTests(unittest.TestCase):
    def test_each_compiler_has_five_independent_unproved_obligations(self):
        with tempfile.TemporaryDirectory() as d:
            dest = Path(d) / "formal.json"
            with mock.patch.object(sys, "argv", ["formal_report", "--output", str(dest)]), \
                 mock.patch.object(formal.subprocess, "run") as run, contextlib.redirect_stdout(io.StringIO()):
                code = formal.main()
            data = json.loads(dest.read_text(encoding="utf-8"))
            self.assertEqual(code, 1)
            self.assertEqual(len(data["goals"]), 10)
            for compiler in ("jwc", "rust"):
                self.assertEqual({x["goal"] for x in data["goals"] if x["compiler"] == compiler}, set(formal.GOALS))
            self.assertTrue(all(x["status"] == "NOT_PROVEN" for x in data["goals"]))
            self.assertNotEqual(data["compilers"]["jwc"]["source"]["sha256"], data["compilers"]["rust"]["source"]["sha256"])
            run.assert_not_called()

    def test_jwc_selection_does_not_include_rust_proofs(self):
        with tempfile.TemporaryDirectory() as d:
            dest = Path(d) / "formal.json"
            with mock.patch.object(sys, "argv", ["formal_report", "--compiler", "jwc", "--output", str(dest)]), \
                 contextlib.redirect_stdout(io.StringIO()):
                formal.main()
            data = json.loads(dest.read_text(encoding="utf-8"))
            self.assertEqual(len(data["goals"]), 5)
            self.assertEqual(set(data["compilers"]), {"jwc"})

if __name__ == "__main__":
    unittest.main()
