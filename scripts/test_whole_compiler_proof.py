"""Negative tests for generated, implementation-wide proof obligation coverage."""
import copy
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))
import whole_compiler_proof as proof


class WholeProofInventoryTests(unittest.TestCase):
    def test_inventory_command_succeeds_for_valid_unproved_inventory(self):\n        import tempfile\n        with tempfile.TemporaryDirectory() as d:\n            output = Path(d) / "inventory.json"\n            self.assertEqual(proof.main(["--output", str(output)]), 0)\n            data = __import__("json").loads(output.read_text(encoding="utf-8"))\n            self.assertEqual(data["status"], "NOT_PROVEN")\n            self.assertEqual(data["proved_obligations"], 0)\n\n    def test_all_five_goals_for_both_compilers(self):
        result = proof.inventory()
        proof.validate(result)
        self.assertEqual(result["status"], "NOT_PROVEN")
        self.assertEqual(result["proved_obligations"], 0)
        for compiler in ("jwc", "rust"):
            self.assertEqual({r["goal"] for r in result["components"]
                              if r["compiler"] == compiler}, set(proof.GOALS))

    def test_proof_inventory_covers_compiler_source_snapshot(self):\n        from formal_report import source_identity\n        for compiler in ("jwc", "rust"):\n            with self.subTest(compiler=compiler):\n                self.assertEqual(set(proof.production_files(compiler)),\n                                 set(source_identity(compiler)["files"]))\n\n    def test_build_manifests_are_proof_inputs(self):\n        paths = proof.production_files("rust")\n        self.assertIn("Cargo.toml", paths)\n        self.assertIn("Cargo.lock", paths)\n        obligations = proof.inventory()["components"]\n        for name in ("Cargo.toml", "Cargo.lock"):\n            self.assertEqual({row["goal"] for row in obligations\n                              if row["compiler"] == "rust" and row["source"] == name},\n                             set(proof.GOALS))\n\n    def test_unhashable_identity_fails_closed(self):\n        result = proof.inventory()\n        result["components"][0]["id"] = []\n        with self.assertRaises(ValueError):\n            proof.validate(result)\n\n    def test_missing_or_duplicate_source_obligation_rejected(self):
        result = proof.inventory()
        damaged = copy.deepcopy(result)
        damaged["components"].pop(0)
        with self.assertRaises(ValueError):
            proof.validate(damaged)
        duplicated = copy.deepcopy(result)
        duplicated["components"].append(copy.deepcopy(result["components"][0]))
        with self.assertRaises(ValueError):
            proof.validate(duplicated)

    def test_missing_single_goal_for_existing_source_rejected(self):
        result = proof.inventory()
        damaged = copy.deepcopy(result)
        damaged["components"].pop(2)
        with self.assertRaises(ValueError):
            proof.validate(damaged)

    def test_phase_or_goal_relabelled_rejected(self):
        result = proof.inventory()
        for key, value in (("phase", "integration"), ("goal", "memory"),
                           ("status", "REFINEMENT_PROVEN")):
            damaged = copy.deepcopy(result)
            damaged["components"][0][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                proof.validate(damaged)

    def test_claimed_full_success_metadata_rejected(self):
        result = proof.inventory()
        result["status"] = "PROVEN"
        result["proved_obligations"] = result["total_obligations"]
        with self.assertRaises(ValueError):
            proof.validate(result)

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
