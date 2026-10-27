"""Refinement boundary records must be current, complete and never self-proven."""
import hashlib
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))
import semantic_refinement_boundaries as stages


class FirstFiveStagesTests(unittest.TestCase):
    def test_both_compilers_and_five_stages(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            for names in stages.PHASE_SOURCES.values():
                for files in names.values():
                    for filename in files:
                        path = root / filename
                        path.parent.mkdir(parents=True, exist_ok=True)
                        path.write_bytes(filename.encode())
            result = stages.report(root)
            self.assertEqual(result["required"], 10)
            self.assertEqual(result["proven"], 0)
            self.assertEqual(result["status"], "NOT_PROVEN")
            self.assertEqual(len({r["id"] for r in result["obligations"]}), 10)
            for row in result["obligations"]:
                self.assertEqual(row["proof_status"], "NOT_PROVEN")
                for filename, digest in row["sources"].items():
                    self.assertEqual(
                        digest, hashlib.sha256((root / filename).read_bytes()).hexdigest())

    def test_missing_source_blocks_inventory(self):
        with tempfile.TemporaryDirectory() as td:
            with self.assertRaisesRegex(ValueError, "missing"):
                stages.report(Path(td))


if __name__ == "__main__":
    unittest.main()
