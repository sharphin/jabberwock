"""Negative controls for the 34 independent compiler acceptance obligations."""
import copy
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))
import acceptance_report as audit
import acceptance_requirements as req


def fixture():
    items = []
    goals = []
    for compiler in req.COMPILERS:
        for n in range(1, 13):
            items.append({"id": f"windows-{compiler}-{n:02}", "compiler": compiler,
                          "item": n, "target_os": "windows", "probe_names": [f"{compiler}-probe-{n}"],
                          "status": "PROBES_PASS_ONLY", "acceptance": "BLOCKED",
                          "coverage_gaps": ["whole feature verification missing"]})
        for goal in req.GOALS:
            goals.append({"id": compiler + "-" + goal, "compiler": compiler,
                          "goal": goal, "acceptance": "NOT_PROVEN",
                          "proof_check": {"status": "KERNEL_CHECKED"},
                          "source_sha256": compiler + "-sha",
                          "implementation_correspondence": "MANUAL_MODEL_NOT_PROVEN",
                          "excluded": ["unmodelled LLVM"], "theorems": [goal]})
    practical = {"schema_version": 3, "suite": {"target_os": "windows",
                  "compilers": list(req.COMPILERS)}, "run_status": "COMPLETE",
                  "active_probe": None, "items": items}
    formal = {"schema_version": 3, "compilers": {c: {"source": {"sha256": c + "-sha"}}
                                               for c in req.COMPILERS},
              "goals": goals}
    identity = {c: {"sha256": c + "-sha"} for c in req.COMPILERS}
    return practical, formal, identity


class AcceptanceRequirementsTests(unittest.TestCase):
    def check(self, practical, formal, identity):
        return audit.review(practical, formal, local_os="windows", identities=identity)

    def test_complete_catalog_is_24_plus_10(self):
        cases = req.obligation_catalog()
        self.assertEqual(len(cases), 34)
        self.assertEqual(len({c["id"] for c in cases}), 34)
        self.assertEqual(sum(c["kind"] == "practical" for c in cases), 24)
        self.assertEqual(sum(c["kind"] == "formal" for c in cases), 10)
        self.assertTrue(all(c["required_statement"] for c in cases))

    def test_mock_kernel_and_all_probes_still_not_whole_implementation_proof(self):
        practical, formal, identities = fixture()
        result = self.check(practical, formal, identities)
        self.assertEqual(result["status"], "NOT_ACCEPTED")
        self.assertEqual(result["summary"], {"total": 34, "accepted": 0,
                                              "practical": 24, "formal": 10, "with_blockers": 34})
        self.assertTrue(all(x["blockers"] for x in result["obligations"]))

    def test_spoofed_acceptance_fields_never_pass(self):
        practical, formal, identities = fixture()
        for item in practical["items"]:
            item["acceptance"] = "PASS"
            item["coverage_gaps"] = []
        for proof in formal["goals"]:
            proof["acceptance"] = "PROVEN"
            proof["implementation_correspondence"] = "KERNEL_VERIFIED_WHOLE_IMPLEMENTATION"
            proof["excluded"] = []
        result = self.check(practical, formal, identities)
        self.assertEqual(result["status"], "NOT_ACCEPTED")
        self.assertEqual(result["summary"]["accepted"], 0)

    def test_missing_or_duplicate_rows_fail_before_counting(self):
        practical, formal, identity = fixture()
        for change in ("missing_item", "duplicate_item", "missing_goal", "foreign_compiler"):
            p, f = copy.deepcopy((practical, formal))
            if change == "missing_item":
                p["items"].pop()
            elif change == "duplicate_item":
                p["items"][-1] = p["items"][0].copy()
            elif change == "missing_goal":
                f["goals"].pop()
            else:
                f["goals"][0]["compiler"] = "rust"
            with self.subTest(change=change), self.assertRaises(ValueError):
                self.check(p, f, identity)

    def test_wrong_os_rejected(self):
        practical, formal, identities = fixture()
        practical["suite"]["target_os"] = "linux"
        with self.assertRaises(ValueError):
            self.check(practical, formal, identities)

    def test_source_provenance_mismatch_blocks_mathematical_claim(self):
        practical, formal, identities = fixture()
        formal["compilers"]["jwc"]["source"]["sha256"] = "different-snapshot"
        result = self.check(practical, formal, identities)
        jwc = next(x for x in result["obligations"] if x["id"] == "jwc-G-semantics")
        self.assertTrue(any("snapshot differs" in b for b in jwc["blockers"]))

    def test_independent_practical_coverage_gaps_exposed(self):
        practical, formal, identities = fixture()
        practical["items"][0]["coverage_gaps"].append("not validated under concurrency")
        result = self.check(practical, formal, identities)
        first = next(x for x in result["obligations"] if x["id"] == "jwc-S01")
        self.assertTrue(any("concurrency" in b for b in first["blockers"]))


if __name__ == "__main__":
    unittest.main()
