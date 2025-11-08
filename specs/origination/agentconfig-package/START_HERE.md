# 🤖 CODING AGENT PROMPT: AgentConfig Implementation

**You are an AI coding agent tasked with implementing AgentConfig.**

## 📌 YOUR MISSION

Implement a production-ready **AgentConfig** system: a declarative configuration language combining TOML/YAML ergonomics with Datalog-style logic programming for AI agent configuration.

## 📁 PROVIDED FILES

You have been given:

1. **IMPLEMENTATION.md** - Your primary guide (READ THIS FIRST!)
   - Complete implementation roadmap
   - Project structure
   - Code specifications
   - Implementation priorities
   - Testing strategy
   - Acceptance criteria

2. **agentconfig-design.md** - System design specification
   - Language syntax and semantics
   - Type system details
   - Datalog engine architecture
   - Testing framework design
   - Agent integration patterns
   - Complete examples

3. **agentconfig_implementation.py** - Reference implementation
   - Working Python code demonstrating core concepts
   - Can be used as a starting point or reference
   - Shows Datalog engine, parser basics, testing

4. **real_world_example.ac** - Production configuration example
   - 500+ lines of real AgentConfig code
   - Shows all language features in use
   - Use as test case for your implementation

5. **agentconfig-advantages.md** - Design rationale
   - Why this design works
   - Comparison with alternatives
   - Real-world use cases

## 🎯 IMPLEMENTATION PRIORITY

**START HERE - Phase 1 (Weeks 1-8):**

### Week 1-2: Parser & AST
```
✓ Set up Python project structure
✓ Implement TOML parser (use tomli/tomlkit)
✓ Implement Datalog parser (use lark-parser)
✓ Define AST node classes
✓ Write parser tests
```

### Week 3-4: Type System
```
✓ Implement type definitions (primitive, struct, union, constrained)
✓ Implement unification algorithm
✓ Implement constraint checking
✓ Write type system tests
```

### Week 5-6: Datalog Engine
```
✓ Implement fact storage (indexed)
✓ Implement semi-naive evaluation
✓ Implement rule stratification
✓ Implement query resolver
✓ Write engine tests
```

### Week 7-8: Integration
```
✓ Connect all components
✓ Implement AgentConfig main class
✓ End-to-end validation
✓ Integration tests
```

## 🏗️ PROJECT STRUCTURE TO CREATE

```
agentconfig/
├── pyproject.toml           # Python project config
├── README.md                # User documentation
├── IMPLEMENTATION.md        # Your guide
│
├── src/agentconfig/
│   ├── __init__.py
│   ├── parser/              # Phase 1: Week 1-2
│   │   ├── toml_parser.py
│   │   ├── datalog_parser.py
│   │   ├── ast.py
│   │   └── validator.py
│   │
│   ├── types/               # Phase 1: Week 3-4
│   │   ├── type_system.py
│   │   ├── constraints.py
│   │   ├── unification.py
│   │   └── checker.py
│   │
│   ├── engine/              # Phase 1: Week 5-6
│   │   ├── datalog.py
│   │   ├── fact_store.py
│   │   ├── evaluator.py
│   │   ├── stratifier.py
│   │   └── query.py
│   │
│   ├── testing/             # Phase 2
│   ├── adapters/            # Phase 3
│   └── cli/                 # Phase 2
│
├── tests/                   # Unit tests
├── examples/                # Example configs
└── docs/                    # Documentation
```

## 📋 STEP-BY-STEP INSTRUCTIONS

### Step 1: Read & Understand (15-30 minutes)
1. Read **IMPLEMENTATION.md** thoroughly
2. Skim **agentconfig-design.md** for context
3. Look at **real_world_example.ac** to see the language

### Step 2: Set Up Project (30 minutes)
```bash
# Create project structure
mkdir -p agentconfig/src/agentconfig/{parser,types,engine,testing,adapters,cli}
cd agentconfig

# Create pyproject.toml (spec in IMPLEMENTATION.md)
# Create __init__.py files
# Set up git repo
```

