# RUNE specification

RUNE is a single file format that merges familiar TOML style data with a compact Horn clause rules sublanguage.
The system compiles to a typed JSON IR and a stratified Datalog program. Evaluation is deterministic and testable.

## File sections

- Header: `version = "rune/0.3"`
- Data: TOML style tables and dotted paths. Values are typed scalar, array, or object.
- Rules: a `[rules]` block with facts and rules.
- Tests: optional `[tests.*]` tables with Given and Expect maps.

## Semantics

- Each data leaf seeds `val(Path, Value)` facts. Dotted paths are canonicalized to JSON Pointer like forms.
- Rules are stratified. Negation is only allowed across strata.
- Assignments use `set(Path, Value)`. Conflicts are errors unless values match or a higher priority rule wins.
- Collections can be built with comprehension shorthand `collect_enabled_tools([T*]) :- enabled(T).`

## Determinism

RUNE rejects programs with cycles through negation. Evaluation uses bottom up semi naive evaluation.
Type checking happens before evaluation. Closed by default object schemas avoid accidental widening.

## Provider emits

- OpenAI: function calling tools from JSON Schema, model and sampling parameters.
- Anthropic: tools with `input_schema` and compatible model parameters.
- Gemini: `functionDeclarations` and structured output when tools are not used.

Provider model names and minor wire details change over time. RUNE treats those as data in config overlays.
Do not hardcode model names in the engine.

## JSON IR shape

```json
{
  "$version": "rune/0.3",
  "inputs": { "task": "code", "budget": { "hour": 3.0 } },
  "facts": [ ["val", "task", "code"], ["val", "budget.hour", 3.0] ],
  "derived": {
    "provider": {
      "openai":  {"model": "gpt-4o-mini", "temperature": 0.2, "tools": ["web_search"]},
      "claude":  {"model": "claude-3.7-haiku", "temperature": 0.2, "tools": []},
      "gemini":  {"model": "gemini-2.5-flash", "temperature": 0.2, "tools": []}
    }
  },
  "explain": {
    "provider.openai.model": [
      "choose_model(openai, gpt-4o-mini) :- code_task(), low_budget().",
      "code_task() :- task = \"code\".",
      "low_budget() :- budget.hour =< 5.0."
    ]
  }
}
```

## Error model

- Type error: fail fast with the exact path and expected vs actual kinds.
- Stratification error: cycle through negation reported with the cycle path.
- Conflict error: two rules set the same path with distinct values and no priority winner.

## Grammar

See `GRAMMAR.ebnf` for the reference grammar.
