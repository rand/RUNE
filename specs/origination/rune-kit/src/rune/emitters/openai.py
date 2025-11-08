from __future__ import annotations
from typing import Any, Dict

def project(ir: Dict[str, Any], config: Dict[str, Any]) -> Dict[str, Any]:
    # Expect ir to already contain derived provider map
    provider = ir["derived"]["provider"].get("openai", {})
    tools = []
    tool_defs = config.get("tools", {})
    for name, t in tool_defs.items():
        if "input_schema" in t:
            tools.append({
                "type": "function",
                "function": {
                    "name": name,
                    "description": t.get("description", name),
                    "parameters": t["input_schema"],
                },
            })
    return {
        "model": provider.get("model", config.get("providers", {}).get("openai", {}).get("model", "gpt-4o")),
        "temperature": provider.get("temperature", 0.2),
        "tools": tools,
    }
