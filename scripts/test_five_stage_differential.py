"""End-to-end tests for the five-stage two-compiler differential runner."""
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))
import five_stage_differential as d


class DifferentialTests(unittest.TestCase):
    def fixture(self, root):
        (root / "case.jw").write_text("42", encoding="utf-8")
        driver = [sys.executable, "-c",
                  "from pathlib import Path; import sys; "
                  "print(Path(sys.argv[1]).read_text())",
                  "{source}"]
        return {"schema_version": 1,
                "stages": {stage: {compiler: driver for compiler in d.COMPILERS}
                           for stage in d.STAGES},
                "cases": [{"id": "constant", "source": "case.jw",
                           "expect": {stage: {"exit": 0, "stdout": "42\n"}
                                      for stage in d.STAGES}}]}

    def test_all_ten_stage_compiler_pairs_execute(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            outcome = d.evaluate(self.fixture(root), root)
            self.assertEqual(outcome["status"], "PASS")
            self.assertEqual((outcome["passed"], outcome["total"]), (10, 10))

    def test_any_wrong_semantics_fails(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            cfg = self.fixture(root)
            cfg["cases"][0]["expect"]["lower"]["stdout"] = "bad\n"
            outcome = d.evaluate(cfg, root)
            self.assertEqual(outcome["status"], "FAIL")
            self.assertEqual(outcome["passed"], 8)

    def test_missing_stage_rejected(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            cfg = self.fixture(root)
            del cfg["stages"]["parse"]
            with self.assertRaisesRegex(ValueError, "all five"):
                d.evaluate(cfg, root)

    def test_path_traversal_rejected(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            cfg = self.fixture(root)
            cfg["cases"][0]["source"] = "../case.jw"
            with self.assertRaisesRegex(ValueError, "escaped"):
                d.evaluate(cfg, root)


if __name__ == "__main__":
    unittest.main()
