# AgentConfig Implementation Guide for AI Coding Agents

**Target Audience:** AI coding agents (Claude Code, Codex, etc.)  
**Objective:** Implement a production-ready AgentConfig system  
**Language:** Python (primary), with TypeScript/Rust options for performance-critical components

## 🎯 WHAT TO BUILD

You are implementing **AgentConfig**: a declarative configuration language that combines TOML/YAML ergonomics with Datalog-style logic programming for AI agent configuration.

### Core Capabilities Required

1. **Parser**: Parse `.ac` files (TOML + Datalog syntax)
2. **Type System**: Gradual constraint-based typing (CUE-inspired)
3. **Datalog Engine**: Semi-naive evaluation with stratification
4. **Constraint Solver**: Validate constraints and type checking
5. **Testing Framework**: Unit, property, and integration tests
6. **CLI Tool**: Command-line interface for validation, testing, export
7. **Agent Adapters**: Export to Claude Code, OpenAI, Gemini formats
8. **LSP Server** (stretch): Language Server Protocol for IDE support

## 📋 IMPLEMENTATION PRIORITIES

### Phase 1: Core Language (IMPLEMENT FIRST)
**Priority: CRITICAL**

```
Week 1-2: Parser & AST
├── TOML parser (use tomli/tomlkit)
├── Datalog syntax parser (custom or lark-parser)
├── AST data structures
└── Basic validation

Week 3-4: Type System
├── Type definitions and constraints
├── Unification algorithm
├── Constraint propagation
└── Type checking

Week 5-6: Datalog Engine
├── Fact storage (EDB)
├── Rule evaluation (IDB)
├── Semi-naive algorithm
├── Stratification for negation
└── Query resolver

Week 7-8: Integration
├── Connect all components
├── End-to-end evaluation
├── Error handling
└── Basic tests
```

### Phase 2: Tooling & Testing
**Priority: HIGH**

```
Week 9-10: CLI Tool
├── agentconfig validate
├── agentconfig test
├── agentconfig query
├── agentconfig export
└── agentconfig explain

Week 11-12: Testing Framework
├── Test case parser
├── Test runner
├── Property-based testing
└── Integration test support
```

### Phase 3: Agent Integration
**Priority: HIGH**

```
Week 13-14: Adapters
├── Claude Code adapter
├── OpenAI adapter
├── Gemini adapter
└── Generic JSON export
```

### Phase 4: Advanced Features (OPTIONAL)
**Priority: MEDIUM**

```
Week 15-16: LSP Server
├── Syntax highlighting
├── Autocomplete
├── Validation
└── Go to definition

Week 17-18: Optimization
├── Indexing for fast queries
├── Caching
├── Incremental evaluation
└── Performance profiling
```

## 🏗️ PROJECT STRUCTURE

Create this directory structure:

```
agentconfig/
├── README.md                    # User-facing documentation
├── IMPLEMENTATION.md            # This file
├── pyproject.toml              # Python project config
├── setup.py                    # Installation
│
├── src/
│   └── agentconfig/
│       ├── __init__.py
│       │
│       ├── parser/              # Phase 1
│       │   ├── __init__.py
│       │   ├── toml_parser.py  # Parse TOML sections
│       │   ├── datalog_parser.py # Parse rules/queries
│       │   ├── ast.py          # AST node definitions
│       │   └── validator.py    # Syntax validation
│       │
│       ├── types/               # Phase 1
│       │   ├── __init__.py
│       │   ├── type_system.py  # Type definitions
│       │   ├── constraints.py  # Constraint types
│       │   ├── unification.py  # Unification algorithm
│       │   └── checker.py      # Type checking
│       │
│       ├── engine/              # Phase 1
│       │   ├── __init__.py
│       │   ├── datalog.py      # Main Datalog engine
│       │   ├── fact_store.py   # EDB storage
│       │   ├── evaluator.py    # Semi-naive evaluation
│       │   ├── stratifier.py   # Rule stratification
│       │   └── query.py        # Query resolver
│       │
│       ├── testing/             # Phase 2
│       │   ├── __init__.py
│       │   ├── test_case.py    # Test case definitions
│       │   ├── runner.py       # Test runner
│       │   ├── property.py     # Property-based testing
│       │   └── assertions.py   # Assertion engine
│       │
│       ├── adapters/            # Phase 3
│       │   ├── __init__.py
│       │   ├── base.py         # Base adapter interface
│       │   ├── claude_code.py  # Claude Code export
│       │   ├── openai.py       # OpenAI export
│       │   └── gemini.py       # Gemini export
│       │
│       ├── cli/                 # Phase 2
│       │   ├── __init__.py
│       │   ├── main.py         # CLI entry point
│       │   ├── commands.py     # Command implementations
│       │   └── output.py       # Pretty printing
│       │
│       └── lsp/                 # Phase 4 (optional)
│           ├── __init__.py
│           ├── server.py       # LSP server
│           └── features.py     # LSP features
│
├── tests/                       # Unit tests
│   ├── test_parser.py
│   ├── test_types.py
│   ├── test_engine.py
│   ├── test_testing.py
│   └── test_adapters.py
│
├── examples/                    # Example configs
│   ├── simple.ac
│   ├── python_assistant.ac
│   ├── security_policies.ac
│   └── production_agent.ac
│
├── docs/                        # Documentation
│   ├── design.md               # From deliverables
│   ├── language_reference.md
│   ├── tutorial.md
│   └── api.md
│
└── benchmarks/                  # Performance tests
    ├── small_config.ac
    ├── medium_config.ac
    └── large_config.ac
```

