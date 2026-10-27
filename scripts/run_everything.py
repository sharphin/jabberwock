#!/usr/bin/env python3
"""One command: run the complete local Jabberwock verification battery.

Commands are executed, not inferred from summary metadata. Each stage keeps
its stdout/stderr and independent exit status; no failed step is silently
skipped and unknown/unproved acceptance always fails the final gate.
"""
from __future__ import annotations

import argparse
import datetime as dt
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
WINDOWS = platform.system() == "Windows"
LINUX = platform.system() == "Linux"

def plans(output: Path, *, quick: bool = False) -> list[tuple[str, list[str]]]:
    py = sys.executable
    safety = (["cmd", "/d", "/c", str(ROOT / "scripts/safety_windows.cmd")]
              if WINDOWS else [py, str(ROOT / "scripts/safety_linux.py")])
    benchmark = (["cmd", "/d", "/c", str(ROOT / "scripts/benchmark_windows.cmd")]
                 if WINDOWS else [py, str(ROOT / "scripts/benchmark_report.py")])
    target_os = "windows" if WINDOWS else "linux"
    practical = output / "safety.json"
    formal = output / "formal.json"
    result = [
        ("python-regressions", [py, "-m", "unittest", "discover", "-s", "scripts", "-p", "test_*.py"]),
        ("practical-safety", [*safety, "--compiler", "all", "--output", str(practical)]),
        ("emitter-proof-binding", [py, str(ROOT / "scripts/proof_emitter_correspondence.py"),
                                   "--output", str(output / "emitter-proof-binding.json")]),
        ("formal-proofs", [py, str(ROOT / "scripts/formal_report.py"),
                           "--compiler", "all", "--abstract-model", "--require-full",
                           "--output", str(formal)]),
        ("acceptance-34", [py, str(ROOT / "scripts/acceptance_report.py"),
                            "--os", target_os, "--practical", str(practical), "--formal", str(formal),
                            "--output", str(output / "acceptance-34.json")]),
        ("benchmark-four-way", [*benchmark, *(["--full"] if not quick else []),
                                "--output", str(output / "benchmark-four-way.json")]),
        ("legacy-optimization-counts", [py, str(ROOT / "scripts/check_bench_checks.py")]),
    ]
    return result


def execute_stage(label: str, command: list[str], output: Path, *, timeout: int) -> dict:
    stdout = output / (label + ".stdout.log")
    stderr = output / (label + ".stderr.log")
    started = dt.datetime.now(dt.timezone.utc).isoformat()
    begin = time.monotonic()
    record = {"name": label, "command": command, "started_utc": started,
              "stdout_log": str(stdout), "stderr_log": str(stderr)}
    print("START", label, flush=True)
    with stdout.open("wb") as out, stderr.open("wb") as err:
        try:
            proc = subprocess.run(command, cwd=ROOT, stdout=out, stderr=err,
                                  timeout=timeout, check=False)
            record["exit_code"] = proc.returncode
            record["status"] = "EXIT_ZERO" if proc.returncode == 0 else "EXIT_NONZERO"
        except subprocess.TimeoutExpired:
            record["status"] = "TIMEOUT"
            record["reason"] = "time limit exceeded: " + str(timeout) + "s"
        except OSError as error:
            record["status"] = "INFRA_ERROR"
            record["reason"] = str(error)
    record["duration_s"] = round(time.monotonic() - begin, 3)
    print(record["status"], label, "(logs:", stdout, stderr, ")", flush=True)
    return record


def summarize(stages: list[dict], output: Path, *, quick: bool) -> dict:
    by_name = {x["name"]: x for x in stages}
    expected = {"python-regressions", "practical-safety", "emitter-proof-binding", "formal-proofs",
                "acceptance-34", "benchmark-four-way", "legacy-optimization-counts"}
    verdict = "FAIL"
    reason = "one or more verification stages failed"
    if set(by_name) != expected:
        reason = "required stages missing or duplicated"
    elif all(x["status"] == "EXIT_ZERO" for x in stages):
        try:
            acceptance = json.loads((output / "acceptance-34.json").read_text(encoding="utf-8"))
            benchmark = json.loads((output / "benchmark-four-way.json").read_text(encoding="utf-8"))
            practical = json.loads((output / "safety.json").read_text(encoding="utf-8"))
            formal = json.loads((output / "formal.json").read_text(encoding="utf-8"))
            count = acceptance.get("summary", {})
            if (acceptance.get("status") == "ACCEPTED" and count.get("accepted") == 34 and
                    practical.get("run_status") == "COMPLETE" and
                    formal.get("summary", {}).get("all_compilers_proven") is True and
                    all(c.get("status") == "PASS" for c in benchmark.get("cases", {}).values()) and
                    len(benchmark.get("cases", {})) == (5 if quick else 21)):
                verdict, reason = "PASS", "all required evidence and execution stages passed"
            else:
                reason = "34-obligation proof, safety, or benchmark acceptance not established"
        except (OSError, ValueError, TypeError, AttributeError) as error:
            reason = "missing or malformed final reports: " + str(error)
    return {"schema_version": 1, "kind": "local-full-verification",
            "status": verdict, "reason": reason, "mode": "quick" if quick else "full",
            "host_os": platform.system(), "stages": stages,
            "passed_stages": sum(x["status"] == "EXIT_ZERO" for x in stages),
            "required_stages": len(expected),
            "created_utc": dt.datetime.now(dt.timezone.utc).isoformat()}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "target" / "verification-complete")
    parser.add_argument("--quick", action="store_true",
                        help="5 benchmark cases rather than 21; never qualifies as full acceptance")
    parser.add_argument("--stage-timeout", type=int, default=86400)
    args = parser.parse_args(argv)
    if not (WINDOWS or LINUX):
        parser.error("supported only on Windows and Linux")
    if args.stage_timeout < 1:
        parser.error("stage timeout must be positive")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    stages = []
    for label, command in plans(output, quick=args.quick):
        # Delete prior output: never reuse a report from a previous attempt.
        report_name = {"practical-safety": "safety.json", "formal-proofs": "formal.json",
                       "emitter-proof-binding": "emitter-proof-binding.json",
                       "acceptance-34": "acceptance-34.json",
                       "benchmark-four-way": "benchmark-four-way.json"}.get(label)
        if report_name:
            try:
                (output / report_name).unlink(missing_ok=True)
            except OSError as exc:
                stages.append({"name": label, "status": "INFRA_ERROR", "reason": str(exc)})
                continue
        stages.append(execute_stage(label, command, output, timeout=args.stage_timeout))
    result = summarize(stages, output, quick=args.quick)
    if args.quick:
        result["status"] = "FAIL"
        result["reason"] = "quick sampling cannot satisfy full acceptance"
    destination = output / "summary.json"
    destination.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print("FINAL:", result["status"], "|", result["reason"], flush=True)
    print("Report:", destination, flush=True)
    return 0 if result["status"] == "PASS" else 1


if __name__ == "__main__":
    sys.exit(main())
