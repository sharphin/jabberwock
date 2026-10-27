#!/usr/bin/env python3
"""Independent, fail-closed proof-status transition gate.

A PASS string, source hash, theorem name, or successful test is not a
refinement proof. This gate accepts only a registered independent verifier.
There is currently NO registered whole-implementation semantics verifier;
therefore actual repository obligations remain NOT_PROVEN. A verifier must
return a kernel-backed result for the exact obligation and source snapshot.
"""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path
import sys

from whole_compiler_proof import inventory, validate

ROOT = Path(__file__).resolve().parents[1]
REQUIRED = frozenset(("source_refinement", "target_semantics", "external_assumptions",
                      "build_closure", "kernel_proof"))
VERIFIERS = {}  # Deliberately empty until an independently audited verifier exists.


def status_for(obligation: dict, proofs: dict, verifier_registry=None) -> dict:
    """Reject every self-attested claim; injected test checkers cannot alter CLI registry."""
    registry = VERIFIERS if verifier_registry is None else verifier_registry
    record = proofs.get(obligation["id"])
    if record is None:
        return {"status": "REFINEMENT_NOT_PROVEN", "reason": "no proof artifact"}
    if not isinstance(record, dict) or record.get("obligation_id") != obligation["id"]:
        return {"status": "INVALID_EVIDENCE", "reason": "wrong proof identity"}
    if record.get("source_sha256") != obligation["source_sha256"]:
        return {"status": "INVALID_EVIDENCE", "reason": "stale source fingerprint"}
    if set(record.get("required_claims", [])) != REQUIRED:
        return {"status": "INVALID_EVIDENCE", "reason": "incomplete semantic proof claims"}
    checker = record.get("verifier")
    callback = registry.get(checker) if isinstance(checker, str) else None
    if callback is None:
        return {"status": "REFINEMENT_NOT_PROVEN", "reason": "no trusted independent verifier registered"}
    try:
        checked = callback(obligation, record)
    except Exception as exc:
        return {"status": "INVALID_EVIDENCE", "reason": "independent checker failed: " + str(exc)}
    if checked is not True:
        return {"status": "REFINEMENT_NOT_PROVEN", "reason": "independent checker did not discharge proof"}
    return {"status": "REFINEMENT_PROVEN", "reason": "independently verified"}


def evaluate(base: dict, evidence: dict, verifier_registry=None) -> dict:
    validate(base)
    if not isinstance(evidence, dict) or evidence.get("schema_version") != 1:
        raise ValueError("invalid proof evidence schema")
    records = evidence.get("proofs")
    if not isinstance(records, list):
        raise ValueError("proofs must be an array")
    indexed = {}
    for row in records:
        if not isinstance(row, dict) or not isinstance(row.get("obligation_id"), str):
            raise ValueError("invalid proof record")
        key = row["obligation_id"]
        if key in indexed:
            raise ValueError("duplicate proof: " + key)
        indexed[key] = row
    expected = {o["id"] for o in base["components"]}
    if set(indexed) - expected:
        raise ValueError("proofs refer to foreign obligations")
    findings = []
    for obligation in base["components"]:
        verdict = status_for(obligation, indexed, verifier_registry)
        findings.append({"id": obligation["id"], **verdict})
    total = len(findings)
    proved = sum(x["status"] == "REFINEMENT_PROVEN" for x in findings)
    invalid = sum(x["status"] == "INVALID_EVIDENCE" for x in findings)
    result = {"schema_version": 1, "kind": "refinement-transition-audit",
              "status": "INVALID_EVIDENCE" if invalid else ("PROVEN" if total and proved == total else "NOT_PROVEN"),
              "required": total, "proved": proved, "invalid": invalid,
              "findings": findings,
              "limitations": ["Production has no registered whole-implementation proof checker.",
                              "A hash or successful Lean local model alone is not implementation refinement."]}
    return result


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", type=Path, help="optional proof artifacts, never a proof by itself")
    parser.add_argument("--output", type=Path, default=ROOT / "target/refinement-transition.json")
    args = parser.parse_args(argv)
    try:
        proof = {"schema_version": 1, "proofs": []}
        if args.evidence:
            proof = json.loads(args.evidence.read_text(encoding="utf-8"))
        result = evaluate(inventory(), proof)
    except (OSError, ValueError, TypeError, KeyError) as exc:
        result = {"schema_version": 1, "kind": "refinement-transition-audit",
                  "status": "INVALID_EVIDENCE", "reason": str(exc)}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(result["status"], "->", args.output, flush=True)
    return 2 if result["status"] == "INVALID_EVIDENCE" else 0


if __name__ == "__main__":
    sys.exit(main())