## 🔧 TECHNICAL SPECIFICATIONS

### 1. Parser Implementation

**Use:** `lark-parser` for Datalog syntax, `tomllib` (Python 3.11+) or `tomli` for TOML

**Grammar for Datalog portion:**
```lark
?start: rule+

rule: atom ":-" atom_list "."

atom: IDENTIFIER "(" arg_list ")" ("{" attr_list "}")?
    | "not" atom

atom_list: atom ("," atom)*

arg_list: arg ("," arg)*
arg: VARIABLE | CONSTANT | STRING

attr_list: attr ("," attr)*
attr: IDENTIFIER "=" value

VARIABLE: /[A-Z][a-zA-Z0-9_]*/
IDENTIFIER: /[a-z][a-zA-Z0-9_]*/
CONSTANT: /[a-z0-9_]+/ | STRING | NUMBER
```

**Key Classes:**
```python
@dataclass
class Atom:
    predicate: str
    args: List[Union[Variable, Constant]]
    negated: bool = False
    attributes: Dict[str, Any] = field(default_factory=dict)

@dataclass
class Rule:
    head: Atom
    body: List[Atom]

@dataclass
class AgentConfig:
    name: str
    metadata: Dict[str, Any]
    facts: List[Fact]
    rules: List[Rule]
    constraints: List[Constraint]
    queries: List[Query]
```

### 2. Type System Implementation

**Algorithm:** Hindley-Milner-style unification with constraints

**Key Classes:**
```python
class Type:
    """Base type class."""
    pass

class PrimitiveType(Type):
    """int, string, bool, float"""
    kind: str

class StructType(Type):
    """Record/struct types"""
    fields: Dict[str, Type]

class ConstrainedType(Type):
    """Type with constraints: int & > 0 & < 100"""
    base_type: Type
    constraints: List[Constraint]

class UnionType(Type):
    """Sum types: "A" | "B" | "C" """
    variants: List[Type]

def unify(t1: Type, t2: Type) -> Type:
    """Unify two types, return most specific type."""
    # Implementation of Robinson's unification
    pass

def check_constraints(value: Any, type: Type) -> bool:
    """Check if value satisfies type constraints."""
    pass
```

### 3. Datalog Engine Implementation

**Algorithm:** Semi-naive evaluation

**Pseudo-code:**
```python
def evaluate(facts: Set[Fact], rules: List[Rule]) -> Set[Fact]:
    """Semi-naive evaluation."""
    # 1. Stratify rules
    strata = stratify_rules(rules)
    
    all_facts = facts.copy()
    
    # 2. Evaluate each stratum
    for stratum in strata:
        delta = facts.copy()  # New facts
        
        # 3. Fixed-point iteration
        while delta:
            new_facts = set()
            
            for rule in stratum:
                # Only use derivations involving ≥1 delta fact
                derived = evaluate_rule(rule, all_facts, delta)
                new_facts.update(derived)
            
            delta = new_facts - all_facts
            all_facts.update(delta)
    
    return all_facts
```

