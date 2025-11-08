# RUNE – Declarative config plus rules for LLM providers

RUNE combines TOML or YAML style ergonomics with Datalog style facts and rules.
You write one file with data plus concise rules. RUNE derives concrete payloads for OpenAI, Anthropic Claude, and Google Gemini.
This kit gives you a ready to build reference implementation in Python and a testable CLI surface.
It also includes an agent prompt to drive a coding agent to implement the system end to end.

Status: scaffolding on 2025-11-08. The code skeleton compiles. The rule engine and parsers have stubs with TODOs for the agent to fill.

## Quick start

```bash
# Python 3.11 recommended
pipx install uv || python -m pip install --upgrade pip

# Create a virtual env and install deps
python -m venv .venv && source .venv/bin/activate
pip install -e ".[dev]"

# Run unit tests
pytest -q

# Try the CLI help
rune --help

# Derive an OpenAI payload from the example config
rune derive -p openai examples/configs/basic.rune
```

See `PROMPT.md` for an agent oriented set of instructions to implement the missing parts.
