"""Negative controls for generated LLVM guard dataflow validation."""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from llvm_generation_guard import verify_llvm_guard

VALID = """define i64 @demo(ptr %metadata, i64 %saved) {
entry:
  %current = load atomic i64, ptr %metadata monotonic, align 8
  %different = icmp ne i64 %current, %saved
  %retired = icmp eq i64 %current, -1
  %bad = or i1 %different, %retired
  br i1 %bad, label %invalid, label %valid
invalid:
  call void @jw_fail_dangling(ptr null, ptr null, ptr null, i64 0)
  unreachable
valid:
  ret i64 5
}
"""


class GeneratedLLVMGuardTests(unittest.TestCase):
    def test_dataflow_and_reject_branch(self):
        record = verify_llvm_guard(VALID)
        self.assertEqual(record["status"], "LLVM_GUARD_DATAFLOW_MATCHED")
        self.assertEqual(record["saved_generation"], "%saved")
        self.assertEqual(record["reject_block"], "invalid")

    def test_corrupted_guards_are_rejected(self):
        changes = (
            ("retirement missing", "icmp eq i64 %current, -1", "icmp eq i64 %current, 0"),
            ("wrong loaded generation", "icmp eq i64 %current, -1", "icmp eq i64 %saved, -1"),
            ("wrong combine", "or i1 %different, %retired", "and i1 %different, %retired"),
            ("wrong branch", "br i1 %bad, label %invalid, label %valid",
             "br i1 %bad, label %valid, label %invalid"),
            ("returns after trap", "  unreachable", "  ret i64 0"),
            ("missing trap", "call void @jw_fail_dangling", "call void @jw_not_dangling"),
            ("nonatomic load", "load atomic i64", "load i64"),
        )
        for name, before, after in changes:
            with self.subTest(name=name), self.assertRaises(ValueError):
                verify_llvm_guard(VALID.replace(before, after, 1))

    def test_ambiguous_multiple_gates_are_rejected(self):
        extra = VALID.replace("define i64 @demo", "define i64 @other").replace(
            "%current", "%current2").replace("%different", "%different2"
            ).replace("%retired", "%retired2").replace("%bad", "%bad2"
            ).replace("invalid:", "invalid2:").replace("%invalid", "%invalid2"
            ).replace("valid:", "valid2:").replace("%valid", "%valid2"
            ).replace("%saved", "%saved2")
        with self.assertRaises(ValueError):
            verify_llvm_guard(VALID + "\n" + extra)


if __name__ == "__main__":
    unittest.main()