**Key Optimizations:**
```python
class FactStore:
    """Indexed fact storage for fast lookups."""
    
    def __init__(self):
        # Index by predicate
        self.by_predicate: Dict[str, Set[Fact]] = {}
        
        # Index by first argument (for joins)
        self.by_first_arg: Dict[Any, Set[Fact]] = {}
    
    def add(self, fact: Fact):
        """Add fact with indexing."""
        self.by_predicate[fact.predicate].add(fact)
        if fact.args:
            self.by_first_arg[fact.args[0]].add(fact)
    
    def lookup(self, predicate: str, first_arg: Any = None) -> Set[Fact]:
        """Fast indexed lookup."""
        if first_arg is not None:
            return self.by_first_arg.get(first_arg, set())
        return self.by_predicate.get(predicate, set())
```

### 4. Testing Framework Implementation

**Test Case Format:**
```python
@dataclass
class TestCase:
    name: str
    given_facts: List[Fact]
    when_query: Atom
    then_assertions: List[Assertion]

class Assertion:
    def check(self, results: Set[Fact]) -> bool:
        pass

class ExistsAssertion(Assertion):
    pattern: Atom
    
    def check(self, results: Set[Fact]) -> bool:
        return any(matches(f, self.pattern) for f in results)

class CountAssertion(Assertion):
    operator: str  # "==", ">", "<", etc.
    value: int
    
    def check(self, results: Set[Fact]) -> bool:
        return eval(f"{len(results)} {self.operator} {self.value}")
```

### 5. CLI Tool Implementation

**Use:** `click` or `typer` for CLI framework

```python
import click

@click.group()
def cli():
    """AgentConfig CLI tool."""
    pass

@cli.command()
@click.argument('config_file')
def validate(config_file):
    """Validate an AgentConfig file."""
    config = parse_file(config_file)
    is_valid, errors = config.validate()
    
    if is_valid:
        click.echo(click.style("✓ Valid", fg="green"))
    else:
        click.echo(click.style("✗ Invalid", fg="red"))
        for error in errors:
            click.echo(f"  - {error}")

@cli.command()
@click.argument('config_file')
@click.argument('query')
def query(config_file, query):
    """Run a query against a config."""
    config = parse_file(config_file)
    engine = config.evaluate()
    results = engine.query(parse_query(query))
    
    for result in results:
        click.echo(result)

@cli.command()
@click.argument('config_file')
@click.option('--target', default='claude-code')
@click.option('--output', default='output.json')
def export(config_file, target, output):
    """Export config for an agent."""
    config = parse_file(config_file)
    exported = config.export_for_agent(target)
    
    with open(output, 'w') as f:
        json.dump(exported, f, indent=2)
    
    click.echo(f"Exported to {output}")
```

## 🧪 TESTING STRATEGY

### Unit Tests
Test each component in isolation:

```python
# tests/test_engine.py
def test_simple_rule_evaluation():
    engine = DatalogEngine()
    
    # Add facts
    engine.add_fact(Fact("parent", ("alice", "bob")))
    engine.add_fact(Fact("parent", ("bob", "charlie")))
    
    # Add rule: grandparent(X, Z) :- parent(X, Y), parent(Y, Z)
    engine.add_rule(Rule(
        head=Atom("grandparent", ["X", "Z"]),
        body=[
            Atom("parent", ["X", "Y"]),
            Atom("parent", ["Y", "Z"])
        ]
    ))
    
    # Evaluate
    all_facts = engine.evaluate()
    
    # Check result
    assert Fact("grandparent", ("alice", "charlie")) in all_facts
```

### Integration Tests
Test full workflows:

```python
# tests/test_integration.py
def test_full_config_evaluation():
    config = parse_file("examples/python_assistant.ac")
    
    # Should parse without errors
    assert config is not None
    
    # Should validate
    is_valid, errors = config.validate()
    assert is_valid, f"Validation errors: {errors}"
    
    # Should evaluate
    engine = config.evaluate()
    assert len(engine.idb) > 0  # Should derive some facts
    
    # Should export
    exported = config.export_for_agent("claude-code")
    assert "model" in exported
    assert "tools" in exported
```

