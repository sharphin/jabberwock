"""Negative tests for concrete LLVM IR auditing."""
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))
import llvm_ir_audit as audit


class LLVMAuditTests(unittest.TestCase):
    def test_missing_ir_is_not_success(self):
        with tempfile.TemporaryDirectory() as d:
            folder = Path(d)
            with mock.patch.object(audit.shutil, "which", return_value="/fake/opt"):
                self.assertEqual(audit.main(["--ir-root", str(folder),
                                             "--output", str(folder / "report.json")]), 1)
            report = json.loads((folder / "report.json").read_text(encoding="utf-8"))
            self.assertEqual(report["status"], "BLOCKED")
            self.assertFalse(report["semantic_refinement_proven"])

    def test_failed_llvm_verifier_blocks_ir(self):
        with tempfile.TemporaryDirectory() as d:
            file = Path(d) / "bad.ll"
            file.write_text("not llvm ir", encoding="utf-8")
            with mock.patch.object(audit.subprocess, "run") as process:
                process.return_value.returncode = 1
                process.return_value.stderr = "parse error"
                result = audit.verify_ir(file, "opt")
            self.assertEqual(result["status"], "INVALID_IR")
            self.assertFalse(result["semantic_refinement_proven"])

    def test_ir_well_formed_does_not_claim_semantic_proof(self):
        with tempfile.TemporaryDirectory() as d:
            file = Path(d) / "sample.ll"
            file.write_text("define i32 @main() { ret i32 0 }", encoding="utf-8")
            with mock.patch.object(audit.subprocess, "run") as process:
                process.return_value.returncode = 0
                process.return_value.stderr = ""
                result = audit.verify_ir(file, "opt")
            self.assertEqual(result["status"], "IR_WELL_FORMED")
            self.assertFalse(result["semantic_refinement_proven"])


if __name__ == "__main__":
    unittest.main()
