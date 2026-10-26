#!/usr/bin/env python3
"""Compare programs compiled by self-hosted jwc with Rust, C++, and Go ONLY.

The Rust-based Jabberwock compiler is used to bootstrap jwc, not benchmarked.
Linux-only measurement uses bench/measure.c (wait4). One JSON report.
"""
from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import statistics
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
BENCH = ROOT / "bench"
QUICK = ("fib", "matmul", "mandelbrot", "sort", "linkedlist")
TARGETS = ("jwc", "rust", "cpp", "go")
MAX_LOG = 12000


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(command, *, timeout=600, env=None):
    p = subprocess.run([str(v) for v in command], cwd=ROOT, env=env, capture_output=True,
                       text=True, encoding="utf-8", errors="replace", timeout=timeout)
    return {"cmd": [str(v) for v in command], "exit": p.returncode,
            "stdout": p.stdout[-MAX_LOG:], "stderr": p.stderr[-MAX_LOG:]}


def version(command):
    try:
        p = run(command, timeout=20)
        return (p["stdout"] or p["stderr"]).splitlines()[0] if p["exit"] == 0 else None
    except (OSError, subprocess.TimeoutExpired, IndexError):
        return None


def fingerprint():
    return {"platform": platform.platform(), "machine": platform.machine(),
            "processor": platform.processor(),
            "clang": version(["clang", "--version"]),
            "clang++": version(["clang++", "--version"]),
            "rustc": version(["rustc", "--version"]),
            "go": version(["go", "version"])}


def parse_measure(value):
    try:
        low, median, high, rss = (float(v) for v in value.strip().split())
    except ValueError as e:
        raise ValueError("invalid measure.c result: " + repr(value)) from e
    if low < 0 or median < low or high < median or rss < 0:
        raise ValueError("invalid measurement ordering: " + repr(value))
    return {"min_s": low, "median_s": median, "max_s": high, "peak_rss_mb": rss}


def compare(current, baseline, threshold):
    if current.get("schema_version") != 2 or baseline.get("schema_version") != 2:
        return {"status": "NOT_COMPARABLE", "reason": "report schema differs"}
    if (current["environment"] != baseline.get("environment")
            or current["runs"] != baseline.get("runs")
            or current["flags"] != baseline.get("flags")
            or current["compiler_hashes"].get("runtime") != baseline.get("compiler_hashes", {}).get("runtime")
            or current["source_hashes"] != baseline.get("source_hashes")):
        return {"status": "NOT_COMPARABLE", "reason": "environment, workload sources, runtime, flags or repeats differ"}
    rows = []
    for name, case in current["cases"].items():
        prev = baseline.get("cases", {}).get(name, {})
        if case.get("status") != "PASS" or prev.get("status") != "PASS":
            continue
        old = prev["metrics"]["jwc"]["median_s"]
        new = case["metrics"]["jwc"]["median_s"]
        if old <= 0:
            continue
        percent = (new / old - 1) * 100
        rows.append({"case": name, "old_s": old, "new_s": new,
                     "change_percent": round(percent, 2), "regression": percent > threshold})
    return {"status": "COMPARED" if rows else "NO_MATCHING_CASES",
            "threshold_percent": threshold, "cases": rows,
            "regressions": [r["case"] for r in rows if r["regression"]]}

