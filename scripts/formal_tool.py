#!/usr/bin/env python3
"""One-command formal verification tool runner (not the old compiler test suite).

Runs implementation inventory, Lean kernel/axiom audit, the source-emitter
correspondence checker, and the evidence/refinement gate. Missing proof
evidence never becomes PASS. No test fixtures or CI are created.
"""
from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

def atomic_json(path: Path, value: dict) -> None:
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    os.replace(temporary, path)

def run_stage(name: str, argv: list[str], output: Path, timeout: int) -> dict:
    start = time.monotonic()
    stdout = output / (name + ".stdout.log")
    stderr = output / (name + ".stderr.log")
    # Do not leave old successful-looking logs around when a stage cannot run.
    stdout.unlink(missing_ok=True)
    stderr.unlink(missing_ok=True)
    try:
        with stdout.open("wb") as out, stderr.open("wb") as err:
            completed = subprocess.run(argv, cwd=ROOT, stdin=subprocess.DEVNULL,
                                       stdout=out, stderr=err, timeout=timeout,
                                       check=False)
        return {"name": name, "status": "COMPLETED" if completed.returncode == 0 else "NONZERO_EXIT",
                "returncode": completed.returncode,
                "seconds": round(time.monotonic() - start, 3),
                "stdout": str(stdout), "stderr": str(stderr)}
    except subprocess.TimeoutExpired:
        return {"name": name, "status": "TIMEOUT", "seconds": round(time.monotonic() - start, 3),
                "stdout": str(stdout), "stderr": str(stderr)}
    except OSError as exc:
        return {"name": name, "status": "EXECUTION_ERROR", "reason": str(exc),
                "seconds": round(time.monotonic() - start, 3),
                "stdout": str(stdout), "stderr": str(stderr)}

def read_report(path: Path, expected_kind: str | None = None) -> dict:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
        if not isinstance(data, dict):
            raise ValueError("report is not a JSON object")
        if expected_kind is not None and data.get("kind") != expected_kind:
            raise ValueError("unexpected report kind")
        return data
    except (OSError, ValueError) as exc:
        return {"status": "INVALID_REPORT", "reason": str(exc)}

