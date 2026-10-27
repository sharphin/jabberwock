"""Tests for transitive build closure, including negative graph controls."""
import hashlib
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))
from build_closure_graph import audit


class BuildGraphTests(unittest.TestCase):
    def graph(self, root):
        (root / "tool.bin").write_bytes(b"tool")
        (root / "input.jw").write_bytes(b"source")
        h = lambda b: hashlib.sha256(b).hexdigest()
        return {
            "schema_version": 1,
            "nodes": {
                "compiler": {"kind": "tool", "path": "tool.bin", "sha256": h(b"tool")},
                "source": {"kind": "file", "path": "input.jw", "sha256": h(b"source")},
                "build": {"kind": "action", "inputs": ["compiler", "source"],
                          "command": ["tool.bin", "input.jw"], "outputs": ["out.ll"]}},
            "targets": ["build"]}

    def test_walks_and_hashes_complete_declared_dag(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            result = audit(self.graph(root), root, {})
            self.assertEqual(result["visited_nodes"], 3)
            self.assertEqual(result["status"], "DECLARED_CLOSURE_VERIFIED")
            self.assertFalse(result["observed_inputs_complete"])

    def test_source_identity_missing_from_graph_blocks(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            graph = self.graph(root)
            required = {"src/lib.rs": hashlib.sha256(b"library").hexdigest()}
            with mock.patch("build_closure_graph.source_identity",
                            return_value={"files": required}):
                with self.assertRaisesRegex(ValueError, "missing compiler source leaves"):
                    audit(graph, root, {}, compiler="rust")

    def test_missing_input_blocks(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            graph = self.graph(root)
            graph["nodes"]["build"]["inputs"].append("missing")
            with self.assertRaisesRegex(ValueError, "missing graph node"):
                audit(graph, root, {})

    def test_unreachable_node_blocks(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            graph = self.graph(root)
            graph["nodes"]["forgotten"] = {"kind": "file", "path": "input.jw",
                                             "sha256": hashlib.sha256(b"source").hexdigest()}
            with self.assertRaisesRegex(ValueError, "unreachable"):
                audit(graph, root, {})

    def test_mutated_file_blocks(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            graph = self.graph(root)
            (root / "input.jw").write_bytes(b"changed")
            with self.assertRaisesRegex(ValueError, "hash mismatch"):
                audit(graph, root, {})

    def test_cycle_blocks(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            graph = self.graph(root)
            graph["nodes"]["build"]["inputs"].append("build")
            with self.assertRaisesRegex(ValueError, "cyclic dependency"):
                audit(graph, root, {})


if __name__ == "__main__":
    unittest.main()