def rotated_targets(round_id: int) -> tuple[str, ...]:
    """Rotate and periodically reverse orders to limit fixed-order bias."""
    if round_id < 0:
        raise ValueError("negative round")
    offset = round_id % len(TARGETS)
    order = TARGETS[offset:] + TARGETS[:offset]
    return tuple(reversed(order)) if (round_id // len(TARGETS)) % 2 else order


def stable_output(digests: list[str]) -> bool:
    """Every repetition must produce exactly the same output."""
    return bool(digests) and len(set(digests)) == 1



def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--full", action="store_true")
    p.add_argument("--cases", nargs="+")
    p.add_argument("--runs", type=int, default=5)
    p.add_argument("--jwc", type=Path, help="existing self-hosted compiler (skips bootstrap)")
    p.add_argument("--output", type=Path, default=ROOT / "target/benchmark-report.json")
    p.add_argument("--baseline", type=Path)
    p.add_argument("--threshold", type=float, default=15)
    p.add_argument("--dry-run", action="store_true")
    a = p.parse_args()
    if a.runs < 1 or a.threshold < 0:
        p.error("runs must be >=1, threshold >=0")
    all_names = sorted(x.stem for x in (BENCH / "jabberwock").glob("*.jw"))
    selected = a.cases if a.cases else all_names if a.full else list(QUICK)
    if not selected or len(selected) != len(set(selected)) or set(selected) - set(all_names):
        p.error("unknown or duplicate case; available: " + ", ".join(all_names))
    if a.dry_run:
        print("Compilers: jwc (self-hosted) / Rust (rustc) / C++ (clang++) / Go (go)")
        print("Cases:", ", ".join(selected))
        return 0

    out = a.output.resolve()
    out.parent.mkdir(parents=True, exist_ok=True)
    work = out.parent / (out.stem + "-work")
    work.mkdir(parents=True, exist_ok=True)
    exe_suffix = ".exe" if os.name == "nt" else ""
    known_jwc = [ROOT / "target/release/jwc-local", ROOT / "target/release/jwc",
                 ROOT / "jwc-bin", ROOT / "target/release/jwc-benchmark"]
    existing_jwc = next((candidate for candidate in known_jwc
                         if candidate.is_file() and os.access(candidate, os.X_OK)), None)
    jwc = a.jwc.resolve() if a.jwc else (existing_jwc or ROOT / "target/release/jwc-benchmark")
    bootstrap_needed = not a.jwc and existing_jwc is None
    rust_bootstrap = ROOT / "target/release" / ("jabberwock" + exe_suffix)
    measure = work / "measure"
    flags = {"jwc_codegen": ["--ll"], "jwc_native": ["clang", "-O2", "-flto", "-fuse-ld=lld"],
             "rust": ["rustc", "-C", "opt-level=3", "-C", "lto=fat", "--edition", "2021"],
             "cpp": ["clang++", "-O2", "-flto", "-fuse-ld=lld", "-std=c++17", "-pthread"],
             "go": ["go", "build"]}
    report = {"schema_version": 2, "started_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
              "git_sha": version(["git", "rev-parse", "HEAD"]), "environment": fingerprint(),
              "runs": a.runs, "suite": selected, "subjects": list(TARGETS),
              "flags": flags, "compiler_hashes": {}, "source_hashes": {}, "preparation": [], "cases": {},
              "baseline_comparison": None,
              "limitations": ("Rust-based Jabberwock compiler is bootstrap only. jwc emits LLVM; clang links shared C runtime. "
                              "Equivalent outputs required. Differences in libraries and algorithms remain; "
                              "LTO flags are set for each compiled target but are not identical optimizers.")}

    def save():
        report["updated_utc"] = dt.datetime.now(dt.timezone.utc).isoformat()
        out.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    def prepare(command, label, timeout=1200):
        try:
            res = run(command, timeout=timeout)
            report["preparation"].append({"name": label, **res})
            save()
            return res["exit"] == 0
        except (OSError, subprocess.TimeoutExpired) as e:
            report["preparation"].append({"name": label, "error": str(e)})
            save()
            return False

    missing = [v for v in ("cc", "clang", "clang++", "rustc", "ld.lld", "go") if not shutil.which(v)]
    if bootstrap_needed and not shutil.which("cargo"):
        missing.append("cargo")
    reason = None
    if not sys.platform.startswith("linux"):
        reason = "bench/measure.c needs Linux wait4; use Linux or WSL2"
    elif missing:
        reason = "missing programs: " + ", ".join(missing)
    elif bootstrap_needed:
        if not prepare(["cargo", "build", "--release"], "bootstrap-rust-compiler"):
            reason = "failed to build bootstrap Rust Jabberwock compiler"
        elif not prepare([rust_bootstrap, "build", "-o", jwc, ROOT / "jwc/main.jw"], "bootstrap-jwc", timeout=1800):
            reason = "failed to bootstrap jwc"
    if not reason and not jwc.is_file():
        reason = "jwc executable missing: " + str(jwc)
    if not reason and not prepare(["cc", "-O2", "-o", measure, BENCH / "measure.c"], "measure-helper"):
        reason = "failed to compile Linux timing helper"
    if reason:
        for name in selected:
            report["cases"][name] = {"status": "NOT_RUN", "reason": reason}
        save()
        print("NOT_RUN:", reason)
        return 2

    report["compiler_hashes"] = {"jwc": sha(jwc), "runtime": sha(ROOT / "runtime/jw_rt.c"),
                                 "rustc": report["environment"]["rustc"],
                                 "clang++": report["environment"]["clang++"],
                                 "go": report["environment"]["go"]}
    save()
    for index, name in enumerate(selected, 1):
        dest = work / name
        dest.mkdir(exist_ok=True)
        source = BENCH / "jabberwock" / (name + ".jw")
        cpp = BENCH / "cpp" / (name + ".cpp")
        rust = BENCH / "rust" / (name + ".rs")
        go = BENCH / "go" / (name + ".go")
        report["source_hashes"][name] = {lang: sha(path) for lang, path in (("jwc", source), ("cpp", cpp), ("rust", rust), ("go", go))}
        ll = dest / "jwc.ll"
        binaries = {k: dest / k for k in TARGETS}
        item = {"status": "INFRA_ERROR", "commands": [], "metrics": {}, "outputs": {}}
        report["cases"][name] = item
        try:
            # LLVM can be larger than log excerpts: capture full stdout as bytes.
            ll_process = subprocess.run([str(jwc), "--ll", str(source)], cwd=ROOT,
                                        capture_output=True, timeout=600)
            if ll_process.returncode or b"define void @jw_main(" not in ll_process.stdout:
                raise ValueError("jwc LLVM capture failed")
            ll.write_bytes(ll_process.stdout)
            item["commands"].append({"stage": "jwc-LLVM", "exit": ll_process.returncode,
                                     "cmd": [str(jwc), "--ll", str(source)],
                                     "llvm_bytes": len(ll_process.stdout),
                                     "stderr": ll_process.stderr.decode("utf-8", "replace")[-MAX_LOG:]})
            compile_steps = [
                ("jwc-native", [*flags["jwc_native"], ll, ROOT / "runtime/jw_rt.c",
                                "-lm", "-pthread", "-Wno-override-module", "-o", binaries["jwc"]]),
                ("rust", [*flags["rust"], "-o", binaries["rust"], rust]),
                ("cpp", [*flags["cpp"], "-o", binaries["cpp"], cpp]),
                ("go", [*flags["go"], "-o", binaries["go"], go])]
            for label, command in compile_steps:
                result = run(command, timeout=600)
                item["commands"].append({"stage": label, **result})
                if result["exit"]:
                    raise ValueError(label + " compilation failed")
            timings = {target: [] for target in TARGETS}
            memory = {target: [] for target in TARGETS}
            checksums = {target: [] for target in TARGETS}
            for round_id in range(a.runs):
                # Rotate the order so one language is not always first or last.
                order = rotated_targets(round_id)
                for target in order:
                    output = dest / (target + ".run" + str(round_id) + ".stdout")
                    result = run([measure, 1, output, binaries[target]], timeout=3600)
                    item["commands"].append({"stage": "measure-" + target, "round": round_id, **result})
                    if result["exit"]:
                        raise ValueError(target + " execution failed in round " + str(round_id))
                    one = parse_measure(result["stdout"])
                    timings[target].append(one["median_s"])
                    memory[target].append(one["peak_rss_mb"])
                    data = output.read_bytes()
                    checksums[target].append(hashlib.sha256(data).hexdigest())
            for target in TARGETS:
                if not stable_output(checksums[target]):
                    raise ValueError(target + " output changes across repetitions")
                values = sorted(timings[target])
                item["metrics"][target] = {"min_s": values[0],
                                           "median_s": statistics.median(values),
                                           "max_s": values[-1],
                                           "peak_rss_mb": max(memory[target]),
                                           "samples_s": timings[target]}
                item["outputs"][target] = {"sha256": checksums[target][0],
                                           "all_repetitions_verified": len(checksums[target])}
            if len({v["sha256"] for v in item["outputs"].values()}) != 1:
                item["status"] = "OUTPUT_MISMATCH"
            else:
                item["status"] = "PASS"
                item["ratios"] = {
                    "jwc_vs_rust": round(item["metrics"]["jwc"]["median_s"] /
                                         item["metrics"]["rust"]["median_s"], 4),
                    "jwc_vs_cpp": round(item["metrics"]["jwc"]["median_s"] /
                                        item["metrics"]["cpp"]["median_s"], 4),
                    "jwc_vs_go": round(item["metrics"]["jwc"]["median_s"] /
                                       item["metrics"]["go"]["median_s"], 4)}
        except (OSError, ValueError, subprocess.TimeoutExpired) as e:
            item["status"] = "FAIL"
            item["reason"] = str(e)
        save()
        print(f"[{index}/{len(selected)}] {name}: {item['status']}", flush=True)
    if a.baseline:
        try:
            baseline = json.loads(a.baseline.read_text(encoding="utf-8"))
            report["baseline_comparison"] = compare(report, baseline, a.threshold)
        except (OSError, ValueError, KeyError, TypeError) as e:
            report["baseline_comparison"] = {"status": "INVALID_BASELINE", "reason": str(e)}
    save()
    print("Report:", out)
    comparison = report["baseline_comparison"]
    return int(any(x["status"] != "PASS" for x in report["cases"].values())
               or (comparison is not None and
                   (comparison["status"] != "COMPARED" or bool(comparison.get("regressions")))))


if __name__ == "__main__":
    sys.exit(main())
