from __future__ import annotations
from dataclasses import dataclass, field
from typing import Any, Dict, Tuple, Iterable
from .ir import IR

@dataclass
class EvalContext:
    inputs: Dict[str, Any]
    facts: list[Tuple[str,str,Any]] = field(default_factory=list)
    derived: Dict[str, Any] = field(default_factory=dict)
    explain: Dict[str, list[str]] = field(default_factory=dict)

def seed_facts(inputs: Dict[str, Any]) -> list[Tuple[str,str,Any]]:
    out = []
    def walk(prefix, v):
        if isinstance(v, dict):
            for k, vv in v.items():
                walk(f"{prefix}.{k}" if prefix else k, vv)
        else:
            out.append(("val", prefix, v))
    walk("", inputs)
    return out

def evaluate(ctx: EvalContext) -> IR:
    # TODO real evaluation of rules and assignments
    facts = seed_facts(ctx.inputs)
    ctx.facts = facts
    # For now pass through inputs as derived provider map so the CLI can run
    derived = {"provider": ctx.inputs.get("providers", {})}
    return IR(version="rune/0.3", inputs=ctx.inputs, facts=facts, derived=derived, explain={})
