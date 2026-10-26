#!/usr/bin/env python3
"""Run one OS's independent safety items 1..12 and write ONE JSON report.

A passing probe does not establish formal acceptance of the corresponding item.
Python 3.10+; platform-dependent checks are recorded as NOT_RUN, never passed.
"""
from __future__ import annotations

import argparse
import datetime as datetime
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import time

import local_safety as safety
import safety_platforms as suites

ROOT = Path(__file__).resolve().parents[1]
LOG_LIMIT = 24000


def excerpt(value: str) -> dict:
    """Keep the beginning AND end of diagnostics, including early errors and final summaries."""
    raw = value.encode("utf-8", errors="replace")
    if len(raw) <= LOG_LIMIT:
        return {"text": value, "truncated": False, "bytes": len(raw)}
    front = raw[:LOG_LIMIT // 2].decode("utf-8", errors="replace")
    back = raw[-LOG_LIMIT // 2:].decode("utf-8", errors="replace")
    return {"text": front + "\n...[truncated]...\n" + back,
            "truncated": True, "bytes": len(raw),
            "sha256": hashlib.sha256(raw).hexdigest()}


def version(command: list[str]) -> str | None:
    if not shutil.which(command[0]):
        return None
    try:
        r = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=8)
        return (r.stdout or r.stderr).strip()[:400] if r.returncode == 0 else None
    except (OSError, subprocess.TimeoutExpired):
        return None


def run_probe(probe: dict, report_dir: Path, dry_run: bool) -> dict:
    name, command = probe["name"], probe["cmd"]
    entry = {"name": name, "items": probe["items"], "command": command,
             "environment_overrides": probe["env"], "status": "NOT_RUN",
             "expected": {"exit_code": 0}, "reason": None}
    if probe.get("target_os"):
        entry["target_os"] = probe["target_os"]
    if dry_run:
        entry["status"] = "PLANNED"
        entry["reason"] = probe.get("unavailable_reason")
        return entry
    reason = None
    if probe.get("unavailable_reason"):
        reason = probe["unavailable_reason"]
    elif probe.get("target_os") and probe["target_os"] != suites.host_os():
        reason = "probe requires " + probe["target_os"] + "; results from another OS cannot substitute"
    elif probe["linux"] and not safety.IS_LINUX:
        reason = "Linux-only verification unavailable"
    elif probe["clang"] and not shutil.which(os.environ.get("JW_CLANG", "clang")):
        reason = "clang unavailable"
    elif probe["lake"] and not shutil.which("lake"):
        reason = "Lean lake unavailable"
    elif not shutil.which(command[0]) and not Path(command[0]).is_file():
        reason = "command executable unavailable: " + command[0]
    elif probe["jwc"] and not Path(probe.get("jwc_executable", safety.jwc_path())).is_file():
        reason = "jwc-local not available"
    if reason:
        entry["reason"] = reason
        return entry

    nested = probe.get("case_report") or {
        "edges-8-9": report_dir / "edges-8-9.json",
        "effects-10-11-12": report_dir / "practical-effects.json",
    }.get(name)
    nested = Path(nested) if nested else None
    if nested:
        try:
            # Never attach an old run's cases to the current command.
            nested.unlink(missing_ok=True)
        except OSError as error:
            entry["status"], entry["reason"] = "INFRA_ERROR", "cannot clear old case report: " + str(error)
            return entry

    if name in ("cargo-test", "release-test") and not shutil.which(os.environ.get("JW_CLANG", "clang")):
        entry["coverage_notes"] = [
            "clang unavailable: some Cargo tests return early but are displayed as ok; "
            "native/jwc/sanitizer coverage is not established by this result"
        ]

    env = dict(os.environ)
    for k in ("JW_CFLAGS", "JW_NATIVE", "JW_ROBUST_ASAN", "JW_ROBUST_NATIVE",
              "JW_FUZZ_JWC", "JW_FUZZ", "JW_ROBUST", "JWC_REGEN", "JW_REQUIRE_RETIREMENT_TESTS"):
        env.pop(k, None)
    env.update(probe["env"])
    begin = time.monotonic()
    try:
        run = subprocess.run(command, cwd=ROOT, env=env, text=True,
                             encoding="utf-8", errors="replace",
                             capture_output=True, timeout=probe["timeout"])
        entry["actual"] = {"exit_code": run.returncode,
                           "stdout": excerpt(run.stdout), "stderr": excerpt(run.stderr)}
        entry["status"] = "PROBE_PASS" if run.returncode == 0 else "FAIL"
        if run.returncode < 0:
            entry["status"] = "CRASH"
            entry["reason"] = "terminated by signal " + str(-run.returncode)
        elif run.returncode == 2 and nested:
            entry["status"] = "INFRA_ERROR"
    except subprocess.TimeoutExpired as error:
        entry["status"], entry["reason"] = "TIMEOUT", "timeout after " + str(probe["timeout"]) + " seconds"
        entry["actual"] = {"stdout": excerpt((error.stdout or b"").decode("utf-8", "replace") if isinstance(error.stdout, bytes) else (error.stdout or "")),
                           "stderr": excerpt((error.stderr or b"").decode("utf-8", "replace") if isinstance(error.stderr, bytes) else (error.stderr or ""))}
    except OSError as error:
        entry["status"], entry["reason"] = "INFRA_ERROR", str(error)
    entry["seconds"] = round(time.monotonic() - begin, 3)

    # Existing targeted scripts write per-case details. Inline them so the JSON is
    # independently usable without uploading adjacent log files.
    if nested and nested.is_file():
        try:
            details = json.loads(nested.read_text(encoding="utf-8"))
            if not isinstance(details, dict) or not isinstance(details.get("results"), list):
                raise ValueError("expected an object containing a results list")
            entry["case_report"] = details
            cases = details["results"]
            if not cases or any(not isinstance(r, dict) or r.get("case_status", r.get("status")) not in ("pass", "fail")
                                for r in cases):
                raise ValueError("no executed cases or invalid case status")
            if probe.get("expected_cases") is not None and len(cases) != probe["expected_cases"]:
                raise ValueError("expected " + str(probe["expected_cases"]) + " cases, got " + str(len(cases)))
            if entry["status"] not in ("TIMEOUT", "CRASH", "INFRA_ERROR"):
                if details.get("infrastructure_errors"):
                    entry["status"] = "INFRA_ERROR"
                    entry["reason"] = "case report contains infrastructure errors"
                elif any(r.get("case_status", r.get("status")) == "fail" for r in cases):
                    entry["status"] = "FAIL"
        except (OSError, ValueError) as error:
            if entry["status"] not in ("TIMEOUT", "CRASH"):
                entry["status"], entry["reason"] = "INFRA_ERROR", "invalid case report: " + str(error)
    elif nested and entry["status"] not in ("TIMEOUT", "CRASH", "INFRA_ERROR"):
        entry["status"], entry["reason"] = "INFRA_ERROR", "case report was not produced"
    return entry


def main(argv=None, *, target_os=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--platform", choices=suites.TARGETS, help="default: this OS; Windows and Linux results stay separate")
    ap.add_argument("--output", type=Path, help="default: target/safety-<OS>-report.json")
    ap.add_argument("--scale", type=int, default=1, help="fuzz repetitions multiplier")
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args(argv)
    if args.scale < 1:
        ap.error("--scale must be >= 1")
    if target_os and args.platform and target_os != args.platform:
        ap.error("this entry point only runs the " + target_os + " suite")
    selected_os = target_os or args.platform or suites.host_os()
    if selected_os not in suites.TARGETS:
        ap.error("only Windows and Linux have safety suites")
    if not args.dry_run and selected_os != suites.host_os():
        ap.error("run the " + selected_os + " suite on " + selected_os + "; WSL2 produces Linux results, not Windows results")
    output = (args.output or ROOT / "target" / ("safety-" + selected_os + "-report.json")).resolve()
    work = output.parent / (output.stem + "-work")
    if not args.dry_run:
        work.mkdir(parents=True, exist_ok=True)

    catalog = suites.catalog(selected_os, args.scale, work)
    # Build required tooling once, before running item 1. One probe may cover
    # several criteria; run it once and reference its result from each item.
    prereq = [safety.probe("cargo-debug-build", [12], ["cargo", "build"], timeout=1200)]
    prereq[0]["target_os"] = selected_os
    prereq.extend(p for p in catalog if p["name"] in ("cargo-build", "jwc-build"))
    ordered = prereq[:]
    seen = {p["name"] for p in ordered}
    for n in range(1, 13):
        for p in catalog:
            if n in p["items"] and p["name"] not in seen:
                ordered.append(p)
                seen.add(p["name"])

    def recorded_version(command):
        return None if args.dry_run else version(command)
    git = recorded_version(["git", "rev-parse", "HEAD"])
    report = {
        "schema_version": 2,
        "suite": {"id": selected_os + "-12", "target_os": selected_os, "item_count": 12},
        "scope": "Probe results only. Not proof or certification of all 12 acceptance criteria.",
        "started_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "commit": git,
        "platform": {"os": platform.platform(), "system": platform.system(),
                     "machine": platform.machine(), "python": sys.version},
        "tools": {"rustc": recorded_version(["rustc", "--version"]), "cargo": recorded_version(["cargo", "--version"]),
                  "clang": recorded_version([os.environ.get("JW_CLANG", "clang"), "--version"]),
                  "lake": recorded_version(["lake", "--version"])},
        "scale": args.scale, "preparation": [], "items": [], "results": [],
    }
    produced = {}
    for p in prereq:
        r = run_probe(p, work, args.dry_run)
        produced[p["name"]] = r
        report["preparation"].append(r)
        print("PREP " + p["name"] + ": " + r["status"], flush=True)
    for n in range(1, 13):
        references = []
        for p in ordered:
            if p["name"] in {q["name"] for q in prereq} or n not in p["items"]:
                continue
            if p["name"] not in produced:
                produced[p["name"]] = run_probe(p, work, args.dry_run)
                print(f"[{n:02d}/12] {p['name']}: {produced[p['name']]['status']}", flush=True)
            references.append(p["name"])
        # Include previously run shared probes for this criterion.
        references = [p["name"] for p in ordered if n in p["items"] and p["name"] in produced]
        results = [produced[x] for x in references]
        problems = [x for x in results if x["status"] in
                    ("FAIL", "CRASH", "TIMEOUT", "INFRA_ERROR")]
        not_run = [x for x in results if x["status"] in ("NOT_RUN", "PLANNED")]
        status = ("FAIL" if problems else "NOT_RUN" if not_run or not results
                  else "PROBES_PASS_ONLY")
        if args.dry_run:
            status = "PLANNED"
        report["items"].append({"item": n, "name": safety.NAMES[n], "status": status,
                                "id": selected_os + "-" + str(n).zfill(2), "target_os": selected_os,
                                "acceptance": "NOT_ASSESSED", "probe_names": references,
                                "failures": [x["name"] for x in problems],
                                "unavailable": [x["name"] for x in not_run]})
        print(f"{selected_os.upper()} ITEM {n:02d}/12 {safety.NAMES[n]}: {status}", flush=True)
    report["results"] = [produced[p["name"]] for p in ordered if p["name"] in produced]
    report["finished_utc"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    if not args.dry_run:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print("Report: " + str(output), flush=True)
    else:
        print("Dry run; no report written", flush=True)
    return int(any(i["status"] in ("FAIL", "NOT_RUN") for i in report["items"]))


if __name__ == "__main__":
    sys.exit(main())
