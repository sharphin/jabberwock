#!/usr/bin/env python3
"""Enumerate the *whole actual implementation* and its outstanding proof links.

This generator never certifies a compiler based on theorem names or test PASS.
A missing source, unsupported transform, or absent independently checked
implementation refinement is BLOCKED. The inventory is generated from actual
source files rather than from the list of proposed theorems.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
GOALS = ("semantics", "memory", "types", "ownership", "checks")
# Every supported production source file must belong to exactly one
# implementation phase. New sources are discovered automatically.
ROOTS = {"jwc": (("jwc", ".jw"), ("runtime", ".c"), ("runtime", ".h")),
         "rust": (("src", ".rs"), ("runtime", ".c"), ("runtime", ".h"))}
PHASES = ("lex", "parse", "typecheck", "lower", "analyze",
          "optimize", "emit_llvm", "runtime", "integration")
# These bindings define the *minimum* implementation proof boundaries.
# They do not prove correctness. Each boundary must eventually provide an
# artifact validated against the actual implementation and target semantics.
BOUNDARIES = {
    "lex": ("semantics", "types"),
    "parse": ("semantics", "types"),
    "typecheck": ("types", "ownership", "semantics"),
    "lower": ("semantics", "types", "ownership", "checks"),
    "analyze": ("semantics", "memory", "ownership", "checks"),
    "optimize": ("semantics", "memory", "ownership", "checks"),
    "emit_llvm": GOALS,
    "runtime": GOALS,
    "integration": GOALS,
}


def phase_of(path: str) -> str:
    if path.startswith("runtime/"):
        return "runtime"
    name = Path(path).stem.lower()
    if name in ("lexer",):
        return "lex"
    if name in ("parser", "modules", "ast", "astdump", "diag"):
        return "parse"
    if name in ("typeck", "types", "ty"):
        return "typecheck"
    if name in ("lower",):
        return "lower"
    if name in ("analyze",):
        return "analyze"
    if name in ("llvm", "llvmfn", "native"):
        return "emit_llvm"
    if name in ("ir", "mod"):
        return "optimize"
    if name in ("exec",):
        return "runtime"
    return "integration"


def production_files(compiler: str) -> dict[str, str]:
    if compiler not in ROOTS:
        raise ValueError("unsupported compiler")
    results = {}
    for folder, suffix in ROOTS[compiler]:
        directory = ROOT / folder
        if not directory.is_dir():
            raise ValueError("production source root missing: " + folder)
        for p in directory.rglob("*"):
            if p.is_file() and p.suffix == suffix:
                rel = p.relative_to(ROOT).as_posix()
                if rel in results:
                    raise ValueError("duplicate source: " + rel)
                results[rel] = hashlib.sha256(p.read_bytes()).hexdigest()
    if not results or not any(p.startswith("runtime/") for p in results):
        raise ValueError("missing compiler or runtime sources")
    return dict(sorted(results.items()))


def inventory() -> dict:
    components = []
    for compiler in ROOTS:
        for path, digest in production_files(compiler).items():
            phase = phase_of(path)
            assert phase in PHASES
            for goal in BOUNDARIES[phase]:
                components.append({
                    "id": compiler + ":" + goal + ":" + path,
                    "compiler": compiler, "goal": goal, "phase": phase,
                    "source": path, "source_sha256": digest,
                    "status": "REFINEMENT_NOT_PROVEN",
                    "obligation": "Prove that this production source and its externally "
                                  "observable interactions implement the formally "
                                  "defined " + phase + " refinement for " + goal,
                    "required_evidence": ["kernel-checked refinement theorem",
                                          "formal source-to-implementation correspondence",
                                          "all external assumptions and target semantics discharged"],
                })
    return {"schema_version": 1, "kind": "whole-implementation-proof-inventory",
            "status": "NOT_PROVEN", "boundaries": list(PHASES),
            "required_goals": list(GOALS), "compilers": list(ROOTS),
            "total_obligations": len(components), "proved_obligations": 0,
            "components": components,
            "limitations": [
                "Source inventory is not a correctness proof.",
                "A file hash is not a semantic refinement proof.",
                "LLVM poison, pointer provenance, concurrency, FFI and OS effects remain proof obligations.",
                "This report deliberately provides no user-settable accepted switch."]}


def validate(result: dict) -> None:
    """Reject omission or tampering of any source/phase/goal relation.

    Recompute the canonical inventory directly from the working tree: simply
    keeping one row per file is insufficient, because one proof goal could be
    silently removed while its sibling rows remain.
    """
    if not isinstance(result, dict) or result.get("schema_version") != 1:
        raise ValueError("invalid inventory envelope")
    components = result.get("components")
    if not isinstance(components, list) or not components:
        raise ValueError("empty or malformed proof obligations")
    if any(not isinstance(o, dict) for o in components):
        raise ValueError("malformed proof obligation")
    ids = [o.get("id") for o in components]
    if len(ids) != len(set(ids)):
        raise ValueError("duplicate proof obligation")
    expected = inventory()
    expected_rows = {o["id"]: o for o in expected["components"]}
    actual_rows = {o["id"]: o for o in components}
    if set(actual_rows) != set(expected_rows):
        missing = sorted(set(expected_rows) - set(actual_rows))
        extra = sorted(set(actual_rows) - set(expected_rows))
        raise ValueError("missing/foreign proof obligations: " +
                         repr({"missing": missing[:10], "extra": extra[:10]}))
    for identity, original in expected_rows.items():
        if actual_rows[identity] != original:
            raise ValueError("modified source, phase, goal or evidence requirement: " + identity)
    for key in ("status", "total_obligations", "proved_obligations",
                "required_goals", "compilers", "boundaries"):
        if result.get(key) != expected[key]:
            raise ValueError("invalid inventory metadata: " + key)


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path,
                        default=ROOT / "target" / "whole-proof-inventory.json")
    args = parser.parse_args(argv)
    try:
        result = inventory()
        validate(result)
    except (OSError, ValueError, KeyError) as exc:
        result = {"schema_version": 1, "kind": "whole-implementation-proof-inventory",
                  "status": "INVALID_INVENTORY", "reason": str(exc)}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(result["status"], result.get("total_obligations", 0),
          "implementation proof obligations ->", args.output, flush=True)
    return 0 if result["status"] == "PROVEN" else 1


if __name__ == "__main__":
    sys.exit(main())
