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
                file.write_text("--ast --typed --ir-raw --ir", encoding="utf-8")
            source = root / "sample.jw"
            source.write_text("{}", encoding="utf-8")
            rust = root / "rust.bin"
            jwc = root / "jwc.bin"
            rust.write_bytes(b"x")
            jwc.write_bytes(b"x")
            def fake_run(binary, stage, source, *, root, timeout):
                return {"status": "EXECUTED", "exit": 0,
                        "stdout": "rust" if binary == rust else "jwc", "stderr": ""}
            with patch.object(tool, "run_boundary", side_effect=fake_run):
                report = tool.verify(root, {"rust": rust, "jwc": jwc}, [source])
            self.assertEqual(report["status"], "FAIL")
            self.assertEqual(report["matched"], 0)


if __name__ == "__main__":
    unittest.main()
