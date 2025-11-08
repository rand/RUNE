# AgentConfig Quick Reference

**One-page cheat sheet for AgentConfig syntax and concepts**

## Basic Syntax

### Agent Definition
```agentconfig
agent "my-agent" {
  model = "claude-sonnet-4-5"
  
  context {
    temperature = 0.7
    max_tokens = 100000
  }
}
```

### Facts
```agentconfig
facts {
  # Simple fact
  language("python")
  
  # Fact with attributes
  tool("pytest") {
    for_language = "python"
    type = "testing"
  }
  
  # Fact with multiple arguments
  parent("alice", "bob")
}
```

### Rules
```agentconfig
rules {
  # Basic rule: head :- body
  grandparent(X, Z) :-
    parent(X, Y),
    parent(Y, Z).
  
  # Rule with negation
  unknown_file(F) :-
    file(F),
    not analyzed(F).
  
  # Rule with attributes
  use_tool(T) :-
    current_file(F),
    tool(T) { for_language = "python" }.
  
  # Multiple conditions (AND)
  safe_to_execute(Action) :-
    action(Action),
    not destructive(Action),
    user_approved(Action).
}
```

### Constraints
```agentconfig
constraints {
  # Simple constraint
  temperature >= 0.0 && temperature <= 1.0
  
  # Forall constraint
  forall file in files:
    file_size(file) <= 1048576
  
  # Exists constraint
  exists test:
    test_coverage >= 0.8
  
  # Pattern matching
  path =~ allowed_paths
}
```

### Types
```agentconfig
types {
  # Primitive types
  Name :: string
  Age :: int & > 0 & < 150
  Score :: float & >= 0.0 & <= 1.0
  
  # Struct types
  Person :: {
    name: string,
    age: int & > 0,
    email: string =~ ".*@.*"
  }
  
  # Union types
  Status :: "active" | "inactive" | "pending"
  
  # Constrained types
  Port :: int & > 0 & < 65536
}
```

### Tests
```agentconfig
test "rule produces expected result" {
  given {
    facts {
      parent("alice", "bob")
      parent("bob", "charlie")
    }
  }
  
  when {
    query grandparent(X, Z)
  }
  
  then {
    assert exists X, Z where X == "alice" && Z == "charlie"
    assert count(X, Z) == 1
  }
}
```

### Export
```agentconfig
export target "claude-code" {
  format = "json"
  output = ".claude/config.json"
  
  transform {
    system_prompt += render_rules(rules)
  }
}
```

## Datalog Concepts

### Variables vs Constants
- **Variables**: Start with uppercase (X, Y, File, Tool)
- **Constants**: Start with lowercase or are strings/numbers

### Atoms
- Predicate with arguments: `parent(X, Y)`
- With attributes: `tool(T) { type = "testing" }`
- Negated: `not analyzed(F)`

### Rules
- Format: `head :- body1, body2, ...`
- Head: Single atom
- Body: Multiple atoms (comma = AND)

### Queries
- Find facts matching a pattern
- Variables in query get bound to values
- Example: `parent(X, "bob")` finds all X where parent(X, "bob")

## Common Patterns

### File Extension Mapping
```agentconfig
rules {
  file_language(File, Lang) :-
    file_extension(File, ".py"),
    Lang = "python".
  
  file_language(File, Lang) :-
    file_extension(File, ".rs"),
    Lang = "rust".
}
```

### Tool Selection
```agentconfig
rules {
  use_tool(Tool) :-
    current_file(File),
    file_language(File, Lang),
    tool(Tool) { for_language = Lang }.
}
```

### Security Check
```agentconfig
rules {
  deny_access(Path) :-
    forbidden_path(Pattern),
    path_matches(Path, Pattern).
  
  allow_access(Path) :-
    allowed_path(Pattern),
    path_matches(Path, Pattern),
    not deny_access(Path).
}
```

### Dependency Tracking
```agentconfig
rules {
  # Direct dependency
  depends_on(A, B) :- direct_import(A, B).
  
  # Transitive dependency (recursive)
  depends_on(A, C) :-
    depends_on(A, B),
    depends_on(B, C).
  
  # Circular dependency detection
  has_circular_dependency(A) :-
    depends_on(A, B),
    depends_on(B, A).
}
```