### Step 3: Implement Parser (Week 1-2)
Start with `src/agentconfig/parser/ast.py`:

```python
"""AST node definitions for AgentConfig."""
from dataclasses import dataclass, field
from typing import List, Dict, Any, Union

@dataclass
class Atom:
    """An atom in a rule or query."""
    predicate: str
    args: List[Union[str, Any]]
    negated: bool = False
    attributes: Dict[str, Any] = field(default_factory=dict)

@dataclass
class Rule:
    """A Datalog rule: head :- body."""
    head: Atom
    body: List[Atom]

# ... more AST nodes (see IMPLEMENTATION.md)
```

Then implement parsers:
- `toml_parser.py` for TOML sections
- `datalog_parser.py` for rules/queries

### Step 4: Implement Type System (Week 3-4)
See **IMPLEMENTATION.md** section "Type System Implementation" for:
- Type class hierarchy
- Unification algorithm
- Constraint checking

### Step 5: Implement Datalog Engine (Week 5-6)
See **IMPLEMENTATION.md** section "Datalog Engine Implementation" for:
- Semi-naive evaluation algorithm
- Rule stratification
- Query resolution

### Step 6: Write Tests (Throughout)
For each component:
```python
# tests/test_parser.py
def test_parse_simple_rule():
    text = 'foo(X) :- bar(X).'
    rule = parse_rule(text)
    assert rule.head.predicate == 'foo'
    assert len(rule.body) == 1

# tests/test_engine.py
def test_simple_derivation():
    engine = DatalogEngine()
    engine.add_fact(Fact("parent", ("alice", "bob")))
    # ... add rule ...
    results = engine.evaluate()
    # ... assert results ...
```

### Step 7: Build CLI Tool (Week 9-10)
```python
# src/agentconfig/cli/main.py
import click

@click.group()
def cli():
    """AgentConfig CLI."""
    pass

@cli.command()
@click.argument('config_file')
def validate(config_file):
    """Validate a config file."""
    # Implementation here
```

### Step 8: Implement Adapters (Week 13-14)
```python
# src/agentconfig/adapters/claude_code.py
def export_claude_code(config: AgentConfig) -> dict:
    """Export config for Claude Code."""
    return {
        "model": config.metadata.get("model"),
        "system_prompt": render_rules(config.rules),
        # ... more
    }
```

## ✅ VERIFICATION

After each phase, verify:

```bash
# Parser works
python -m pytest tests/test_parser.py

# Can parse example
python -c "from agentconfig.parser import parse_file; parse_file('examples/simple.ac')"

# Engine works
python -m pytest tests/test_engine.py

# Full integration
python -m pytest tests/

# CLI works
agentconfig validate examples/real_world_example.ac
agentconfig query examples/simple.ac "use_tool(T)"
```

## 🎓 KEY ALGORITHMS TO IMPLEMENT

### 1. Semi-Naive Evaluation
Location: `src/agentconfig/engine/evaluator.py`

```python
def semi_naive_evaluate(rules, facts):
    all_facts = facts.copy()
    delta = facts.copy()
    
    while delta:
        new_facts = set()
        for rule in rules:
            derived = evaluate_rule_with_delta(rule, all_facts, delta)
            new_facts.update(derived)
        delta = new_facts - all_facts
        all_facts.update(delta)
    
    return all_facts
```

### 2. Unification
Location: `src/agentconfig/engine/query.py`

```python
def unify(pattern, fact, bindings):
    if pattern.predicate != fact.predicate:
        return None
    
    new_bindings = bindings.copy()
    for p_arg, f_arg in zip(pattern.args, fact.args):
        if is_variable(p_arg):
            if p_arg in new_bindings:
                if new_bindings[p_arg] != f_arg:
                    return None
            else:
                new_bindings[p_arg] = f_arg
        elif p_arg != f_arg:
            return None
    
    return new_bindings
```

### 3. Rule Stratification
Location: `src/agentconfig/engine/stratifier.py`

