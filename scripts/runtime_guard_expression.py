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
