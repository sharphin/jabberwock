#!/usr/bin/env python3
"""One-command Jabberwock benchmark: one JSON, optional same-machine regression comparison.

Uses the existing bench/run.sh measurements and compiler flags unchanged.
A mismatch (the legacy script only warns) is treated as a hard failure here.
"""
from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
BENCH = ROOT / "bench"
QUICK = ("fib", "matmul", "mandelbrot", "sort", "linkedlist")
VARIANTS = ("jabberwock", "no_elide", "unchecked", "go", "cpp", "cpp_assert", "rust")
CELL = re.compile(r"^([0-9.]+) \[([0-9.]+)[–-]([0-9.]+)\]$")
EXCERPT = 12000


def version(argv: list[str]) -> str | None:
    if not shutil.which(argv[0]):
        return None
    try:
        p = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=10)
        return ((p.stdout or p.stderr).splitlines() or [""])[0] if p.returncode == 0 else None
    except (OSError, subprocess.TimeoutExpired):
        return None


def md_rows(path: Path) -> list[list[str]]:
    if not path.exists():
        return []
    rows = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.startswith("|") or line.startswith("|---"):
            continue
        vals = [x.strip() for x in line.strip().strip("|").split("|")]
        if vals and vals[0] not in ("名前", ""):
            rows.append(vals)
    return rows


def parse_case(work: Path, name: str) -> dict:
    time_rows = [row for row in md_rows(work / "time.md") if row[0] == name]
    rss_rows = [row for row in md_rows(work / "rss.md") if row[0] == name]
    check_rows = [row for row in md_rows(work / "checks.md") if row[0] == name]
    if len(time_rows) != 1 or len(time_rows[0]) != 8:
        raise ValueError("missing or malformed timing row")
    if len(rss_rows) != 1 or len(rss_rows[0]) != 5:
        raise ValueError("missing or malformed memory row")
    timings = {}
    for key, cell in zip(VARIANTS, time_rows[0][1:]):
        m = CELL.fullmatch(cell)
        if not m:
            raise ValueError("malformed timing cell: " + cell)
        median, smallest, largest = (float(v) for v in m.groups())
        timings[key] = {"min_s": smallest, "median_s": median, "max_s": largest}
    rss = {name: float(value) for name, value in
           zip(("jabberwock", "go", "cpp", "rust"), rss_rows[0][1:])}
    checks = None
    if len(check_rows) == 1 and len(check_rows[0]) == 7:
        checks = dict(zip(("sites_generation", "sites_elided", "sites_index",
                           "executed_generation", "executed_elided", "executed_index"),
                          [int(x) for x in check_rows[0][1:]]))
    return {"timings": timings, "peak_rss_mb": rss, "checks": checks,
            "ratios": {
                "jw_vs_cpp": round(timings["jabberwock"]["median_s"] / timings["cpp"]["median_s"], 4),
                "jw_vs_rust": round(timings["jabberwock"]["median_s"] / timings["rust"]["median_s"], 4)
            }}


def fingerprint() -> dict:
    return {"platform": platform.platform(), "machine": platform.machine(),
            "processor": platform.processor(),
            "clang": version(["clang", "--version"]),
            "clang++": version(["clang++", "--version"]),
            "rustc": version(["rustc", "--version"]),
            "go": version(["go", "version"])}


