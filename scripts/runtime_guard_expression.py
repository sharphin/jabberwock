#!/usr/bin/env python3
"""Independently inspect the narrow C generation-capacity expression.

This is an exact expression-language check, NOT a verified C frontend,
whole-runtime refinement proof, or compiler semantic certification.
"""
from __future__ import annotations

import re
from pathlib import Path

# A deliberately small expression grammar: identifiers, parentheses, &, -,
# !=, <=, and &&. Reject any syntax outside that fragment.
TOKEN = re.compile(r"\s*(GEN_RETIRED|GEN_LIMIT|GEN_COUNTER_MASK|gen|step|&&|!=|<=|[()&-])")
EXPECTED = (
    "&&",
    ("&&", ("!=", "gen", "GEN_RETIRED"),
     ("<=", "step", "GEN_LIMIT")),
    ("<=", ("&", "gen", "GEN_COUNTER_MASK"),
     ("-", "GEN_LIMIT", "step")),
)
PRECEDENCE = {"&&": 1, "!=": 2, "<=": 2, "&": 3, "-": 4}
NAMES = {"gen", "step", "GEN_RETIRED", "GEN_LIMIT", "GEN_COUNTER_MASK"}


def expression_tokens(expression: str) -> list[str]:
    tokens = []
    offset = 0
    while offset < len(expression):
        if not expression[offset:].strip():
            break
        match = TOKEN.match(expression, offset)
        if not match or match.end() == offset:
            raise ValueError("unsupported C expression syntax")
        tokens.append(match.group(1))
        offset = match.end()
    return tokens


def parse_expression(tokens: list[str]):
    position = 0

    def primary():
        nonlocal position
        if position >= len(tokens):
            raise ValueError("incomplete C expression")
        token = tokens[position]
        position += 1
        if token == "(":
            expression = binary(1)
            if position >= len(tokens) or tokens[position] != ")":
                raise ValueError("unclosed C grouping")
            position += 1
            return expression
        if token not in NAMES:
            raise ValueError("unknown identifier in generation guard")
        return token

    def binary(minimum: int):
        nonlocal position
        left = primary()
        while position < len(tokens):
            operator = tokens[position]
            priority = PRECEDENCE.get(operator, 0)
            if priority < minimum:
                break
            position += 1
            right = binary(priority + 1)
            left = (operator, left, right)
        return left

    result = binary(1)
    if position != len(tokens):
        raise ValueError("trailing or unsupported C guard tokens")
    return result



# Whitelisted Lean AST surface for the already kernel-audited BitVec guard.
# The matching is exact modulo whitespace. An arbitrary change to the Lean
# executable guard must not be accepted merely because old theorem names remain.
LEAN_GUARD = (
    "decide (gen ≠ retiredWord) && step.ule limit && "
    "(gen &&& counterMask).ule (limit - step)"
)
LEAN_SPEC = (
    "gen ≠ retiredWord ∧ step ≤ limit ∧ "
    "(gen &&& counterMask) ≤ limit - step"
)


def lean_definition(source: str, declaration: str, arguments: str, result: str) -> str:
    # Exclude the next top-level declaration: do not search for a familiar
    # snippet elsewhere in the file or inside a later theorem.
    pattern = (
        r"(?m)^def\s+" + re.escape(declaration) +
        r"\s*" + arguments + r"\s*:\s*" + result +
        r"\s*:=\s*(.*?)(?=^(?:def|theorem|abbrev|end)\b|\Z)"
    )
    matches = re.findall(pattern, source, re.DOTALL | re.MULTILINE)
    if len(matches) != 1:
        raise ValueError("missing or ambiguous Lean declaration: " + declaration)
    # Ignore Lean comments only if absent, to keep this subset auditable.
    body = matches[0].strip()
    if "--" in body or "/-" in body:
        raise ValueError("comments within reviewed Lean guard are unsupported")
    return " ".join(body.split())


