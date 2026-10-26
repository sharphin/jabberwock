#!/usr/bin/env python3
"""Local-only independent practical and mathematical proof audit.

Always fail closed: a passed subprocess without matching, complete report evidence
is not certification. Input case catalogs may grow without changing the 12+5 goals.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import platform
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
COMPILERS = ("jwc", "rust")
GOALS = ("semantics", "memory", "types", "ownership", "checks")


def judge(practical: dict, formal: dict, compilers=COMPILERS) -> dict:
    expected = set(compilers)
    expected_items = {(c, n) for c in expected for n in range(1, 13)}
    expected_goals = {(c, g) for c in expected for g in GOALS}
    items = practical.get("items")
    proofs = formal.get("goals")
    if not isinstance(items, list) or not isinstance(proofs, list):
        return {"status": "INVALID_EVIDENCE", "reason": "item or proof list missing"}
    try:
        item_keys = [(x["compiler"], x["item"]) for x in items]
        goal_keys = [(x["compiler"], x["goal"]) for x in proofs]
    except (KeyError, TypeError):
        return {"status": "INVALID_EVIDENCE", "reason": "item or proof identity missing"}
    if (len(item_keys) != len(expected_items) or set(item_keys) != expected_items or
            len(goal_keys) != len(expected_goals) or set(goal_keys) != expected_goals):
        return {"status": "INVALID_EVIDENCE", "reason": "missing, duplicate or alien goal/item"}
    if practical.get("run_status") != "COMPLETE" or formal.get("infrastructure_error") or formal.get("interrupted"):
        return {"status": "NOT_PROVEN", "reason": "incomplete run or proof infrastructure problem"}
    if practical.get("suite", {}).get("compilers") not in (list(compilers), tuple(compilers)):
        return {"status": "INVALID_EVIDENCE", "reason": "compiler provenance mismatch"}
    if set(formal.get("compilers", {})) != expected:
        return {"status": "INVALID_EVIDENCE", "reason": "formal compiler provenance mismatch"}
    practical_ok = all(x.get("acceptance") == "PASS" for x in items)
    formal_ok = all(x.get("acceptance") == "PROVEN" and
                    x.get("proof_check", {}).get("status") == "KERNEL_CHECKED" for x in proofs)
    return {"status": "PASS" if practical_ok and formal_ok else "NOT_PROVEN",
            "practical_accepted": sum(x.get("acceptance") == "PASS" for x in items),
            "formal_accepted": sum(x.get("acceptance") == "PROVEN" and
                                   x.get("proof_check", {}).get("status") == "KERNEL_CHECKED" for x in proofs),
            "required_practical": len(expected_items),
            "required_formal": len(expected_goals),
            "note": "Local report audit only. Acceptance predicates must be backed by independently checked evidence."}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "target/local-verification.json")
    parser.add_argument("--practical", type=Path, help="existing local safety report (otherwise execute suite)")
    parser.add_argument("--formal", type=Path, help="existing local formal report (otherwise execute Lean audit)")
    args = parser.parse_args()
    if platform.system() not in ("Windows", "Linux"):
        parser.error("only Linux and Windows safety suites exist")
    os_name = platform.system().lower()
    practical_path = args.practical or ROOT / "target" / ("safety-" + os_name + "-report.json")
    formal_path = args.formal or ROOT / "target/formal-report.json"
    executed = []
    for supplied, label, command in (
        (args.practical, "practical", [sys.executable, str(ROOT / "scripts" / ("safety_" + os_name + ".py")),
                                      "--compiler", "all", "--output", str(practical_path)]),
        (args.formal, "formal", [sys.executable, str(ROOT / "scripts/formal_report.py"),
                                "--compiler", "all", "--require-full", "--output", str(formal_path)])):
        if supplied is None:
            try:
                result = subprocess.run(command, cwd=ROOT, timeout=86400)
                executed.append({"stage": label, "exit_code": result.returncode})
            except (OSError, subprocess.TimeoutExpired) as exc:
                executed.append({"stage": label, "error": str(exc)})
    def read(path):
        try:
            return json.loads(path.read_text(encoding="utf-8"))
        except (OSError, ValueError):
            return {}
    practical = read(practical_path)
    formal = read(formal_path)
    verdict = judge(practical, formal)
    if any("error" in x for x in executed):
        verdict = {"status": "INFRA_ERROR", "reason": "local suite could not be executed"}
    report = {"schema_version": 1, "kind": "local-verification", "verdict": verdict,
              "runs": executed, "practical_report": str(practical_path), "formal_report": str(formal_path)}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(verdict, ensure_ascii=False, indent=2))
    print("Report:", args.output)
    return 0 if verdict["status"] == "PASS" and all(r.get("exit_code") == 0 for r in executed) else 1


if __name__ == "__main__":
    sys.exit(main())
