You are a senior coding agent. Implement the RUNE system from this repository.
Follow the steps in order. Confirm each checkpoint by running tests and small demos.

Objectives
- Parse RUNE files with TOML style data and a compact rules block
- Type check values and schemas
- Compile rules to a stratified Datalog program
- Evaluate with a bottom up semi naive engine
- Accumulate `set(Path, Value)` assignments with conflict detection
- Produce a normalized JSON IR and provider payloads
- Provide a CLI with check, derive, test, explain

Constraints
- Python 3.11
- Keep external deps minimal. Allowed: tomli, lark, pydantic, typer, rich, jsonschema, pytest
- No network calls in the engine. Emitters are pure projections
- Do not hardcode provider model names. Read from config

Deliverables
- Working package importable as `rune`
- CLI command `rune`
- Passing unit tests under `tests`
- Golden snapshots under `examples`

Steps
1. Implement `rune.typing` with kind introspection and structural object schemas. Write tests.
2. Implement `rune.toml_parser` to read the data section into a typed store. Support dotted paths.
3. Implement `rune.rules_parser` using Lark or a hand rolled parser. Produce an AST. Include positions for good errors.
4. Build stratification in `rune.engine.stratify`. Reject cycles that go through negation.
5. Implement bottom up semi naive evaluation. Seed `val(Path, Value)` from data. Support standard builtins and list comprehensions.
6. Implement conflict detection for `set(Path, Value)`. Support optional `prio(rule, int)` predicate for tie breaks.
7. Build the JSON IR in `rune.ir`. Include compact explain traces keyed by derived paths.
8. Implement `rune.emitters.openai`, `anthropic`, `gemini`. Each exposes `project(ir, config)`.
9. Build the Typer CLI in `rune.cli` with subcommands: `check`, `derive`, `test`, `explain`.
10. Fill `tests/test_demo.py` until green. Expand examples as golden tests.

Acceptance
- `rune check examples/configs/basic.rune` exits 0
- `rune derive -p openai examples/configs/basic.rune` prints a valid payload
- `rune test` runs inline tests defined in the example file and passes
- `pytest -q` is green
