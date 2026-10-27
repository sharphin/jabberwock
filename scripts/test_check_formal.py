"""The Lean axiom audit must cover both compiler-specific subtrees."""
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))
import check_formal


class LeanCoverageTests(unittest.TestCase):
    def test_discovers_root_and_nested_compiler_theorems(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder) / "JabberwockFormal"
            (root / "Jwc").mkdir(parents=True)
            (root / "Rust").mkdir(parents=True)
            cases = (
                ("Shared.lean", "JabberwockFormal.Shared", "shared"),
                ("Jwc/Checks.lean", "JabberwockFormal.Jwc.Checks", "jwc"),
                ("Rust/Checks.lean", "JabberwockFormal.Rust.Checks", "rust"),
            )
            for filename, namespace, name in cases:
                (root / filename).write_text(
                    "set_option warningAsError true\nnamespace " + namespace +
                    "\ntheorem " + name + " : True := by trivial\nend " + namespace + "\n",
                    encoding="utf-8")
            names = check_formal.discover_theorems(Path(folder))
            self.assertEqual(set(names), {namespace + "." + name
                                          for _, namespace, name in cases})

    def test_nested_escape_is_rejected(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder) / "JabberwockFormal" / "Rust"
            root.mkdir(parents=True)
            (root / "Unsafe.lean").write_text(
                "set_option warningAsError true\n"
                "namespace JabberwockFormal.Rust.Unsafe\n"
                "theorem forged : True := by sorry\n",
                encoding="utf-8")
            with self.assertRaisesRegex(SystemExit, "unapproved proof escape"):
                check_formal.discover_theorems(Path(folder))


if __name__ == "__main__":
    unittest.main()
