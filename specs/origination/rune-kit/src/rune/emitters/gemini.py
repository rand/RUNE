from __future__ import annotations
from typing import Any, Dict

def project(ir: Dict[str, Any], config: Dict[str, Any]) -> Dict[str, Any]:
    provider = ir["derived"]["provider"].get("gemini", {})
    fns = []
    tool_defs = config.get("tools", {})
    for name, t in tool_defs.items():
        if "input_schema" in t:
            fns.append({
                "name": name,
                "description": t.get("description", name),
                "parameters": t["input_schema"],
            })
    return {
        "model": provider.get("model", config.get("providers", {}).get("gemini", {}).get("model", "gemini-pro")),
        "generationConfig": {"temperature": provider.get("temperature", 0.2)},
        "tools": [{"functionDeclarations": fns}] if fns else [],
    }
