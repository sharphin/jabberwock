"""Negative controls for the real-source LLVM generation-check correspondence audit."""
import sys
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))
import proof_emitter_correspondence as bind


class EmittedGateTests(unittest.TestCase):
    def test_both_real_emitter_templates_match_the_declared_guard_shape(self):
        for compiler in ("jwc", "rust"):
            with self.subTest(compiler=compiler):
                record = bind.validate(compiler)
                self.assertEqual(record["compiler"], compiler)
                self.assertEqual(record["status"], "EMITTER_GATE_STRUCTURE_MATCHED")
                self.assertEqual(len(record["sha256"]), 64)

    def test_missing_retirement_rejection_fails(self):
        for compiler in ("jwc", "rust"):
            path, code = bind.emitter_segment(compiler)
            mutated = code.replace("icmp eq i64 ", "icmp sle i64 ", 1)
            with self.subTest(compiler=compiler), mock.patch.object(
                    bind, "emitter_segment", return_value=(path, mutated)):
                with self.assertRaises(ValueError):
                    bind.validate(compiler)

    def test_changed_branch_polarity_fails(self):
        for compiler in ("jwc", "rust"):
            path, code = bind.emitter_segment(compiler)
            mutated = code.replace("if_true", "if_false", 1)
            with self.subTest(compiler=compiler), mock.patch.object(
                    bind, "emitter_segment", return_value=(path, mutated)):
                with self.assertRaises(ValueError):
                    bind.validate(compiler)

    def test_removed_atomicity_or_trap_fails(self):
        for compiler in ("jwc", "rust"):
            path, code = bind.emitter_segment(compiler)
            for altered in (code.replace("monotonic, align 8", "non_atomic, align 8", 1),
                            code.replace("jw_fail_dangling", "jw_return_dangling", 1)):
                with self.subTest(compiler=compiler), mock.patch.object(
                        bind, "emitter_segment", return_value=(path, altered)):
                    with self.assertRaises(ValueError):
                        bind.validate(compiler)


    def test_duplicate_gate_fragments_are_rejected(self):
        for compiler in ("jwc", "rust"):
            path, code = bind.emitter_segment(compiler)
            for fragment in bind.STEPS:
                mutated = code + "\\n" + fragment
                with self.subTest(compiler=compiler, fragment=fragment), mock.patch.object(
                        bind, "emitter_segment", return_value=(path, mutated)):
                    with self.assertRaises(ValueError):
                        bind.validate(compiler)


    def test_interposed_instruction_or_fallthrough_fails(self):
        for compiler in ("jwc", "rust"):
            path, code = bind.emitter_segment(compiler)
            line = ('    let ok = b.if_true(bad);' if compiler == "jwc"
                    else '    let (_, ok) = b.if_true(&bad);')
            self.assertIn(line, code)
            for injected in (
                line + "\n    b.op(\"ret void\");",
                line + "\n    b.op(\"br label %bypass\");",
            ):
                modified = code.replace(line, injected, 1)
                with self.subTest(compiler=compiler, injected=injected):
                    with mock.patch.object(bind, "emitter_segment",
                                           return_value=(path, modified)):
                        with self.assertRaisesRegex(ValueError, "contiguous"):
                            bind.validate(compiler)



    def test_real_ir_analyzers_keep_element_borrow_checks(self):
        # Until the alias/reallocation invariant is proved, both compiler
        # implementations must refuse RefElem as a standalone proof of safety.
        root = Path(__file__).resolve().parents[1]
        rust = (root / "src/ir/analyze.rs").read_text(encoding="utf-8")
        jwc = (root / "jwc/analyze.jw").read_text(encoding="utf-8")
        self.assertIn(
            "f == Some(Fact::Own) || is_live(facts, f)", rust)
        self.assertNotIn(
            "matches!(f, Some(Fact::RefElem(_))) || is_live(facts, f)", rust)
        self.assertIn("f == OWN || is_live(facts, f)", jwc)
        self.assertNotIn("is_ref_elem(f) || is_live(facts, f)", jwc)


if __name__ == "__main__":
    unittest.main()