### Performance Tests
Benchmark on different config sizes:

```python
# benchmarks/bench_evaluation.py
import time

def benchmark_evaluation(config_file):
    start = time.time()
    config = parse_file(config_file)
    parse_time = time.time() - start
    
    start = time.time()
    engine = config.evaluate()
    eval_time = time.time() - start
    
    return {
        "parse_time_ms": parse_time * 1000,
        "eval_time_ms": eval_time * 1000,
        "facts": len(engine.edb),
        "derived": len(engine.idb)
    }
```

## 📝 IMPLEMENTATION CHECKLIST

### Phase 1: Core Language ✓ Priority 1

- [ ] **Parser**
  - [ ] TOML section parser
  - [ ] Datalog rule parser
  - [ ] AST node classes
  - [ ] Syntax validator
  - [ ] Error messages
  
- [ ] **Type System**
  - [ ] Type definitions (primitive, struct, union, constrained)
  - [ ] Unification algorithm
  - [ ] Constraint checking
  - [ ] Type inference
  - [ ] Error reporting
  
- [ ] **Datalog Engine**
  - [ ] Fact storage (indexed)
  - [ ] Rule stratification
  - [ ] Semi-naive evaluator
  - [ ] Query resolver
  - [ ] Negation handling
  
- [ ] **Integration**
  - [ ] Connect parser → types → engine
  - [ ] AgentConfig class
  - [ ] Validation pipeline
  - [ ] Basic error handling

### Phase 2: Tooling ✓ Priority 2

- [ ] **CLI Tool**
  - [ ] `agentconfig validate`
  - [ ] `agentconfig test`
  - [ ] `agentconfig query`
  - [ ] `agentconfig export`
  - [ ] Pretty output formatting
  
- [ ] **Testing Framework**
  - [ ] Test case parser
  - [ ] Assertion types
  - [ ] Test runner
  - [ ] Property-based testing
  - [ ] Test reporting

### Phase 3: Adapters ✓ Priority 2

- [ ] **Base Adapter**
  - [ ] Adapter interface
  - [ ] Rule → instruction converter
  - [ ] Tool mapping
  
- [ ] **Claude Code**
  - [ ] JSON format export
  - [ ] System prompt generation
  - [ ] Tool configuration
  
- [ ] **OpenAI**
  - [ ] Assistant API format
  - [ ] Function calling format
  
- [ ] **Gemini**
  - [ ] Gemini format export

### Phase 4: Advanced ✓ Priority 3

- [ ] **LSP Server** (optional)
  - [ ] Basic server setup
  - [ ] Syntax highlighting
  - [ ] Validation
  - [ ] Autocomplete
  
- [ ] **Optimization**
  - [ ] Indexing improvements
  - [ ] Caching
  - [ ] Profiling
  - [ ] Performance tuning

## 🚀 GETTING STARTED - FIRST STEPS

### Step 1: Set up project structure
```bash
mkdir -p agentconfig/src/agentconfig/{parser,types,engine,testing,adapters,cli}
touch agentconfig/src/agentconfig/{__init__,parser,types,engine,testing,adapters,cli}/__init__.py
```

### Step 2: Create pyproject.toml
```toml
[project]
name = "agentconfig"
version = "0.1.0"
description = "Declarative configuration for AI agents"
requires-python = ">=3.11"
dependencies = [
    "lark-parser>=0.12.0",
    "tomli>=2.0.0",
    "click>=8.1.0",
    "typing-extensions>=4.5.0",
]

[project.optional-dependencies]
dev = [
    "pytest>=7.0.0",
    "pytest-cov>=4.0.0",
    "black>=23.0.0",
    "mypy>=1.0.0",
    "ruff>=0.1.0",
]

[project.scripts]
agentconfig = "agentconfig.cli.main:cli"
```

### Step 3: Start with parser
Begin implementing `src/agentconfig/parser/ast.py` with the AST node definitions from the design doc.

### Step 4: Build incrementally
After each major component, write tests and verify it works before moving to the next.

## 💡 IMPLEMENTATION TIPS

