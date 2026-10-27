#!/usr/bin/env python3
"""Build an actionable work-package view of whole-implementation proof gaps.

This is planning/reporting only. It preserves every source/goal obligation and
cannot promote any obligation to proven. Related local-model evidence is
reported separately and never counts as implementation refinement.
"""
from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path

from whole_compiler_proof import GOALS, PHASES, inventory as make_inventory, validate

ROOT = Path(__file__).resolve().parents[1]
# Work from the target/runtime side first, then move outward through the
# implementation pipeline. This is a prioritization suggestion, not a proof
# dependency claim.
PRIORITY = ("runtime", "emit_llvm", "optimize", "lower", "analyze",
            "typecheck", "parse", "lex", "integration")

NEXT_ACTION = {
    "runtime": "Define operational behavior and invariants for each runtime subsystem; prove source-to-model correspondence, including atomics and allocator assumptions.",
    "emit_llvm": "Define the observable LLVM subset emitted by the backend and prove each emitter rule preserves typed-IR behavior.",
    "optimize": "Prove each IR rewrite preserves the chosen observation relation; identify poison, overflow, and aliasing assumptions.",
    "lower": "Prove typed source constructs lower to IR with the same values, errors, ownership, and checks.",
    "analyze": "Prove static analyses are sound for every safety check that relies on their results.",
    "typecheck": "Define typing and ownership judgments and prove the implementation accepts only programs satisfying them.",
    "parse": "Prove lexer/parser output agrees with the source grammar, including malformed-input behavior.",
    "lex": "Specify tokenization and source-span behavior; prove the concrete lexer implements it.",
    "integration": "Close cross-module, FFI, build, and entry-point assumptions after component proofs exist.",
}


def load_object(path: Path, kind: str) -> dict:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict) or data.get("kind") != kind:
        raise ValueError("unexpected report kind: " + str(path))
    return data


def build_plan(inv: dict, refinement: dict, runtime_model: dict | None = None) -> dict:
    validate(inv)
    rows = inv["components"]
    grouped = defaultdict(list)
    seen_source_hashes: dict[str, str] = {}
    for row in rows:
        key = (row["compiler"], row["phase"])
        grouped[key].append(row)
        old = seen_source_hashes.setdefault(row["source"], row["source_sha256"])
        if old != row["source_sha256"]:
            raise ValueError("inconsistent source fingerprint across obligations: " + row["source"])

    packages = []
    for compiler in ("jwc", "rust"):
        for phase in PRIORITY:
            members = grouped.get((compiler, phase), [])
            if not members:
                continue
            source_hashes = {}
            goal_counts = {goal: 0 for goal in GOALS}
            for row in members:
                source_hashes[row["source"]] = row["source_sha256"]
                if row["goal"] not in goal_counts:
                    raise ValueError("unknown proof goal: " + str(row["goal"]))
                goal_counts[row["goal"]] += 1
            packages.append({
                "id": compiler + ":" + phase,
                "compiler": compiler,
                "phase": phase,
                "priority_rank": PRIORITY.index(phase) + 1,
                "status": "NOT_PROVEN",
                "source_count": len(source_hashes),
                "source_sha256": dict(sorted(source_hashes.items())),
                "underlying_obligation_count": len(members),
                "goal_counts": goal_counts,
                "obligation_ids": sorted(row["id"] for row in members),
                "next_action": NEXT_ACTION[phase],
                "proof_requirements": [
                    "explicit formal semantics and observation relation",
                    "kernel-checked theorem for the concrete implementation behavior",
                    "trusted source-to-theorem correspondence",
                    "closed build/toolchain and external assumptions",
                ],
            })

    expected = inv["total_obligations"]
    covered = sum(p["underlying_obligation_count"] for p in packages)
    if covered != expected:
        raise ValueError("work packages do not cover the full inventory")
    proved = refinement.get("proved", 0)
    if refinement.get("required") != expected:
        raise ValueError("refinement report and inventory obligation counts differ")
    if refinement.get("status") not in ("NOT_PROVEN", "PROVEN"):
        raise ValueError("invalid refinement report status")

    result = {
        "schema_version": 1,
        "kind": "refinement-work-frontier",
        "status": "PLAN_READY_NOT_PROVEN",
        "underlying_obligations": expected,
        "underlying_proven": proved,
        "work_package_count": len(packages),
        "source_count": len(seen_source_hashes),
        "priority_order": list(PRIORITY),
        "packages": packages,
        "related_model_evidence": {
            "status": runtime_model.get("status", "UNAVAILABLE") if runtime_model else "UNAVAILABLE",
            "checked_theorems": runtime_model.get("checked_count", 0) if runtime_model else 0,
            "discharges_implementation_obligations": 0,
            "note": "Runtime-generation Nat/BitVec model checks are useful local evidence, but structural C matching does not establish C semantic refinement.",
        },
        "next_recommended_package": next(
            (p["id"] for p in packages if p["status"] != "PROVEN"), None),
        "limitations": [
            "Packages are a planning view over all original source/goal obligations; no obligations are removed or merged for acceptance.",
            "The suggested order is a work-prioritization heuristic, not a formal dependency proof.",
            "A successful plan report is not a proof and cannot change FINAL: NOT_PROVEN.",
        ],
    }
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inventory", type=Path, required=True)
    parser.add_argument("--refinement", type=Path, required=True)
    parser.add_argument("--runtime-model", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        inv = load_object(args.inventory, "whole-implementation-proof-inventory")
        refinement = load_object(args.refinement, "refinement-transition-audit")
        runtime_model = load_object(args.runtime_model, "runtime-generation-arithmetic-proof") if args.runtime_model else None
        result = build_plan(inv, refinement, runtime_model)
    except (OSError, ValueError, TypeError, KeyError) as exc:
        result = {"schema_version": 1, "kind": "refinement-work-frontier",
                  "status": "INVALID_REPORT", "reason": str(exc)}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(result["status"], result.get("work_package_count", 0), "work packages /",
          result.get("underlying_proven", 0), "/", result.get("underlying_obligations", 0),
          "proved ->", args.output, flush=True)
    return 0 if result["status"] == "PLAN_READY_NOT_PROVEN" else 2


if __name__ == "__main__":
    raise SystemExit(main())
