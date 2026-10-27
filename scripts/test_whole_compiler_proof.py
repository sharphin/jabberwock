"""Negative tests for generated, implementation-wide proof obligation coverage."""
import copy
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))
import whole_compiler_proof as proof


class WholeProofInventoryTests(unittest.TestCase):
    def test_all_five_goals_for_both_compilers(self):
        result = proof.inventory()
        proof.validate(result)
        self.assertEqual(result["status"], "NOT_PROVEN")
        self.assertEqual(result["proved_obligations"], 0)
        for compiler in ("jwc", "rust"):
            self.assertEqual({r["goal"] for r in result["components"]
                              if r["compiler"] == compiler}, set(proof.GOALS))

    def test_missing_or_duplicate_source_obligation_rejected(self):
        result = proof.inventory()
        damaged = copy.deepcopy(result)
        damaged["components"].pop(0)
        with self.assertRaises(ValueError):
            proof.validate(damaged)
        duplicated = copy.deepcopy(result)
        duplicated["components"].append(copy.deepcopy(result["components"][0]))
        with self.assertRaises(ValueError):
            proof.validate(duplicated)

    def test_tampered_source_hash_rejected(self):
        result = proof.inventory()
        damaged = copy.deepcopy(result)
        damaged["components"][0]["source_sha256"] = "forged"
        with self.assertRaises(ValueError):
            proof.validate(damaged)

    def test_unrecognized_source_still_generates_obligations(self):
        self.assertEqual(proof.phase_of("jwc/new_optimizer.jw"), "integration")
        self.assertEqual(set(proof.BOUNDARIES["integration"]), set(proof.GOALS))


if __name__ == "__main__":
    unittest.main()
