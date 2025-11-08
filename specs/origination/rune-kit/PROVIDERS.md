# Providers and payload projections

RUNE treats provider wire details as targets that are easy to swap and version.
Keep these rules in mind during implementation.

- Do not hardcode model names. Keep them in config data or overlays.
- Validate tool schemas before emit. Fail fast with a clear message that points to the tool.
- Provide an option to print a dry run payload with comments removed and keys sorted.
- Provide a strict JSON guard utility for post processing LLM outputs that should be JSON.

Each emitter module must export a `project(ir, config)` function that returns native Python dict payloads.
