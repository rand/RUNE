from __future__ import annotations
from dataclasses import dataclass, field
from typing import Any

@dataclass
class IR:
    version: str
    inputs: dict[str, Any]
    facts: list[tuple[str, str, Any]]
    derived: dict[str, Any]
    explain: dict[str, list[str]] = field(default_factory=dict)

    def to_json_obj(self) -> dict:
        return {
            "$version": self.version,
            "inputs": self.inputs,
            "facts": [list(x) for x in self.facts],
            "derived": self.derived,
            "explain": self.explain,
        }
