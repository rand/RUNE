# Implementation plan

1. Parser and typing
   - Implement TOML to typed store with dotted paths canonicalized
   - Define value kinds and structural object schemas

2. Rules AST and stratification
   - Implement a minimal rules parser with an operator precedence table
   - Build the predicate dependency graph and compute strata
   - Reject programs with cycles through negation

3. Engine
   - Bottom up semi naive evaluator
   - Builtins: comparisons, arithmetic, regex, time ops
   - `set(Path, Value)` accumulation with conflict detection and optional rule priorities

4. IR and explain
   - Materialize derived map into JSON IR
   - Keep a compact trace per derived path

5. Emitters
   - OpenAI function calling projection
   - Anthropic tools projection
   - Gemini functionDeclarations

6. CLI and tests
   - `rune check`, `rune derive`, `rune test`, `rune explain`
   - Golden tests under `examples` and `tests`
