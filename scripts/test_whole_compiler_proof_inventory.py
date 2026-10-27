"""Regression tests for cross-platform source-path normalization in the proof inventory."""
import unittest

from whole_compiler_proof import canonical_source_path, merge_source, phase_of


class SourcePathNormalizationTests(unittest.TestCase):
    def test_windows_and_posix_spellings_have_one_canonical_path(self):
        self.assertEqual(canonical_source_path(r"src\ir\exec.rs"), "src/ir/exec.rs")
        self.assertEqual(canonical_source_path("src/ir/exec.rs"), "src/ir/exec.rs")

    def test_path_aliases_merge_as_one_fingerprint(self):
        sources = {}
        merge_source(sources, r"src\ir\exec.rs", "same-sha")
        merge_source(sources, "src/ir/exec.rs", "same-sha")
        self.assertEqual(sources, {"src/ir/exec.rs": "same-sha"})

    def test_path_alias_with_different_content_is_rejected(self):
        sources = {}
        merge_source(sources, r"src\ir\exec.rs", "sha-a")
        with self.assertRaisesRegex(ValueError, "inconsistent compiler source fingerprint"):
            merge_source(sources, "src/ir/exec.rs", "sha-b")

    def test_phase_detection_accepts_windows_paths(self):
        self.assertEqual(phase_of(r"runtime\jw_rt.c"), "runtime")
        self.assertEqual(phase_of(r"src\ir\analyze.rs"), "analyze")


if __name__ == "__main__":
    unittest.main()
