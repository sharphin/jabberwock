"""Regression tests for direct invocation of the two production compilers."""
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))
import real_compiler_five_stages as tool


class RealCompilerStagesTest(unittest.TestCase):
    def test_exact_stage_switches(self):
        self.assertEqual(list(tool.STAGES),
                         ["source_semantics", "parse", "typecheck", "lower", "ir_semantics"])
        self.assertEqual(tool.STAGES["lower"], ("--ir-raw",))
        self.assertEqual(tool.STAGES["ir_semantics"], ("--ir",))

    def test_parser_covers_real_lexer_and_ast(self):
        self.assertEqual(tool.PROBES["parse"], ("--tokens", "--ast"))

    def test_source_bindings_are_production_files(self):
        root = Path(__file__).resolve().parents[1]
        bindings = tool.check_cli_sources(root)
        self.assertEqual(set(bindings), {"rust", "jwc"})

    def test_mismatch_is_not_accepted(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            for p in ("src/main.rs", "jwc/main.jw"):
                file = root / p
                file.parent.mkdir(parents=True, exist_ok=True)
                file.write_text("--tokens --ast --typed --ir-raw --ir", encoding="utf-8")
            source = root / "sample.jw"
            source.write_text("{}", encoding="utf-8")
            rust = root / "rust.bin"
            jwc = root / "jwc.bin"
            rust.write_bytes(b"x")
            jwc.write_bytes(b"x")
            def fake_run(binary, stage, source, *, root, timeout, probe=None):
                return {"status": "EXECUTED", "exit": 0,
                        "stdout": "rust" if binary == rust else "jwc", "stderr": ""}
            with patch.object(tool, "run_boundary", side_effect=fake_run):
                report = tool.verify(root, {"rust": rust, "jwc": jwc}, [source])
            self.assertEqual(report["status"], "FAIL")
            self.assertEqual(report["matched"], 0)


    def test_identical_compiler_failures_do_not_count_as_success(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            for filename in ("src/main.rs", "jwc/main.jw"):
                path = root / filename
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("--tokens --ast --typed --ir-raw --ir", encoding="utf-8")
            sample = root / "sample.jw"
            sample.write_text("invalid", encoding="utf-8")
            a, b = root / "a", root / "b"
            a.touch()
            b.touch()
            with patch.object(tool, "run_boundary", return_value={
                "status": "EXECUTED", "exit": 1, "stdout": "", "stderr": "error"}):
                report = tool.verify(root, {"rust": a, "jwc": b}, [sample])
            self.assertEqual(report["status"], "FAIL")
            self.assertEqual(report["matched"], 0)

    def test_expected_negative_cases_can_match(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            for filename in ("src/main.rs", "jwc/main.jw"):
                path = root / filename
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("--tokens --ast --typed --ir-raw --ir", encoding="utf-8")
            sample = root / "sample.jw"
            sample.write_text("invalid", encoding="utf-8")
            a, b = root / "a", root / "b"
            a.touch()
            b.touch()
            expectations = {str(sample.resolve()): {
                stage: {"exit": 1, "stdout": ""} for stage in tool.STAGES}}
            with patch.object(tool, "run_boundary", return_value={
                "status": "EXECUTED", "exit": 1, "stdout": "", "stderr": "error"}):
                report = tool.verify(root, {"rust": a, "jwc": b}, [sample],
                                     expectations=expectations)
            self.assertEqual(report["status"], "MATCH")
            self.assertEqual(report["matched"], 6)


if __name__ == "__main__":
    unittest.main()
