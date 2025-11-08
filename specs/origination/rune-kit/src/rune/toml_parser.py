from __future__ import annotations
from typing import Any, Dict
import tomli

def parse_data_section(text: str) -> Dict[str, Any]:
    # Parse the entire file as TOML and return the raw dict.
    # The rules block will be handled by rules_parser.
    # For now we keep it simple and let the rules parser strip itself.
    return tomli.loads(text)
