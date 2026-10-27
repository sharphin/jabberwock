"""Regression: caller facts must reflect all arguments' moves before parameter liveness is recorded."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


class CallArgumentLivenessTest(unittest.TestCase):
    def test_rust_call_liveness_after_argument_moves(self):
        source = (ROOT / "src/ir/analyze.rs").read_text(encoding="utf-8")
        start = source.index("fn operands(&mut self, os:")
        end = source.index("\n    fn record(", start)
        body = source[start:end]
        self.assertLess(body.index("self.operand(o, facts);"),
                        body.index("fs.iter().map(|f| is_live(facts, *f))"))

    def test_jwc_call_liveness_after_argument_moves(self):
        source = (ROOT / "jwc/analyze.jw").read_text(encoding="utf-8")
        start = source.index("fn mut bool[] operands(")
        end = source.index("\n    fn mut record(", start)
        body = source[start:end]
        self.assertLess(body.index("operand(ref mut os[j], ref mut facts);"),
                        body.index("safe.push(is_live(facts, f))"))


if __name__ == "__main__":
    unittest.main()
