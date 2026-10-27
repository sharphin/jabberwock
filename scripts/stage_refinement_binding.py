#!/usr/bin/env python3
"""Bind real compiler stage sources to reviewed Lean contracts without asserting proof."""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path
from semantic_refinement_boundaries import obligation_rows
from lean_refinement_contracts import load_contracts

ROOT = Path(__file__).resolve().parents[1]

def inspect(root: Path) -> dict:
    boundaries = obligation_rows(root)
    contracts = load_contracts(root, None)
    # The contract loader validates the checked-in declaration format, while
    # this tool independently computes source fingerprints for every stage.
    path = root / "formal" / "stage-refinement-bindings.json"
    if not path.is_file():
        return {"schema_version": 1, "kind": "stage-refinement-bindings",
                "status": "NOT_PROVEN", "bindings": [],
                "required": len(boundaries), "covered": 0,
                "reason": "no reviewed stage-to-theorem bindings provided"}
    if path.is_symlink():
        raise ValueError("stage bindings must not be a symlink")
    doc = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(doc, dict) or doc.get("schema_version") != 1 or not isinstance(doc.get("bindings"), list):
        raise ValueError("invalid binding registry")
    expected = {row["id"]: row for row in boundaries}
    theorem_names = {x["theorem"] for x in contracts}
    seen = set()
    findings = []
    for entry in doc["bindings"]:
        if not isinstance(entry, dict) or set(entry) != {"stage_id", "theorem", "sources"}:
            raise ValueError("malformed stage binding")
        stage_id, theorem, sources = entry["stage_id"], entry["theorem"], entry["sources"]
        if not isinstance(stage_id, str) or stage_id in seen or stage_id not in expected:
            raise ValueError("duplicate or foreign stage identifier")
        seen.add(stage_id)
        if not isinstance(theorem, str) or theorem not in theorem_names:
            raise ValueError("stage theorem is absent from the reviewed contract registry")
        if sources != expected[stage_id]["sources"]:
            raise ValueError("compiler source fingerprints do not match stage binding")
        findings.append({"stage_id": stage_id, "theorem": theorem,
                         "source_hashes_checked": True, "status": "BOUND_NOT_PROVEN"})
    return {"schema_version": 1, "kind": "stage-refinement-bindings",
            "status": "NOT_PROVEN", "required": len(boundaries),
            "covered": len(findings), "bindings": findings,
            "limitations": ["A hash binding is not semantic refinement.",
                            "Lean theorem checks and semantic implementation proof remain separate."]}

def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        result = inspect(ROOT)
    except (ValueError, OSError, KeyError, TypeError) as exc:
        result = {"schema_version": 1, "kind": "stage-refinement-bindings",
                  "status": "INVALID_EVIDENCE", "reason": str(exc)}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    return 2 if result["status"] == "INVALID_EVIDENCE" else 0

if __name__ == "__main__":
    raise SystemExit(main())
