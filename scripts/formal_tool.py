#!/usr/bin/env python3
"""One-command formal verification tool runner (not the old compiler test suite).

Runs implementation inventory, Lean kernel/axiom audit, the source-emitter
correspondence checker, and the evidence/refinement gate. Missing proof
evidence never becomes PASS. No test fixtures or CI are created.
"""
from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import platform
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

def atomic_json(path: Path, value: dict) -> None:
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    os.replace(temporary, path)

def log_fingerprint(path: Path) -> dict:
    """Record log bytes and a digest so copied evidence can be checked later.

    This is an integrity fingerprint, not a digital signature or immutable seal.
    """
    try:
        raw = path.read_bytes()
    except OSError as exc:
        return {"path": str(path), "status": "UNAVAILABLE", "reason": str(exc)}
    return {"path": str(path), "status": "AVAILABLE", "bytes": len(raw),
            "sha256": hashlib.sha256(raw).hexdigest()}


def attach_log_fingerprints(record: dict, stdout: Path, stderr: Path) -> dict:
    record["log_evidence"] = {
        "stdout": log_fingerprint(stdout),
        "stderr": log_fingerprint(stderr),
    }
    return record


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
        return attach_log_fingerprints(
            {"name": name, "status": "COMPLETED" if completed.returncode == 0 else "NONZERO_EXIT",
             "returncode": completed.returncode,
             "seconds": round(time.monotonic() - start, 3),
             "stdout": str(stdout), "stderr": str(stderr)}, stdout, stderr)
    except subprocess.TimeoutExpired:
        return attach_log_fingerprints(
            {"name": name, "status": "TIMEOUT", "seconds": round(time.monotonic() - start, 3),
             "stdout": str(stdout), "stderr": str(stderr)}, stdout, stderr)
    except OSError as exc:
        return attach_log_fingerprints(
            {"name": name, "status": "EXECUTION_ERROR", "reason": str(exc),
             "seconds": round(time.monotonic() - start, 3),
             "stdout": str(stdout), "stderr": str(stderr)}, stdout, stderr)

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
        ("runtime-generation-binding", [py, str(scripts / "runtime_generation_binding.py"),
                                         "--output", str(output / "runtime-generation-binding.json")]),
        ("runtime-generation-proof", [py, str(scripts / "runtime_generation_proof.py"),
                                       "--output", str(output / "runtime-generation-proof.json"),
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
    plans.append(("refinement-frontier", [py, str(scripts / "refinement_frontier.py"),
                                          "--inventory", str(output / "inventory.json"),
                                          "--refinement", str(output / "refinement.json"),
                                          "--runtime-model", str(output / "runtime-generation-proof.json"),
                                          "--output", str(output / "refinement-frontier.json")]))
    plans.append(("coverage", [py, str(scripts / "formal_proof_coverage.py"),
                                "--boundaries", str(output / "semantic-boundaries.json"),
                                "--bindings", str(output / "stage-refinement-binding.json"),
                                "--refinement", str(output / "refinement.json"),
                                "--output", str(output / "proof-coverage.json")]))
    # Bind every stage output to its declared report schema. A successful
    # process exit must not let an unrelated or malformed JSON report satisfy
    # the gate. The optional kernel manifest report has no "kind" field yet;
    # its status and compiler_refinement fields remain independently inspected.
    known_reports = {
        "inventory": ("inventory.json", "whole-implementation-proof-inventory"),
        "semantic-boundaries": ("semantic-boundaries.json", "first-five-semantic-refinement-boundaries"),
        "semantic-composition": ("semantic-composition.json", "compiler-semantic-composition"),
        "stage-refinement-binding": ("stage-refinement-binding.json", "stage-refinement-bindings"),
        "worklist-structure": ("worklist-structure.json", "worklist-structure-audit"),
        "emitter-binding": ("emitter-binding.json", "emitter-correspondence"),
        "emitter-semantics": ("emitter-semantics.json", "emitter-template-semantics"),
        "runtime-generation-binding": ("runtime-generation-binding.json", "runtime-generation-structural-binding"),
        "runtime-generation-proof": ("runtime-generation-proof.json", "runtime-generation-arithmetic-proof"),
        "lean-evidence": ("lean-evidence.json", None),
        "refinement": ("refinement.json", "refinement-transition-audit"),
        "refinement-frontier": ("refinement-frontier.json", "refinement-work-frontier"),
        "coverage": ("proof-coverage.json", "formal-proof-coverage"),
    }
    # Remove stale reports even for optional stages.
    for filename, _expected_kind in known_reports.values():
        (output / filename).unlink(missing_ok=True)
    stages = []
    for name, command in plans:
        stage = run_stage(name, command, output, args.stage_timeout)
        report_spec = known_reports.get(name)
        if report_spec:
            filename, expected_kind = report_spec
            report = read_report(output / filename, expected_kind)
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
    # Distinguish infrastructure failures from honest unproved obligations.
    # The gate must remain capable of accepting independently established
    # refinements in the future, without treating model-only checks as proofs.
    infrastructure_errors = any(
        stage["status"] in ("TIMEOUT", "EXECUTION_ERROR")
        or (stage["status"] == "NONZERO_EXIT" and stage["name"] != "refinement")
        or (stage["name"] == "refinement" and stage.get("returncode") not in (0, 1))
        or stage.get("report", {}).get("status") in (
            "INVALID_REPORT", "INVALID_EVIDENCE", "INVALID_INVENTORY",
            "INVALID_BASELINE", "FAIL", "NOT_CHECKED", "STRUCTURE_MISMATCH")
        for stage in stages
    )
    reports = {name: stage.get("report", {}) for name, stage in by_name.items()}
    expected_model_statuses = {
        "emitter-binding": "STRUCTURE_MATCHED",
        "worklist-structure": "STRUCTURE_MATCHED",
        "emitter-semantics": "KERNEL_CHECKED_TEMPLATE_MODEL_ONLY",
        "runtime-generation-binding": "STRUCTURE_MATCHED",
        "runtime-generation-proof": "KERNEL_CHECKED_RUNTIME_GENERATION_MODELS_ONLY",
    }
    for name, expected in expected_model_statuses.items():
        if reports[name].get("status") != expected:
            infrastructure_errors = True
    if "lean-evidence" in reports:
        lean_evidence = reports["lean-evidence"]
        if (lean_evidence.get("status") != "KERNEL_CHECKED"
                or lean_evidence.get("compiler_refinement") != "NOT_PROVEN"):
            infrastructure_errors = True
    # Reject a reported successful stage with a missing or contradictory log
    # fingerprint. Hashing does not seal logs, but a missing digest must not
    # silently pass the formal-tool aggregation boundary.
    for stage in stages:
        logs = stage.get("log_evidence", {})
        for channel in ("stdout", "stderr"):
            record = logs.get(channel, {})
            if (record.get("status") != "AVAILABLE"
                    or not isinstance(record.get("bytes"), int)
                    or record["bytes"] < 0
                    or not isinstance(record.get("sha256"), str)
                    or len(record["sha256"]) != 64):
                infrastructure_errors = True


    refinement_report = reports["refinement"]
    composition_report = reports["semantic-composition"]
    coverage_report = reports["coverage"]
    frontier_report = reports["refinement-frontier"]
    boundaries_report = reports["semantic-boundaries"]
    bindings_report = reports["stage-refinement-binding"]
    inventory_report = reports["inventory"]

    # These stages describe proof obligations and are not proof engines.
    # Their declared inventory and model-only status must never be interpreted
    # as completing the compiler implementation refinement.
    if (boundaries_report.get("status") != "NOT_PROVEN"
            or bindings_report.get("status") != "NOT_PROVEN"
            or inventory_report.get("status") != "NOT_PROVEN"
            or frontier_report.get("status") != "PLAN_READY_NOT_PROVEN"):
        infrastructure_errors = True
    if composition_report.get("status") not in ("NOT_PROVEN", "PROVEN"):
        infrastructure_errors = True
    if coverage_report.get("status") not in ("NOT_PROVEN", "PROVEN"):
        infrastructure_errors = True
    if refinement_report.get("status") not in ("NOT_PROVEN", "PROVEN"):
        infrastructure_errors = True

    # Check the producer's self-consistency. An unproved or mismatched stage
    # cannot become a positive end-to-end verdict by changing one status.
    required = refinement_report.get("required")
    proved = refinement_report.get("proved")
    findings = refinement_report.get("findings")
    if (not isinstance(required, int) or isinstance(required, bool)
            or not isinstance(proved, int) or isinstance(proved, bool)
            or required <= 0 or not 0 <= proved <= required
            or not isinstance(findings, list) or len(findings) != required
            or inventory_report.get("total_obligations") != required):
        infrastructure_errors = True
    composition_required = composition_report.get("required_transitions")
    composition_proved = composition_report.get("proven_transitions")
    if (not isinstance(composition_required, int)
            or not isinstance(composition_proved, int)
            or isinstance(composition_required, bool)
            or isinstance(composition_proved, bool)
            or composition_required <= 0
            or not 0 <= composition_proved <= composition_required):
        infrastructure_errors = True

    ref_proven = (refinement_report.get("status") == "PROVEN"
                  and proved == required and not infrastructure_errors)
    composition_proven = (
        composition_report.get("status") == "PROVEN"
        and composition_report.get("whole_pipeline_proven") is True
        and composition_proved == composition_required
    )
    coverage_required = coverage_report.get("required")
    coverage_proven = (
        coverage_report.get("status") == "PROVEN"
        and isinstance(coverage_required, int)
        and not isinstance(coverage_required, bool)
        and coverage_required > 0
        and coverage_report.get("semantically_proven") == coverage_required
    )
    positive = ref_proven and composition_proven and coverage_proven
    # A wholly proved refinement must be accompanied by both the composed
    # semantics and full coverage. Partial certificates are not promoted.
    if (refinement_report.get("status") == "PROVEN") != positive:
        infrastructure_errors = True
    # The current composition/coverage producers deliberately emit NOT_PROVEN.
    # This branch becomes reachable only after those producers implement real,
    # independently verified semantic evidence and consistently report it.
    if infrastructure_errors:
        overall = "TOOL_ERROR"
    elif positive and all(stage["status"] == "COMPLETED" for stage in stages):
        overall = "PROVEN"
    else:
        overall = "NOT_PROVEN"
    refinement_status = refinement_report.get("status")
    summary = {
        "schema_version": 1, "kind": "jabberwock-formal-tool-run",
        "created_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "environment": {
            "python_version": sys.version,
            "python_executable": sys.executable,
            "platform": platform.platform(),
            "working_tree": str(ROOT),
        },
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
