#!/usr/bin/env python3
"""Kernel-check a narrow runtime-generation arithmetic boundary.

This report is deliberately narrower than source-to-C or whole-compiler
refinement. It verifies exact theorem contracts with Lean 4.24.0, audits their
axiom dependencies, and requires the existing fail-closed C/Lean structural
binding check to pass. Structural matching is not a verified C parser.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FORMAL = ROOT / "formal"
C_SOURCE = ROOT / "runtime" / "jw_rt.c"
LEAN_SOURCE = FORMAL / "JabberwockFormal" / "RuntimeGenerationBitVec.lean"
ALLOWED_AXIOMS = {"propext", "Quot.sound", "Classical.choice"}

CONTRACTS = r"""
import JabberwockFormal.RuntimeGenerationBitVec
open JabberwockFormal.RuntimeGenerationBitVec

example (gen : Word) :
    (gen &&& counterMask).toNat = gen.toNat % radix :=
  masked_counter_matches_mod gen

example (gen step limit : Word) :
    guard gen step limit = true ↔
      gen ≠ retiredWord ∧ step.toNat ≤ limit.toNat ∧
        gen.toNat % radix ≤ limit.toNat - step.toNat :=
  guard_matches_nat gen step limit

example (gen step limit : Word)
    (validGen : gen.toNat < 2 ^ 63)
    (validLimit : limit.toNat < radix)
    (accepted : guard gen step limit = true) :
    (gen + step).toNat = gen.toNat + step.toNat :=
  accepted_increment_no_wrap gen step limit validGen validLimit accepted

example (gen step limit : Word)
    (validGen : gen.toNat < 2 ^ 63)
    (validLimit : limit.toNat < radix)
    (accepted : guard gen step limit = true)
    (positive : 0 < step.toNat) :
    gen + step ≠ gen :=
  accepted_increment_changes_word gen step limit validGen validLimit accepted positive

#print axioms JabberwockFormal.RuntimeGenerationBitVec.masked_counter_matches_mod
#print axioms JabberwockFormal.RuntimeGenerationBitVec.guard_matches_nat
#print axioms JabberwockFormal.RuntimeGenerationBitVec.accepted_increment_no_wrap
#print axioms JabberwockFormal.RuntimeGenerationBitVec.accepted_increment_changes_word
"""

THEOREMS = (
    "JabberwockFormal.RuntimeGenerationBitVec.masked_counter_matches_mod",
    "JabberwockFormal.RuntimeGenerationBitVec.guard_matches_nat",
    "JabberwockFormal.RuntimeGenerationBitVec.accepted_increment_no_wrap",
    "JabberwockFormal.RuntimeGenerationBitVec.accepted_increment_changes_word",
)


def digest(path: Path) -> str:
    if path.is_symlink() or not path.is_file():
        raise ValueError("missing or symlinked source: " + str(path.relative_to(ROOT)))
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(argv: list[str], cwd: Path, timeout: int) -> subprocess.CompletedProcess[str]:
    return subprocess.run(argv, cwd=cwd, capture_output=True, text=True,
                          encoding="utf-8", errors="replace", timeout=timeout, check=False)


def audit(timeout: int) -> dict:
    lake = shutil.which("lake")
    if not lake:
        raise ValueError("lake is required; refusing to skip kernel checking")
    before = {"runtime/jw_rt.c": digest(C_SOURCE),
              "formal/JabberwockFormal/RuntimeGenerationBitVec.lean": digest(LEAN_SOURCE)}
    version = run([lake, "env", "lean", "--version"], FORMAL, timeout)
    if version.returncode != 0 or "version 4.24.0," not in (version.stdout + version.stderr):
        raise ValueError("expected Lean 4.24.0: " + (version.stderr or version.stdout).strip())

    with tempfile.TemporaryDirectory(prefix="jw-runtime-generation-proof-") as temp:
        binding_path = Path(temp) / "binding.json"
        binding = run([sys.executable, str(ROOT / "scripts" / "runtime_generation_binding.py"),
                       "--output", str(binding_path)], ROOT, timeout)
        if binding.returncode != 0:
            raise ValueError("runtime C/Lean structural binding failed: " +
                             (binding.stderr or binding.stdout)[-1500:])
        binding_report = json.loads(binding_path.read_text(encoding="utf-8"))
        if binding_report.get("status") != "STRUCTURE_MATCHED":
            raise ValueError("runtime C/Lean structural binding is not matched")

        build = run([lake, "build"], FORMAL, timeout)
        if build.returncode != 0:
            raise ValueError("Lean build failed: " + (build.stderr or build.stdout)[-2500:])

        contract_file = Path(temp) / "RuntimeGenerationContracts.lean"
        contract_file.write_text(CONTRACTS, encoding="utf-8")
        checked = run([lake, "env", "lean", str(contract_file)], FORMAL, timeout)
        if checked.returncode != 0:
            raise ValueError("Lean rejected a runtime-generation contract: " +
                             (checked.stderr or checked.stdout)[-2500:])
        output = checked.stdout + checked.stderr
        for theorem in THEOREMS:
            pattern = re.escape("'" + theorem + "'") + (
                r" (?:does not depend on any axioms|depends on axioms:\s*\[([^\]]*)\])")
            found = re.search(pattern, output)
            if not found:
                raise ValueError("missing Lean kernel axiom result for " + theorem)
            axioms = {item.strip() for item in (found.group(1) or "").split(",") if item.strip()}
            unexpected = axioms - ALLOWED_AXIOMS
            if unexpected:
                raise ValueError("unapproved axioms for " + theorem + ": " + repr(sorted(unexpected)))

    after = {"runtime/jw_rt.c": digest(C_SOURCE),
             "formal/JabberwockFormal/RuntimeGenerationBitVec.lean": digest(LEAN_SOURCE)}
    if before != after:
        raise ValueError("C or Lean source changed during runtime-generation audit")
    return {
        "schema_version": 1,
        "kind": "runtime-generation-arithmetic-proof",
        "status": "KERNEL_CHECKED_RUNTIME_ARITHMETIC_ONLY",
        "lean_version": version.stdout.strip(),
        "checked_theorems": list(THEOREMS),
        "checked_count": len(THEOREMS),
        "source_sha256": before,
        "structural_binding": {
            "status": binding_report["status"],
            "matched": binding_report.get("matched"),
            "checked": binding_report.get("checked"),
        },
        "limitations": [
            "The Lean kernel checks the stated BitVec arithmetic contracts only.",
            "C-to-Lean binding is structural pattern matching, not a verified C parser or C compiler proof.",
            "This does not prove pointer lifetime, allocator correctness, atomic memory ordering, concurrency safety, or whole-compiler refinement.",
            "This narrow result does not discharge any of the 390 whole-implementation refinement obligations."
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--timeout", type=int, default=600)
    args = parser.parse_args()
    try:
        result = audit(args.timeout)
    except (OSError, ValueError, KeyError, json.JSONDecodeError, subprocess.TimeoutExpired) as exc:
        result = {"schema_version": 1, "kind": "runtime-generation-arithmetic-proof",
                  "status": "NOT_CHECKED", "reason": str(exc),
                  "limitations": ["No proof result is inferred from a failed or skipped check."]}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(result["status"], result.get("checked_count", 0), "runtime arithmetic contracts ->", args.output,
          flush=True)
    return 0 if result["status"] == "KERNEL_CHECKED_RUNTIME_ARITHMETIC_ONLY" else 2


if __name__ == "__main__":
    raise SystemExit(main())
