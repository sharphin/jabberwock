#!/usr/bin/env python3
"""Check compiler-stage-to-Lean theorem bindings without confusing them with a refinement proof.

The checked-in registry specifies reviewed stage/theorem relationships.  For a
bound theorem this checker can invoke Lean's actual kernel audit, rather than
accepting theorem-name strings as proof.  It never certifies semantic
correspondence between a compiler and an abstract model merely from hashes.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path

from semantic_refinement_boundaries import obligation_rows
from lean_refinement_contracts import load_contracts
from lean_evidence_engine import check as check_kernel
from prepare_lean_evidence import prepare as prepare_manifest

ROOT = Path(__file__).resolve().parents[1]


def inspect(root: Path, *, timeout: int = 600) -> dict:
    boundaries = obligation_rows(root)
    reviewed = load_contracts(root, None)
    contract_names = {x["theorem"] for x in reviewed}
    registry = root / "formal" / "stage-refinement-bindings.json"
    if not registry.exists():
        return {"schema_version": 1, "kind": "stage-refinement-bindings",
                "status": "NOT_PROVEN", "required": len(boundaries), "covered": 0,
                "kernel_checked": 0, "bindings": [],
                "reason": "no reviewed stage-to-theorem bindings provided"}
    if registry.is_symlink() or not registry.is_file():
        raise ValueError("stage binding registry must be an ordinary file")
    document = json.loads(registry.read_text(encoding="utf-8"))
    if (not isinstance(document, dict) or document.get("schema_version") != 1
            or not isinstance(document.get("bindings"), list)):
        raise ValueError("invalid stage binding registry")
    expected = {row["id"]: row for row in boundaries}
    visited = set()
    findings = []
    for entry in document["bindings"]:
        if not isinstance(entry, dict) or set(entry) != {"stage_id", "theorem"}:
            raise ValueError("stage binding requires stage_id and theorem")
        stage_id, theorem = entry["stage_id"], entry["theorem"]
        sources = expected[stage_id]["sources"] if isinstance(stage_id, str) and stage_id in expected else {}
        manifest = prepare_manifest(root, stage_id, theorem)
        if not isinstance(stage_id, str) or stage_id in visited or stage_id not in expected:
            raise ValueError("unknown or duplicate stage")
        visited.add(stage_id)
        if not isinstance(theorem, str) or theorem not in contract_names:
            raise ValueError("theorem has no reviewed expected-type contract")
        if sources != expected[stage_id]["sources"]:
            raise ValueError("stage source fingerprints changed: " + stage_id)
        if not isinstance(manifest, dict):
            raise ValueError("missing proof manifest")
        # The compiler sources must be authenticated by the kernel manifest too,
        # not just by the human-maintained registry.
        files = manifest.get("source_files")
        if not isinstance(files, list):
            raise ValueError("proof manifest source_files missing")
        for source, digest in sources.items():
            if {"path": source, "sha256": digest} not in files:
                raise ValueError("proof manifest does not bind " + source)
        if theorem not in manifest.get("theorems", []):
            raise ValueError("theorem missing from kernel manifest")
        # check_kernel runs lake, performs the theorem's axiom audit and checks
        # repository-owned expected proposition types when registered.
        verified = check_kernel(root, manifest, timeout=timeout)
        if verified.get("status") != "KERNEL_CHECKED":
            raise ValueError("kernel did not verify claimed theorem")
        if theorem not in verified.get("contract_types_checked", []):
            raise ValueError("expected proposition was not checked by Lean")
        findings.append({"stage_id": stage_id, "theorem": theorem,
                         "source_hashes_checked": True, "kernel_type_checked": True,
                         "status": "KERNEL_CHECKED_REFINEMENT_NOT_PROVEN"})
    return {"schema_version": 1, "kind": "stage-refinement-bindings",
            "status": "NOT_PROVEN", "required": len(boundaries),
            "covered": len(findings), "kernel_checked": len(findings),
            "bindings": findings,
            "limitations": ["A source fingerprint is not a compiler refinement proof.",
                            "The theorem's expected type must itself describe the real implementation.",
                            "No whole-compiler correctness is certified by this stage."]}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--stage-timeout", type=int, default=600)
    args = parser.parse_args()
    if args.stage_timeout < 1:
        parser.error("timeout must be positive")
    try:
        result = inspect(ROOT, timeout=args.stage_timeout)
    except (ValueError, OSError, KeyError, TypeError, subprocess.TimeoutExpired) as exc:
        result = {"schema_version": 1, "kind": "stage-refinement-bindings",
                  "status": "INVALID_EVIDENCE", "reason": str(exc)}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(result["status"], "stage bindings ->", args.output, flush=True)
    return 2 if result["status"] == "INVALID_EVIDENCE" else 0


if __name__ == "__main__":
    raise SystemExit(main())