def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "target/formal-tool")
    parser.add_argument("--evidence", type=Path, help="existing proof evidence for refinement gate")
    parser.add_argument("--lean-manifest", type=Path, help="existing kernel evidence manifest")
    parser.add_argument("--stage-timeout", type=int, default=600)
    args = parser.parse_args(argv)
    if args.stage_timeout < 1:
        parser.error("--stage-timeout must be positive")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    py = sys.executable
    scripts = ROOT / "scripts"
    plans = [
        ("inventory", [py, str(scripts / "whole_compiler_proof.py"),
                       "--output", str(output / "inventory.json")]),
        ("lean-axioms", [py, str(scripts / "check_formal.py")]),
        ("semantic-boundaries", [py, str(scripts / "semantic_refinement_boundaries.py"),
                                 "--output", str(output / "semantic-boundaries.json")]),
        ("semantic-composition", [py, str(scripts / "semantic_composition.py"),
                                  "--output", str(output / "semantic-composition.json")]),
        ("stage-refinement-binding", [py, str(scripts / "stage_refinement_binding.py"),
                                       "--output", str(output / "stage-refinement-binding.json"),
                                       "--stage-timeout", str(args.stage_timeout)]),
        ("worklist-structure", [py, str(scripts / "audit_worklist_structure.py"),
                                "--output", str(output / "worklist-structure.json")]),
        ("emitter-binding", [py, str(scripts / "proof_emitter_correspondence.py"),
                             "--output", str(output / "emitter-binding.json")]),
        ("emitter-semantics", [py, str(scripts / "verify_emitter_gate_semantics.py"),
                               "--output", str(output / "emitter-semantics.json"),
                               "--timeout", str(args.stage_timeout)]),
    ]
    if args.lean_manifest:
        plans.append(("lean-evidence", [py, str(scripts / "lean_evidence_engine.py"),
                                         "--manifest", str(args.lean_manifest.resolve()),
                                         "--output", str(output / "lean-evidence.json")]))
    refinement = [py, str(scripts / "refinement_transition.py"),
                  "--output", str(output / "refinement.json")]
    if args.evidence:
        refinement += ["--evidence", str(args.evidence.resolve())]
    plans.append(("refinement", refinement))
    plans.append(("coverage", [py, str(scripts / "formal_proof_coverage.py"),
                                "--boundaries", str(output / "semantic-boundaries.json"),
                                "--bindings", str(output / "stage-refinement-binding.json"),
                                "--refinement", str(output / "refinement.json"),
                                "--output", str(output / "proof-coverage.json")]))
    known_reports = {"inventory": "inventory.json", "semantic-boundaries": "semantic-boundaries.json", "semantic-composition": "semantic-composition.json", "stage-refinement-binding": "stage-refinement-binding.json", "emitter-binding": "emitter-binding.json",
                     "lean-evidence": "lean-evidence.json", "refinement": "refinement.json",
                     "coverage": "proof-coverage.json", "emitter-semantics": "emitter-semantics.json",
                     "worklist-structure": "worklist-structure.json"}
    # Remove stale reports even for optional stages.
    for filename in known_reports.values():
        (output / filename).unlink(missing_ok=True)
    stages = []
    for name, command in plans:
        stage = run_stage(name, command, output, args.stage_timeout)
        filename = known_reports.get(name)
        if filename:
            report = read_report(output / filename)
            stage["report"] = report
            # A nonzero proof-obligation exit is not necessarily a tool crash.
        stages.append(stage)
        if name == "refinement":
            report = stage.get("report", {})
            if stage.get("returncode") == 1 and report.get("status") == "NOT_PROVEN":
                findings = report.get("findings", [])
                counts = {}
                for finding in findings:
                    status = finding.get("status", "UNKNOWN")
                    counts[status] = counts.get(status, 0) + 1
                reasons = []
                for finding in findings:
                    reason = finding.get("reason")
                    if isinstance(reason, str) and reason not in reasons:
                        reasons.append(reason)
                    if len(reasons) == 3:
                        break
                print("refinement: NOT_PROVEN (proof gate exited 1 as designed; "
                      + str(report.get("proved", 0)) + "/" + str(report.get("required", len(findings)))
                      + " obligations proven; " + json.dumps(counts, ensure_ascii=False)
                      + "; reasons: " + json.dumps(reasons, ensure_ascii=False) + ")", flush=True)
            else:
                print(name + ": " + stage["status"], flush=True)
        else:
            print(name + ": " + stage["status"], flush=True)
    by_name = {stage["name"]: stage for stage in stages}
    broken = any(
        s["status"] in ("TIMEOUT", "EXECUTION_ERROR") or
        (s["status"] == "NONZERO_EXIT" and s["name"] != "refinement") or
        (s["name"] == "refinement" and s.get("returncode") not in (0, 1)) or
        s.get("report", {}).get("status") in
        ("INVALID_REPORT", "INVALID_EVIDENCE", "INVALID_INVENTORY", "FAIL")
        for s in stages
    )
    refinement_status = by_name["refinement"].get("report", {}).get("status")
    boundaries_status = by_name["semantic-boundaries"].get("report", {}).get("status")
    if boundaries_status != "NOT_PROVEN":
        broken = True
    composition_status = by_name["semantic-composition"].get("report", {}).get("status")
    coverage_status = by_name["coverage"].get("report", {}).get("status")
    if composition_status != "NOT_PROVEN" or coverage_status != "NOT_PROVEN":
        broken = True
    if by_name["emitter-semantics"].get("report", {}).get("status") != "KERNEL_CHECKED_TEMPLATE_MODEL_ONLY":
        broken = True
    if by_name["worklist-structure"].get("report", {}).get("status") != "STRUCTURE_MATCHED":
        broken = True
    if broken:
        overall = "TOOL_ERROR"
    elif (refinement_status == "PROVEN" and
          by_name["semantic-composition"]["report"].get("whole_pipeline_proven") is True and
          by_name["coverage"]["report"].get("semantically_proven") ==
          by_name["coverage"]["report"].get("required") and
          all(s["status"] == "COMPLETED" for s in stages)):
        overall = "PROVEN"
    else:
        overall = "NOT_PROVEN"
    summary = {
        "schema_version": 1, "kind": "jabberwock-formal-tool-run",
        "created_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "status": overall,
        "refinement_status": refinement_status or "UNKNOWN",
        "stages": stages,
        "limitations": [
            "Lean kernel success does not prove whole compiler refinement.",
            "Missing proof evidence is NOT_PROVEN, not a passing result.",
            "No compiler acceptance test matrix is run by this command."
        ]
    }
    atomic_json(output / "summary.json", summary)
    print("FINAL:", overall, "|", output / "summary.json", flush=True)
    return 0 if overall == "PROVEN" else (2 if overall == "TOOL_ERROR" else 1)

if __name__ == "__main__":
    raise SystemExit(main())
