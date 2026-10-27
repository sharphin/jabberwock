"""Behavioral tests for the bounded IR semantics oracle."""
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ir_reference_semantics import Machine, Ref, IRTrap, interpret


class IRReferenceSemanticsTests(unittest.TestCase):
    def test_owner_retirement_invalidates_borrow(self):
        program = {"schema_version": 1, "instructions": [
            {"kind": "own", "local": 0, "value": {"kind": "const", "value": 42}},
            {"kind": "borrow", "owner": 0, "local": 1},
            {"kind": "drop", "local": 0, "owns": True},
            {"kind": "show", "value": {"kind": "deref", "ref": {"kind": "copy", "local": 1}}},
        ]}
        outcome = interpret(program)
        self.assertEqual(outcome["status"], "TRAP")
        self.assertEqual(outcome["at"], 3)

    def test_live_borrow_reads_value(self):
        program = {"schema_version": 1, "instructions": [
            {"kind": "own", "local": 0, "value": {"kind": "const", "value": 17}},
            {"kind": "borrow", "owner": 0, "local": 1},
            {"kind": "show", "value": {"kind": "deref", "ref": {"kind": "copy", "local": 1}}},
        ]}
        self.assertEqual(interpret(program), {"status": "OK", "outputs": [17]})

    def test_move_clears_source_local(self):
        program = {"schema_version": 1, "instructions": [
            {"kind": "assign", "local": 0, "value": {"kind": "const", "value": 7}},
            {"kind": "assign", "local": 1, "value": {"kind": "move", "local": 0}},
            {"kind": "show", "value": {"kind": "copy", "local": 0}},
        ]}
        self.assertEqual(interpret(program)["status"], "TRAP")

    def test_unknown_instruction_fails_closed(self):
        self.assertEqual(interpret({"schema_version": 1, "instructions": [
            {"kind": "unimplemented"}]})["status"], "TRAP")

    def test_generation_mismatch_even_if_cell_live(self):
        machine = Machine()
        ref = machine.allocate(10)
        machine.heap[ref.allocation].generation += 1
        with self.assertRaisesRegex(IRTrap, "stale generation"):
            machine.dereference(ref)


if __name__ == "__main__":
    unittest.main()
