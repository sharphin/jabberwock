#!/usr/bin/env python3
"""Separate formal-proof report; abstract-model success is not implementation proof."""
import argparse
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
NAMES = ("意味保存", "メモリ安全性", "型安全性", "所有権・参照の整合性", "実行時チェックの正しさ")

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "target/formal-report.json")
    args = parser.parse_args()
    report = {"schema_version": 1, "kind": "formal-proof", "goals": [
        {"name": name, "compiler": compiler, "status": "NOT_PROVEN",
         "reason": "実装全体との対応を示す証明は未実装。抽象モデルの定理で代用しない。"}
        for compiler in ("jwc", "rust") for name in NAMES],
        "abstract_model": {"status": "NOT_RUN"}}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    def save():
        args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    save()
    try:
        # Never run practical probes here, and never invoke this from safety_report.
        result = subprocess.run([sys.executable, str(ROOT / "scripts/check_formal.py")], cwd=ROOT,
                                capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=1800)
        report["abstract_model"] = {"status": "MODEL_PASS" if result.returncode == 0 else "FAIL",
                                    "exit_code": result.returncode, "stdout": result.stdout, "stderr": result.stderr}
    except (OSError, subprocess.TimeoutExpired) as error:
        report["abstract_model"] = {"status": "INFRA_ERROR", "error": str(error)}
    except KeyboardInterrupt:
        report["abstract_model"] = {"status": "INTERRUPTED"}
    save()
    print("Formal report: " + str(args.output))
    # Full requested proof goals remain incomplete even if the bounded model builds.
    return 2 if report["abstract_model"]["status"] == "INFRA_ERROR" else 1

if __name__ == "__main__":
    sys.exit(main())
