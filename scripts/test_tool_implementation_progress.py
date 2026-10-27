"""Checks that tool implementation is never confused with target proof acceptance."""
import sys
import unittest
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from tool_implementation_progress import CAPABILITIES, assess

class ToolProgressTests(unittest.TestCase):
    def entries(self, status):
        return {name: {"state": status, "evidence": "scripts/example.py: checked implementation"} for name in CAPABILITIES}

    def test_unassessed_is_not_complete(self):
        result = assess(self.entries("UNKNOWN"))
        self.assertEqual(result["percent"], 0)
        self.assertFalse(result["implementation_complete"])
        self.assertEqual(result["target_proof_acceptance"], "NOT_ASSESSED")
        self.assertEqual(result["tool_tests"], "NOT_ASSESSED")

    def test_complete_tool_does_not_assert_target_is_proven(self):
        result = assess(self.entries("IMPLEMENTED"))
        self.assertTrue(result["implementation_complete"])
        self.assertEqual(result["percent"], 100)
        self.assertEqual(result["target_proof_acceptance"], "NOT_ASSESSED")

    def test_unknown_capabilities_rejected(self):
        rows = self.entries("UNKNOWN")
        rows.pop(next(iter(rows)))
        with self.assertRaises(ValueError):
            assess(rows)

    def test_unsubstantiated_implemented_rejected(self):
        rows = self.entries("IMPLEMENTED")
        rows[next(iter(rows))]["evidence"] = ""
        with self.assertRaises(ValueError):
            assess(rows)

if __name__ == "__main__":
    unittest.main()
