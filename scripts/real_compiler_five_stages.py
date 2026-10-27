#!/usr/bin/env python3
"""Run *actual* Rust and self-hosted jwc compiler binaries at five boundaries.

Uses existing public CLI modes, never an invented IR interpreter.
Stages: execution, tokens/AST, typed AST, pre-analysis IR, post-analysis IR.
Passing finite differential cases does not prove universal semantic correctness.
"""
from __future__ import annotations

import argparse
import difflib
import hashlib
import json
from pathlib import Path
import subprocess
import sys

STAGES = {
    "source_semantics": (),
    "parse": ("--ast",),
    "typecheck": ("--typed",),
    "lower": ("--ir-raw",),
    "ir_semantics": ("--ir",),
}
# Independently assert that both CLI implementations advertise these modes.
IMPLEMENTATIONS = {"rust": "src/main.rs", "jwc": "jwc/main.jw"}


def check_cli_sources(root: Path) -> dict:
    result = {}
    for name, filename in IMPLEMENTATIONS.items():
        file = root / filename
        source = file.read_text(encoding="utf-8")
        for flag in ("--ast", "--typed", "--ir-raw", "--ir"):
            if flag not in source:
                raise ValueError(name + " lacks required CLI boundary: " + flag)
        result[name] = {"path": filename, "sha256": hashlib.sha256(file.read_bytes()).hexdigest()}
    return result


def run_boundary(binary: Path, stage: str, source: Path, *, root: Path, timeout: float):
    flags = STAGES[stage]
    args = [str(binary), *flags, str(source)]
    try:
        proc = subprocess.run(args, cwd=root, input="", capture_output=True,
                              text=True, encoding="utf-8", errors="replace",
                              timeout=timeout, check=False)
        return {"status": "EXECUTED", "exit": proc.returncode,
                "stdout": proc.stdout, "stderr": proc.stderr}
    except (OSError, subprocess.TimeoutExpired) as exc:
        return {"status": "ERROR", "reason": str(exc)}


def verify(root: Path, binaries: dict[str, Path], sources: list[Path],
           timeout: float = 30) -> dict:
    binding = check_cli_sources(root)
    if not sources:
        raise ValueError("no source fixtures specified")
    for name in IMPLEMENTATIONS:
        if name not in binaries or not binaries[name].is_file():
            raise ValueError("missing real executable for " + name)
    records = []
    for source in sources:
        full = source.resolve()
        if not full.is_file():
            raise ValueError("missing test input: " + str(source))
        for stage in STAGES:
            observed = {name: run_boundary(binaries[name].resolve(), stage, full,
                                            root=root, timeout=timeout)
                        for name in IMPLEMENTATIONS}
            left, right = observed["rust"], observed["jwc"]
            if any(x["status"] != "EXECUTED" for x in observed.values()):
                status = "ERROR"
                diff = ""
            else:
                status = ("MATCH" if (left["exit"], left["stdout"]) ==
                          (right["exit"], right["stdout"]) else "MISMATCH")
                diff = "" if status == "MATCH" else "".join(difflib.unified_diff(
                    left["stdout"].splitlines(True), right["stdout"].splitlines(True),
                    fromfile="rust", tofile="jwc"))[:8000]
            records.append({"source": str(full), "source_sha256": hashlib.sha256(
                full.read_bytes()).hexdigest(), "stage": stage,
                "status": status, "observations": observed, "stdout_diff": diff})
    return {"schema_version": 1, "kind": "real-compiler-five-boundary-comparison",
            "status": "MATCH" if all(x["status"] == "MATCH" for x in records) else "FAIL",
            "source_binding": binding, "results": records,
            "matched": sum(x["status"] == "MATCH" for x in records),
            "total": len(records),
            "scope": "observational comparison of two real compilers on supplied fixtures; "
                     "not whole-language proof"}


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rust-bin", required=True, type=Path)
    parser.add_argument("--jwc-bin", required=True, type=Path)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--timeout", type=float, default=30)
    parser.add_argument("--output", type=Path)
    parser.add_argument("sources", nargs="+", type=Path)
    args = parser.parse_args(argv)
    try:
        if args.timeout <= 0:
            raise ValueError("timeout must be positive")
        result = verify(args.root.resolve(),
                        {"rust": args.rust_bin, "jwc": args.jwc_bin},
                        args.sources, args.timeout)
    except (OSError, ValueError) as exc:
        result = {"schema_version": 1, "status": "INVALID", "reason": str(exc)}
    rendered = json.dumps(result, indent=2, ensure_ascii=False) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    print(rendered, end="")
    return 0 if result["status"] == "MATCH" else 1


if __name__ == "__main__":
    sys.exit(main())