def compare(current: dict, baseline: dict, threshold: float) -> dict:
    if current["environment"] != baseline.get("environment") or current["runs"] != baseline.get("runs"):
        return {"status": "NOT_COMPARABLE", "reason": "platform/compiler fingerprint or repeats differ"}
    diffs = []
    for name, item in current["cases"].items():
        before = baseline.get("cases", {}).get(name, {})
        if item.get("status") != "PASS" or before.get("status") != "PASS":
            continue
        old = before["metrics"]["timings"]["jabberwock"]["median_s"]
        new = item["metrics"]["timings"]["jabberwock"]["median_s"]
        if old <= 0:
            continue
        pct = (new / old - 1) * 100
        diffs.append({"case": name, "old_s": old, "new_s": new,
                      "change_percent": round(pct, 2),
                      "regression": pct > threshold})
    return {"status": "COMPARED" if diffs else "NO_MATCHING_CASES",
            "regression_threshold_percent": threshold, "cases": diffs,
            "regressions": [r["case"] for r in diffs if r["regression"]]}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--full", action="store_true", help="all available benchmarks instead of quick sample")
    parser.add_argument("--cases", nargs="+", help="specific benchmark names, e.g. --cases sort fib")
    parser.add_argument("--runs", type=int, default=5, help="repeated measurements; default 5")
    parser.add_argument("--output", type=Path, default=ROOT / "target/benchmark-report.json")
    parser.add_argument("--baseline", type=Path, help="previous benchmark-report.json to compare")
    parser.add_argument("--threshold", type=float, default=15.0, help="regression threshold percent")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()
    if args.runs < 1 or args.threshold < 0:
        parser.error("--runs must be >= 1; --threshold must be >= 0")
    names = sorted(p.stem for p in (BENCH / "jabberwock").glob("*.jw"))
    selected = list(args.cases) if args.cases else names if args.full else list(QUICK)
    if not selected or len(set(selected)) != len(selected) or set(selected) - set(names):
        parser.error("invalid or duplicate benchmark names; available: " + ", ".join(names))
    if args.dry_run:
        print("Cases:", ", ".join(selected), "| runs:", args.runs)
        print("Output:", args.output)
        return 0

    output = args.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    work = output.parent / (output.stem + "-work")
    work.mkdir(parents=True, exist_ok=True)
    env_info = fingerprint()
    git = version(["git", "rev-parse", "HEAD"])
    report = {"schema_version": 1, "started_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
              "commit": git, "environment": env_info, "runs": args.runs,
              "suite": selected, "comparison_notes": (
                  "Jabberwock build may use LTO; C++ -O2, Rust opt-level=3, Go default. "
                  "Algorithms and library behavior differ; unchecked is UNSAFE. "
                  "Relative performance does not prove language-wide superiority."),
              "cases": {}, "baseline_comparison": None}

    missing = [tool for tool in ("cargo", "cc", "go", "clang++", "rustc")
               if not shutil.which(tool)]
    jw = ROOT / "target/release" / ("jabberwock.exe" if os.name == "nt" else "jabberwock")
    # Explicitly report unsupported measurement rather than presenting zero timings.
    infrastructure = []
    if not sys.platform.startswith("linux"):
        infrastructure.append("bench/measure.c uses Linux wait4; this benchmark suite requires Linux")
    if missing:
        infrastructure.append("missing executables: " + ", ".join(missing))
    if infrastructure:
        for name in selected:
            report["cases"][name] = {"status": "NOT_RUN", "reason": "; ".join(infrastructure)}
    else:
        start = time.monotonic()
        built = subprocess.run(["cargo", "build", "--release"], cwd=ROOT, capture_output=True,
                               text=True, encoding="utf-8", errors="replace", timeout=1200)
        report["build"] = {"exit_code": built.returncode, "seconds": round(time.monotonic()-start, 2),
                           "stderr": built.stderr[-EXCERPT:]}
        if built.returncode or not jw.is_file():
            for name in selected:
                report["cases"][name] = {"status": "INFRA_ERROR", "reason": "release compiler build failed"}
        else:
            for i, name in enumerate(selected, 1):
                # Isolated directory prevents stale Markdown tables from previous cases.
                directory = work / name
                directory.mkdir(exist_ok=True)
                runenv = dict(os.environ, BENCH_OUT=str(directory), RUNS=str(args.runs), JW=str(jw))
                start = time.monotonic()
                try:
                    run = subprocess.run(["sh", "bench/run.sh", name], cwd=ROOT, env=runenv,
                                         capture_output=True, text=True, encoding="utf-8",
                                         errors="replace", timeout=3600)
                    item = {"status": "PASS", "exit_code": run.returncode,
                            "seconds": round(time.monotonic() - start, 2),
                            "stdout_tail": run.stdout[-EXCERPT:],
                            "stderr_tail": run.stderr[-EXCERPT:]}
                    if run.returncode != 0:
                        item["status"] = "FAIL"
                    elif "!!" in run.stderr:
                        # Original run.sh prints a warning, but doesn't fail mismatches.
                        item["status"] = "OUTPUT_MISMATCH"
                    if run.returncode == 0:
                        try:
                            item["metrics"] = parse_case(directory, name)
                        except (ValueError, OSError) as error:
                            item["status"], item["reason"] = "INFRA_ERROR", str(error)
                except subprocess.TimeoutExpired as error:
                    item = {"status": "TIMEOUT", "reason": "one benchmark exceeded 3600s",
                            "seconds": round(time.monotonic() - start, 2)}
                except OSError as error:
                    item = {"status": "INFRA_ERROR", "reason": str(error)}
                report["cases"][name] = item
                print(f"[{i}/{len(selected)}] {name}: {item['status']}", flush=True)
    if args.baseline:
        try:
            base = json.loads(args.baseline.read_text(encoding="utf-8"))
            report["baseline_comparison"] = compare(report, base, args.threshold)
        except (OSError, ValueError, KeyError, TypeError) as error:
            report["baseline_comparison"] = {"status": "INVALID_BASELINE", "reason": str(error)}
    report["finished_utc"] = dt.datetime.now(dt.timezone.utc).isoformat()
    output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print("Report:", output)
    bad = any(x["status"] != "PASS" for x in report["cases"].values())
    comparison = report["baseline_comparison"]
    if comparison and comparison["status"] != "COMPARED":
        bad = True
    if comparison and comparison.get("regressions"):
        bad = True
    return int(bad)


if __name__ == "__main__":
    sys.exit(main())
