from __future__ import annotations
from typing import Any, Dict

def project(ir: Dict[str, Any], config: Dict[str, Any]) -> Dict[str, Any]:
    provider = ir["derived"]["provider"].get("claude", {})
    tools = []
    tool_defs = config.get("tools", {})
    for name, t in tool_defs.items():
        if "input_schema" in t:
            tools.append({
                "name": name,
                "description": t.get("description", name),
                "input_schema": t["input_schema"],
            })
    return {
        "model": provider.get("model", config.get("providers", {}).get("claude", {}).get("model", "claude-3.7")),
        "temperature": provider.get("temperature", 0.2),
        "tools": tools,
    }
