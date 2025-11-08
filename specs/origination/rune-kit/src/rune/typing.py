from __future__ import annotations
from dataclasses import dataclass
from typing import Any, Literal

Kind = Literal["null","bool","number","string","array","object"]

def kind_of(value: Any) -> Kind:
    if value is None: return "null"
    if isinstance(value, bool): return "bool"
    if isinstance(value, (int, float)): return "number"
    if isinstance(value, str): return "string"
    if isinstance(value, list): return "array"
    if isinstance(value, dict): return "object"
    raise ValueError(f"Unknown kind for {type(value)}")

@dataclass
class Schema:
    type: Kind | None = None
    properties: dict[str, "Schema"] | None = None
    required: set[str] | None = None
    closed: bool = True

def validate(schema: Schema, value: Any, path: str = "$") -> None:
    from .errors import TypeError as RuneTypeError
    if schema.type and kind_of(value) != schema.type:
        raise RuneTypeError(f"Expected {schema.type} got {kind_of(value)} at {path}", path)
    if schema.type == "object" and schema.properties is not None:
        props = schema.properties
        req = schema.required or set()
        for r in req:
            if r not in value:
                raise RuneTypeError(f"Missing required field {r} at {path}", path)
        if schema.closed:
            for k in value.keys():
                if k not in props:
                    raise RuneTypeError(f"Unexpected field {k} at {path}", path)
        for k, sub in props.items():
            if k in value:
                validate(sub, value[k], f"{path}.{k}")