```python
def stratify_rules(rules):
    # Build dependency graph
    graph = build_dependency_graph(rules)
    
    # Find strongly connected components
    sccs = tarjan_scc(graph)
    
    # Topologically sort respecting negative dependencies
    return topological_sort(sccs)
```

## 🚨 COMMON PITFALLS TO AVOID

1. **Don't skip tests**: Write tests as you go
2. **Don't optimize prematurely**: Get it working first
3. **Handle negation carefully**: Stratify rules properly
4. **Index facts**: O(n) scans will be slow
5. **Use type hints**: Makes debugging easier
6. **Handle errors gracefully**: Good error messages
7. **Document as you go**: Docstrings for all functions

## 🎯 SUCCESS CRITERIA

Your implementation is complete when:

✅ All examples from `real_world_example.ac` parse correctly  
✅ Type checking catches constraint violations  
✅ Datalog engine correctly evaluates all example rules  
✅ Test suite has >90% code coverage  
✅ CLI tool works for validate/test/query/export commands  
✅ Can export to Claude Code format  
✅ Can export to OpenAI format  
✅ Medium configs evaluate in <1 second  
✅ All documentation is complete  

## 🔧 DEPENDENCIES TO USE

From `pyproject.toml`:

```toml
dependencies = [
    "lark-parser>=0.12.0",      # Datalog parsing
    "tomli>=2.0.0",             # TOML parsing (Python 3.11+)
    "click>=8.1.0",             # CLI framework
    "typing-extensions>=4.5.0", # Type hints
]

[project.optional-dependencies]
dev = [
    "pytest>=7.0.0",
    "pytest-cov>=4.0.0",
    "black>=23.0.0",
    "mypy>=1.0.0",
    "ruff>=0.1.0",
]
```

## 📚 WHERE TO FIND ANSWERS

- **How do I parse Datalog?** → `IMPLEMENTATION.md`, section "Parser Implementation"
- **How does semi-naive work?** → `IMPLEMENTATION.md`, section "Datalog Engine"
- **What should the AST look like?** → `agentconfig_implementation.py`
- **How should types work?** → `agentconfig-design.md`, section "Type System"
- **What features are required?** → `real_world_example.ac`
- **How do I test it?** → `IMPLEMENTATION.md`, section "Testing Strategy"

## 🚀 QUICK START COMMANDS

```bash
# 1. Create project
mkdir agentconfig && cd agentconfig

# 2. Copy reference files to docs/
mkdir docs
cp ../agentconfig-design.md docs/
cp ../real_world_example.ac examples/

# 3. Create initial structure
mkdir -p src/agentconfig/{parser,types,engine}
touch src/agentconfig/__init__.py
touch src/agentconfig/parser/{__init__,ast,toml_parser,datalog_parser}.py
touch src/agentconfig/types/{__init__,type_system,constraints,unification}.py
touch src/agentconfig/engine/{__init__,datalog,evaluator,query}.py

# 4. Create pyproject.toml
# (Copy from IMPLEMENTATION.md)

# 5. Start implementing!
# Begin with src/agentconfig/parser/ast.py
```

## 💡 IMPLEMENTATION TIPS

1. **Start simple**: Get basic parsing working first
2. **Build incrementally**: Parser → Types → Engine → CLI
3. **Test early**: Write tests alongside code
4. **Use the reference**: `agentconfig_implementation.py` is your friend
5. **Follow the spec**: `IMPLEMENTATION.md` has all details
6. **Ask for help**: Check design docs when stuck

## 🎬 FINAL NOTES

This is a complete, well-researched design. Everything you need is in these files:

- **IMPLEMENTATION.md** = Your detailed guide
- **agentconfig-design.md** = Complete specification  
- **agentconfig_implementation.py** = Working reference code
- **real_world_example.ac** = Test case and feature demo

The design is elegant, practical, and based on proven technologies (Datalog, CUE, modern config languages). 

Your job is to turn this design into production-ready code.

**You got this! 🚀**

Now go build something awesome.

---

**P.S.** Remember: Get Phase 1 working first (Parser + Types + Engine). Everything else builds on that foundation.
