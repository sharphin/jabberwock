#!/usr/bin/env python3
"""Executable reference semantics for the core IR ownership operations.

This is a bounded *semantic oracle*, not a claim that every Jabberwock IR
instruction is modeled. It deliberately fails on unsupported operations.
Each pointer carries an allocation ID and generation; retirement invalidates
aliases even when an allocator reuses the same address.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any


class IRTrap(Exception):
    pass


@dataclass(frozen=True)
class Ref:
    allocation: int
    generation: int


@dataclass
class Cell:
    generation: int = 0
    value: Any = None
    live: bool = True


@dataclass
class Machine:
    locals: dict[int, Any] = field(default_factory=dict)
    heap: dict[int, Cell] = field(default_factory=dict)
    next_allocation: int = 1
    outputs: list[Any] = field(default_factory=list)

    def allocate(self, value: Any) -> Ref:
        identity = self.next_allocation
        self.next_allocation += 1
        self.heap[identity] = Cell(value=value)
        return Ref(identity, 0)

    def dereference(self, ref: Ref, *, checked: bool = True) -> Any:
        if not isinstance(ref, Ref):
            raise IRTrap("dereference of non-reference")
        cell = self.heap.get(ref.allocation)
        if cell is None or not cell.live:
            raise IRTrap("dangling reference")
        # The oracle never trusts elision from its input; a caller must supply
        # independently certified liveness before removing this check.
        if cell.generation != ref.generation:
            raise IRTrap("stale generation")
        return cell.value

    def retire(self, ref: Ref) -> None:
        cell = self.heap.get(ref.allocation)
        if cell is None or not cell.live or cell.generation != ref.generation:
            raise IRTrap("double retirement or invalid owner")
        cell.live = False
        cell.generation += 1
        cell.value = None

    def _local(self, number: int) -> Any:
        if number not in self.locals:
            raise IRTrap("use of moved or uninitialized local")
        return self.locals[number]

    def read(self, operand: dict) -> Any:
        kind = operand.get("kind")
        if kind == "const":
            return operand["value"]
        if kind == "copy":
            return self._local(operand["local"])
        if kind in ("move", "read"):
            number = operand["local"]
            value = self._local(number)
            del self.locals[number]
            return value
        if kind == "deref":
            return self.dereference(self.read(operand["ref"]))
        raise IRTrap("unsupported operand: " + str(kind))

    def execute(self, instruction: dict) -> Any:
        kind = instruction.get("kind")
        if kind == "assign":
            value = self.read(instruction["value"])
            number = instruction["local"]
            if number in self.locals and isinstance(self.locals[number], Ref):
                self.retire(self.locals[number])
            self.locals[number] = value
        elif kind == "own":
            self.locals[instruction["local"]] = self.allocate(
                self.read(instruction["value"]))
        elif kind == "borrow":
            value = self._local(instruction["owner"])
            if not isinstance(value, Ref):
                raise IRTrap("borrow without an owner")
            self.locals[instruction["local"]] = value
        elif kind == "drop":
            value = self.locals.pop(instruction["local"], None)
            if isinstance(value, Ref) and instruction.get("owns", False):
                self.retire(value)
        elif kind == "discard":
            value = self.read(instruction["value"])
            if instruction.get("owns", False) and isinstance(value, Ref):
                self.retire(value)
        elif kind == "show":
            self.outputs.append(self.read(instruction["value"]))
        else:
            raise IRTrap("unsupported instruction: " + str(kind))

    def run(self, instructions: list[dict]) -> dict:
        for index, instruction in enumerate(instructions):
            try:
                self.execute(instruction)
            except (IRTrap, KeyError, TypeError) as exc:
                return {"status": "TRAP", "at": index, "reason": str(exc),
                        "outputs": self.outputs}
        return {"status": "OK", "outputs": self.outputs}


def interpret(document: dict) -> dict:
    if document.get("schema_version") != 1 or not isinstance(document.get("instructions"), list):
        raise ValueError("invalid reference IR program")
    return Machine().run(document["instructions"])
