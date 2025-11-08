from __future__ import annotations
from dataclasses import dataclass
from typing import Any

# Minimal AST placeholders. The agent will complete these.
@dataclass
class Predicate:
    name: str
    args: list[Any]

@dataclass
class Rule:
    head: Predicate
    body: list[list[Any]]  # list of clauses, each clause is list of literals or guards
    text: str
    line: int

@dataclass
class Program:
    facts: list[Predicate]
    rules: list[Rule]

def extract_rules_block(text: str) -> str:
    # Very crude block extraction. Replace with a proper parser.
    lines = text.splitlines()
    out = []
    in_rules = False
    for ln in lines:
        if ln.strip().startswith("[rules]"):
            in_rules = True
            continue
        if in_rules and ln.strip().startswith("[") and ln.strip() != "[rules]":
            break
        if in_rules:
            out.append(ln)
    return "\n".join(out)

def parse_program(text: str) -> Program:
    # TODO implement real parsing. For now return an empty program.
    return Program(facts=[], rules=[])
