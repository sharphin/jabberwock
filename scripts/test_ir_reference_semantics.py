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


    def test_cfg_branch_and_return(self):
        machine = Machine()
        blocks = [
            {"insts": [], "term": {"kind": "branch",
                "condition": {"kind": "const", "value": True}, "yes": 1, "no": 2}},
            {"insts": [], "term": {"kind": "return",
                "value": {"kind": "const", "value": 42}}},
            {"insts": [], "term": {"kind": "fail", "message": "wrong path"}},
        ]
        self.assertEqual(machine.run_cfg(blocks)["value"], 42)

    def test_cfg_infinite_loop_exhausts_fuel(self):
        machine = Machine()
        result = machine.run_cfg(
            [{"insts": [], "term": {"kind": "goto", "target": 0}}],
            max_steps=3)
        self.assertEqual(result["status"], "TRAP")
        self.assertIn("fuel", result["reason"])

    def test_cfg_unreachable_is_not_success(self):
        machine = Machine()
        result = machine.run_cfg(
            [{"insts": [], "term": {"kind": "unreachable"}}])
        self.assertEqual(result["status"], "TRAP")

    def test_cfg_step_overflow_uses_done_edge(self):
        machine = Machine(locals={0: 127})
        blocks = [
            {"insts": [], "term": {"kind": "step", "counter": 0,
                "amount": {"kind": "const", "value": 1},
                "min": -128, "max": 127, "next": 1, "done": 2}},
            {"insts": [], "term": {"kind": "fail", "message": "wrong step"}},
            {"insts": [], "term": {"kind": "return",
                "value": {"kind": "copy", "local": 0}}},
        ]
        result = machine.run_cfg(blocks)
        self.assertEqual(result["status"], "RETURN")
        self.assertEqual(result["value"], 127)



    def test_rvalue_arithmetic_and_division_truncates_to_zero(self):
        machine = Machine()
        self.assertEqual(machine.evaluate({"kind": "bin", "op": "div",
            "left": {"kind": "const", "value": -7},
            "right": {"kind": "const", "value": 3}}), -2)
        self.assertEqual(machine.evaluate({"kind": "bin", "op": "mod",
            "left": {"kind": "const", "value": -7},
            "right": {"kind": "const", "value": 3}}), -1)

    def test_index_bounds_fail_closed(self):
        machine = Machine()
        expr = {"kind": "index", "base": {"kind": "array",
            "items": [{"kind": "const", "value": 1}]},
            "index": {"kind": "const", "value": -1}}
        with self.assertRaisesRegex(IRTrap, "out-of-bounds"):
            machine.evaluate(expr)

    def test_rvalue_struct_and_comparison(self):
        machine = Machine()
        item = machine.evaluate({"kind": "struct", "fields": {
            "ok": {"kind": "bin", "op": "lt",
                   "left": {"kind": "const", "value": 2},
                   "right": {"kind": "const", "value": 3}}}})
        self.assertEqual(item, {"ok": True})



if __name__ == "__main__":
    unittest.main()