def inspect_lean(lean: Path) -> dict:
    if lean.is_symlink() or not lean.is_file():
        raise ValueError("Lean BitVec source missing or symlinked")
    contents = lean.read_text(encoding="utf-8")
    parameters = r"\(\s*gen\s+step\s+limit\s*:\s*Word\s*\)"
    actual_guard = lean_definition(contents, "guard", parameters, "Bool")
    actual_spec = lean_definition(contents, "guardSpec", parameters, "Prop")
    if actual_guard != LEAN_GUARD or actual_spec != LEAN_SPEC:
        raise ValueError("Lean executable guard or specification changed from reviewed expression")
    return {"status": "LEAN_EXPRESSION_ALIGNED_MODEL_ONLY",
            "scope": "exact BitVec guard and guardSpec expression bodies",
            "implementation_refinement": "NOT_PROVEN"}

# An executable reference semantics for the *restricted* C expression tree.
# This is not a substitute for a validated C frontend or a kernel proof.
UINT64_MASK = (1 << 64) - 1
COUNTER_MASK = (1 << 62) - 1


def eval_unsigned_guard(ast, gen: int, step: int, limit: int) -> bool:
    """Interpret reviewed operators with uint64_t arithmetic and C && semantics."""
    if any(type(word) is not int or not 0 <= word <= UINT64_MASK
           for word in (gen, step, limit)):
        raise ValueError("generation words must be unsigned 64-bit integers")
    symbols = {
        "gen": gen, "step": step, "GEN_RETIRED": UINT64_MASK,
        "GEN_COUNTER_MASK": COUNTER_MASK, "GEN_LIMIT": limit
    }

    def evaluate(node):
        if isinstance(node, str):
            return symbols[node]
        operator, lhs, rhs = node
        if operator == "&&":
            # C short circuits the subtraction when step > limit.
            return int(bool(evaluate(lhs)) and bool(evaluate(rhs)))
        left, right = evaluate(lhs), evaluate(rhs)
        if operator == "!=":
            return int(left != right)
        if operator == "<=":
            return int(left <= right)
        if operator == "&":
            return left & right
        if operator == "-":
            return (left - right) & UINT64_MASK
        raise ValueError("unsupported C guard operator")

    return bool(evaluate(ast))


def bitvec_guard_spec(gen: int, step: int, limit: int) -> bool:
    """Direct 64-bit interpretation of the reviewed Lean guardSpec formula."""
    if any(type(word) is not int or not 0 <= word <= UINT64_MASK
           for word in (gen, step, limit)):
        raise ValueError("generation words must be unsigned 64-bit integers")
    return (gen != UINT64_MASK and step <= limit
            and (gen & COUNTER_MASK) <= ((limit - step) & UINT64_MASK))


def inspect(source: Path) -> dict:
    if source.is_symlink() or not source.is_file():
        raise ValueError("runtime guard source unavailable or symlinked")
    text = source.read_text(encoding="utf-8")
    # Match one complete function body; no fallback to an arbitrary occurrence
    # of the expected expression elsewhere in the source.
    functions = re.findall(
        r"static\s+int\s+gen_has_room\s*\(\s*uint64_t\s+gen\s*,\s*"
        r"uint64_t\s+step\s*\)\s*\{([^{}]*)\}", text,
        flags=re.DOTALL,
    )
    if len(functions) != 1:
        raise ValueError("missing or ambiguous generation guard function")
    body = functions[0].strip()
    returns = re.fullmatch(r"return\s+([^;]+)\s*;", body, flags=re.DOTALL)
    if not returns:
        raise ValueError("generation guard contains unsupported C statements")
    parsed = parse_expression(expression_tokens(returns.group(1)))
    if parsed != EXPECTED:
        raise ValueError("generation guard differs from the reviewed BitVec guard expression")
    # The two interpreters remain separate representations; expression AST
    # alignment is necessary, not sufficient, for universal semantic proof.
    return {
        "status": "EXPRESSION_ALIGNED_MODEL_ONLY",
        "source": source.as_posix(),
        "scope": "gen_has_room expression grammar and operator grouping",
        "implementation_refinement": "NOT_PROVEN",
        "limitations": [
            "C preprocessing, integer conversion, C compiler semantics and macro expansion are not verified.",
            "Alignment with the Lean BitVec guard is expression-level only.",
            "Pointer lifetimes, atomics, allocation, threading and external behavior remain unproved.",
        ],
    }
