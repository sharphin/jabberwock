"""Fail-closed local acceptance matrix tests. No compiler or Lean required."""
from pathlib import Path
import sys
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parent))
import verify_all_local as verify


def fixture():
    compilers = list(verify.COMPILERS)
    items = [{"compiler": c, "item": n, "acceptance": "BLOCKED"}
             for c in compilers for n in range(1, 13)]
    proofs = [{"compiler": c, "goal": g, "acceptance": "NOT_PROVEN",
               "proof_check": {"status": "KERNEL_CHECKED"}}
              for c in compilers for g in verify.GOALS]
    practical = {"run_status": "COMPLETE", "suite": {"compilers": compilers}, "items": items}
    formal = {"compilers": {c: {} for c in compilers}, "goals": proofs}
    return practical, formal


class LocalVerifierTests(unittest.TestCase):
    def test_unproved_with_complete_counts(self):
        practical, formal = fixture()
        result = verify.judge(practical, formal)
        self.assertEqual(result["status"], "NOT_PROVEN")
        self.assertEqual((result["required_practical"], result["required_formal"]), (24, 10))

    def test_forged_cross_compiler_or_duplicate_evidence_is_rejected(self):
        for corrupt in ("item", "goal", "compiler"):
            p, f = fixture()
            if corrupt == "item":
                p["items"][-1] = dict(p["items"][0])
            elif corrupt == "goal":
                f["goals"][-1] = dict(f["goals"][0])
            else:
                f["compilers"] = {"jwc": {}}
            self.assertEqual(verify.judge(p, f)["status"], "INVALID_EVIDENCE")

    def test_absent_and_incomplete_reports_are_not_accepted(self):
        p, f = fixture()
        self.assertEqual(verify.judge({}, f)["status"], "INVALID_EVIDENCE")
        p["run_status"] = "RUNNING"
        self.assertEqual(verify.judge(p, f)["status"], "NOT_PROVEN")

    def test_kernel_only_is_never_formal_acceptance(self):
        p, f = fixture()
        for x in p["items"]:
            x["acceptance"] = "PASS"
        self.assertEqual(verify.judge(p, f)["status"], "NOT_PROVEN")
        self.assertEqual(verify.judge(p, f)["formal_accepted"], 0)


if __name__ == "__main__":
    unittest.main()