### 1. Use Type Hints Everywhere
```python
from typing import List, Set, Dict, Optional, Union

def evaluate_rule(
    rule: Rule,
    all_facts: Set[Fact],
    delta: Set[Fact]
) -> Set[Fact]:
    """Evaluate a single rule."""
    pass
```

### 2. Handle Errors Gracefully
```python
class AgentConfigError(Exception):
    """Base exception for AgentConfig."""
    pass

class ParseError(AgentConfigError):
    """Parsing failed."""
    def __init__(self, message: str, line: int, column: int):
        self.line = line
        self.column = column
        super().__init__(f"Line {line}, column {column}: {message}")

class ValidationError(AgentConfigError):
    """Validation failed."""
    pass
```

### 3. Make it Debuggable
```python
import logging

logger = logging.getLogger(__name__)

def evaluate(self) -> Set[Fact]:
    """Evaluate all rules."""
    logger.info("Starting evaluation")
    logger.debug(f"EDB size: {len(self.edb)}")
    
    for i, stratum in enumerate(self.strata):
        logger.debug(f"Evaluating stratum {i}")
        # ... evaluation logic
    
    logger.info(f"Evaluation complete. Derived {len(self.idb)} facts")
    return self.edb | self.idb
```

### 4. Write Tests First (TDD)
For each feature, write the test before implementing:

```python
# Write this first
def test_unification():
    t1 = StructType({"x": PrimitiveType("int")})
    t2 = StructType({"x": ConstrainedType(PrimitiveType("int"), [">0"])})
    
    result = unify(t1, t2)
    
    assert isinstance(result, StructType)
    assert isinstance(result.fields["x"], ConstrainedType)

# Then implement
def unify(t1: Type, t2: Type) -> Type:
    # Implementation here
    pass
```

### 5. Optimize Later
Get it working first, then profile and optimize:

```python
# First version (simple, slow)
def find_matches(pattern: Atom, facts: Set[Fact]) -> Set[Fact]:
    return {f for f in facts if matches(f, pattern)}

# Optimized version (after profiling shows this is slow)
def find_matches(pattern: Atom, facts: Set[Fact]) -> Set[Fact]:
    # Use index for O(1) lookup instead of O(n) scan
    if pattern.predicate in self.index:
        candidates = self.index[pattern.predicate]
        return {f for f in candidates if matches(f, pattern)}
    return set()
```

## 📚 KEY ALGORITHMS TO IMPLEMENT

### 1. Semi-Naive Evaluation
```python
def semi_naive_evaluate(rules: List[Rule], facts: Set[Fact]) -> Set[Fact]:
    """
    Semi-naive evaluation algorithm.
    Only processes rules that involve new facts.
    """
    all_facts = facts.copy()
    delta = facts.copy()  # Initially, all facts are "new"
    
    while delta:
        new_facts = set()
        
        for rule in rules:
            # For each rule, find all derivations that use
            # at least one fact from delta
            derived = evaluate_rule_with_delta(rule, all_facts, delta)
            new_facts.update(derived)
        
        # Delta is only the truly new facts
        delta = new_facts - all_facts
        all_facts.update(delta)
    
    return all_facts
```

### 2. Rule Stratification
```python
def stratify_rules(rules: List[Rule]) -> List[List[Rule]]:
    """
    Stratify rules based on dependencies and negation.
    Ensures safe evaluation of negation.
    """
    # Build dependency graph
    graph = {}
    neg_edges = set()  # Negative dependencies
    
    for rule in rules:
        pred = rule.head.predicate
        graph[pred] = set()
        
        for atom in rule.body:
            graph[pred].add(atom.predicate)
            if atom.negated:
                neg_edges.add((pred, atom.predicate))
    
    # Compute strongly connected components
    sccs = tarjan_scc(graph)
    
    # Order SCCs respecting negative edges
    return topological_sort(sccs, neg_edges)
```

