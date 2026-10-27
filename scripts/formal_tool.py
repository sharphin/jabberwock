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
        ("emitter-binding", [py, str(scripts / "proof_emitter_correspondence.py"),
                             "--output", str(output / "emitter-binding.json")]),
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
    known_reports = {"inventory": "inventory.json", "emitter-binding": "emitter-binding.json",
                     "lean-evidence": "lean-evidence.json", "refinement": "refinement.json"}
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
        print(name + ": " + stage["status"], flush=True)
    by_name = {stage["name"]: stage for stage in stages}
    broken = any(s["status"] in ("TIMEOUT", "EXECUTION_ERROR") or
                 s.get("report", {}).get("status") in ("INVALID_REPORT", "INVALID_EVIDENCE", "INVALID_INVENTORY", "FAIL")
                 for s in stages)
    refinement_status = by_name["refinement"].get("report", {}).get("status")
    if broken:
        overall = "TOOL_ERROR"
    elif refinement_status == "PROVEN" and all(s["status"] == "COMPLETED" for s in stages):
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
