#!/usr/bin/env python3
"""Validate repository-owned refinement propositions with Lean's kernel.

A matching proposition is necessary but not sufficient to prove that a real
compiler implements that proposition. Never promote compiler proof status here.
"""
from __future__ import annotations
import json
import re
import subprocess
import tempfile
from pathlib import Path

NAME = re.compile(r"^[A-Za-z_][A-Za-z0-9_.]*$")

class ContractError(ValueError):
    pass

def load_contracts(root: Path, requested: list[str] | None) -> list[dict]:
    path = root / "formal" / "refinement-contracts.json"
    if not path.is_file() or path.is_symlink():
        return []
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict) or data.get("schema_version") != 1:
        raise ContractError("unsupported contract schema")
    entries = data.get("contracts")
    if not isinstance(entries, list):
        raise ContractError("invalid contracts list")
    seen = set()
    selected = []
    for item in entries:
        if not isinstance(item, dict) or set(item) != {"theorem", "expected_type"}:
            raise ContractError("malformed refinement contract")
        name, expected = item["theorem"], item["expected_type"]
        if not isinstance(name, str) or not NAME.fullmatch(name) or name in seen:
            raise ContractError("duplicate/invalid theorem name")
        seen.add(name)
        # Prevent additional commands from being injected into generated Lean.
        if (not isinstance(expected, str) or not expected.strip() or
            len(expected) > 8192 or any(ch in expected for ch in ("\n", "\r", ";")) or
            "--" in expected or "/-" in expected or "-/" in expected or
            "#" in expected):
            raise ContractError("unsafe refinement proposition")
        if requested is None or name in requested:
            selected.append(item)
    return selected

def check_contracts(root: Path, lake: str, names: list[str], timeout: int) -> list[str]:
    contracts = load_contracts(root, names)
    if not contracts:
        return []
    # Each statement is checked by Lean itself; no claim from a JSON report is
    # accepted as a substitute for elaboration and kernel type checking.
    commands = ["import JabberwockFormal"]
    for item in contracts:
        commands.append("example : (" + item["expected_type"] + ") := by exact " + item["theorem"])
    with tempfile.TemporaryDirectory(prefix="jw-refinement-contract-") as temp:
        file = Path(temp) / "RefinementContracts.lean"
        file.write_text("\n".join(commands) + "\n", encoding="utf-8")
        process = subprocess.run([lake, "env", "lean", str(file)],
                                 cwd=root / "formal", capture_output=True,
                                 text=True, timeout=timeout, check=False)
        if process.returncode:
            raise ContractError("Lean rejected expected proposition: " +
                                (process.stderr or process.stdout)[-1000:])
    return [item["theorem"] for item in contracts]