### 3. Unification
```python
def unify(pattern: Atom, fact: Fact, bindings: Dict[str, Any]) -> Optional[Dict[str, Any]]:
    """
    Unify a pattern with a fact, extending bindings.
    Returns updated bindings or None if unification fails.
    """
    if pattern.predicate != fact.predicate:
        return None
    
    if len(pattern.args) != len(fact.args):
        return None
    
    new_bindings = bindings.copy()
    
    for pattern_arg, fact_arg in zip(pattern.args, fact.args):
        if isinstance(pattern_arg, Variable):
            var_name = pattern_arg.name
            if var_name in new_bindings:
                # Variable already bound, check consistency
                if new_bindings[var_name] != fact_arg:
                    return None
            else:
                # Bind variable
                new_bindings[var_name] = fact_arg
        else:
            # Constant, must match
            if pattern_arg != fact_arg:
                return None
    
    return new_bindings
```

## 🎓 LEARNING RESOURCES FOR IMPLEMENTATION

### Core Concepts
- **Datalog**: "Foundations of Databases" (Alice book) - free online
- **Semi-naive evaluation**: Soufflé documentation
- **Type systems**: "Types and Programming Languages" (Pierce)
- **Parsing**: Lark tutorial, Python parsing guide

### Reference Implementations
- **Soufflé**: C++ Datalog engine (reference for algorithms)
- **CUE**: Go implementation (reference for constraint types)
- **OPA/Rego**: Go implementation (reference for policy eval)

### Python Tools
- **lark-parser**: Grammar-based parsing
- **dataclasses**: Immutable data structures
- **typing**: Type hints
- **pytest**: Testing framework
- **click/typer**: CLI framework

## 🐛 DEBUGGING TIPS

### 1. Trace Evaluation
```python
def evaluate_rule(rule: Rule, all_facts: Set[Fact], delta: Set[Fact]) -> Set[Fact]:
    """Evaluate rule with tracing."""
    logger.debug(f"Evaluating rule: {rule}")
    
    bindings = find_bindings(rule.body, all_facts, delta)
    logger.debug(f"Found {len(bindings)} bindings")
    
    new_facts = set()
    for binding in bindings:
        fact = instantiate(rule.head, binding)
        logger.debug(f"Derived: {fact}")
        new_facts.add(fact)
    
    return new_facts
```

### 2. Visualize Derivations
```python
def explain(query: Atom, all_facts: Set[Fact]) -> str:
    """
    Explain why a query result was derived.
    Shows the derivation tree.
    """
    # Find matching facts
    results = [f for f in all_facts if matches(f, query)]
    
    explanations = []
    for result in results:
        tree = build_derivation_tree(result, all_facts)
        explanations.append(format_tree(tree))
    
    return "\n\n".join(explanations)
```

### 3. Validate Invariants
```python
def evaluate(self) -> Set[Fact]:
    """Evaluate with invariant checking."""
    all_facts = self.edb.copy()
    
    for iteration in range(MAX_ITERATIONS):
        old_size = len(all_facts)
        
        # ... evaluation logic ...
        
        # Check invariants
        assert len(all_facts) >= old_size, "Monotonicity violated!"
        
        if len(all_facts) == old_size:
            break  # Fixed point
    
    return all_facts
```

## ✅ ACCEPTANCE CRITERIA

Your implementation is complete when:

1. **Parser works**: Can parse all examples from the design doc
2. **Types work**: Type checking catches errors at config time
3. **Engine works**: Correctly evaluates all example rules
4. **Tests pass**: >90% code coverage, all unit tests pass
5. **CLI works**: All commands work as specified
6. **Adapters work**: Can export to Claude Code and OpenAI
7. **Performance**: Evaluates medium config (<500 rules) in <1 second
8. **Documentation**: README and API docs complete
9. **Examples**: 5+ working example configs
10. **No regressions**: Existing examples keep working

## 🎬 NEXT STEPS AFTER IMPLEMENTATION

1. **Package**: Publish to PyPI as `agentconfig`
2. **Documentation**: Full docs site with examples
3. **Community**: Create GitHub repo, accept contributions
4. **Extensions**: VSCode extension, additional adapters
5. **Optimization**: Profile and optimize hot paths
6. **Research**: Paper on the design and use cases

## 🤝 NEED HELP?

Refer to these files in the package:
- `design.md`: Complete system design
- `implementation.py`: Reference implementation
- `real_world_example.ac`: Complete working example
- `advantages.md`: Design rationale and comparisons

Good luck! Build something awesome! 🚀
