# Architecture

## Modules

- `rune.typing` – value kinds, structural types, validation
- `rune.errors` – rich exceptions and blame data
- `rune.ir` – normalized JSON IR types and encoders
- `rune.toml_parser` – data section parser to a typed store
- `rune.rules_parser` – rules parser to an AST
- `rune.engine` – semi naive stratified evaluator, conflict lattice, explain traces
- `rune.emitters` – provider projections
  - `openai`
  - `anthropic`
  - `gemini`
- `rune.cli` – Typer based CLI: check, derive, test, explain

## Data flow

File -> parse data -> type check -> seed facts -> parse rules -> stratify -> evaluate -> set map -> IR -> emit

## Decisions

- Python first for speed of delivery and readability. Rust or Zig backends can replace the engine later.
- Datalog over full Prolog for termination and clear conflict handling.
- TOML oriented data to avoid YAML implicit typing pitfalls.