### Aggregation
```agentconfig
rules {
  # Count
  total_files(Count) :-
    Count = count(File : file(File)).
  
  # Sum
  total_tokens(Sum) :-
    Sum = sum(Tokens : api_call(_, Tokens)).
  
  # Average
  average_coverage(Avg) :-
    Avg = avg(Cov : test_coverage(_, Cov)).
  
  # Maximum
  max_complexity(Max) :-
    Max = max(Score : file_complexity(_, Score)).
}
```

## Type Constraints

### Numeric Ranges
```agentconfig
Age :: int & >= 0 & <= 150
Temperature :: float & >= 0.0 & <= 1.0
Port :: int & > 0 & < 65536
```

### String Patterns
```agentconfig
Email :: string & =~ ".*@.*\\..*"
UUID :: string & =~ "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}"
```

### Struct Constraints
```agentconfig
Config :: {
  name: string,
  port: int & > 0,
  enabled: bool,
  timeout_ms: int & > 0 & < 300000
}
```

### Union Types
```agentconfig
Status :: "active" | "pending" | "inactive"
Level :: "debug" | "info" | "warning" | "error"
```

## Testing Patterns

### Unit Test
```agentconfig
test "description" {
  given { facts { ... } }
  when { query ... }
  then { assert ... }
}
```

### Property Test
```agentconfig
property "monotonicity" {
  forall config in valid_configs:
    derived_facts(config + new_facts) ⊇ derived_facts(config)
}
```

### Integration Test
```agentconfig
integration_test "workflow" {
  setup { ... }
  scenario {
    step "name" { expect ... }
  }
  assert { ... }
}
```

## CLI Commands

```bash
# Validate config
agentconfig validate config.ac

# Run tests
agentconfig test config.ac

# Query config
agentconfig query config.ac "use_tool(T)"

# Export config
agentconfig export config.ac --target claude-code --output config.json

# Explain decision
agentconfig explain config.ac "should_run_tests()" --verbose
```

## Common Operations

### Parse a File
```python
from agentconfig.parser import parse_file
config = parse_file("my_agent.ac")
```

### Validate Configuration
```python
is_valid, errors = config.validate()
if not is_valid:
    for error in errors:
        print(error)
```

### Evaluate Rules
```python
engine = config.evaluate()
all_facts = engine.edb | engine.idb
```

### Query Results
```python
from agentconfig.parser import parse_query
query = parse_query("use_tool(T)")
results = engine.query(query)
```

### Export for Agent
```python
exported = config.export_for_agent("claude-code")
with open("config.json", "w") as f:
    json.dump(exported, f)
```

## Debugging Tips

### Enable Debug Logging
```python
import logging
logging.basicConfig(level=logging.DEBUG)
```

### Trace Rule Evaluation
```python
engine.evaluate(trace=True)
```

### Explain Derivation
```python
explanation = engine.explain(fact)
print(explanation)
```

### Check Constraints
```python
violated = config.check_constraints()
for constraint in violated:
    print(f"Violated: {constraint}")
```

## Best Practices

1. **Start Simple**: Begin with basic facts and rules
2. **Test Early**: Write tests as you add rules
3. **Use Types**: Add type constraints for validation
4. **Document**: Comment complex rules
5. **Stratify**: Be careful with negation
6. **Index**: For large fact bases, use indexed storage
7. **Profile**: Use profiling to optimize slow queries

## Performance Tips

- **Index facts** by predicate and first argument
- **Stratify rules** to minimize iterations
- **Use semi-naive** evaluation (don't recompute)
- **Cache queries** for repeated patterns
- **Limit recursion** depth in tests
- **Profile** with cProfile to find bottlenecks

## Common Errors

### ParseError
```
Line 42: Expected ':-' but got ':'
```
Fix: Check rule syntax, use `:-` not `:`

### ValidationError
```
Constraint violated: temperature out of range
```
Fix: Ensure values satisfy constraints

### StratificationError
```
Rule contains unstratified negation
```
Fix: Reorder rules to stratify properly

### UnificationError
```
Cannot unify int with string
```
Fix: Check types match in rules

## Resources

- Language Reference: `docs/language_reference.md`
- Tutorial: `docs/tutorial.md`
- API Docs: `docs/api.md`
- Examples: `examples/`

---

**Quick Start:** Read `START_HERE.md` and follow Phase 1 implementation.
