"""
AgentConfig: Declarative Configuration with Logic Rules
Implementation Sketch (Python)

This demonstrates the core architecture for parsing, validating,
and evaluating AgentConfig files.
"""

from dataclasses import dataclass, field
from typing import Any, Dict, List, Set, Tuple, Optional, Union
from enum import Enum
import re


# ============================================================================
# Core Data Structures
# ============================================================================

@dataclass
class Fact:
    """A ground fact in the extensional database (EDB)."""
    predicate: str
    args: Tuple[Any, ...]
    attributes: Dict[str, Any] = field(default_factory=dict)
    
    def __hash__(self):
        return hash((self.predicate, self.args))
    
    def __repr__(self):
        args_str = ", ".join(str(a) for a in self.args)
        if self.attributes:
            attrs_str = " { " + ", ".join(f"{k} = {v}" for k, v in self.attributes.items()) + " }"
            return f"{self.predicate}({args_str}){attrs_str}"
        return f"{self.predicate}({args_str})"


@dataclass
class Atom:
    """An atom in a rule body or head."""
    predicate: str
    args: List[Union[str, Any]]  # Variables (str) or constants
    negated: bool = False
    attributes: Dict[str, Any] = field(default_factory=dict)


@dataclass
class Rule:
    """A Datalog rule: head :- body."""
    head: Atom
    body: List[Atom]
    
    def __repr__(self):
        body_str = ", ".join(str(atom) for atom in self.body)
        return f"{self.head} :- {body_str}."


@dataclass
class Query:
    """A query to evaluate."""
    name: str
    atom: Atom
    where_clause: Optional[str] = None


@dataclass
class Constraint:
    """A constraint that must be satisfied."""
    name: str
    expression: str
    type: str = "validation"  # validation, security, performance


# ============================================================================
# Parser (Simplified - would use proper parser in production)
# ============================================================================

class AgentConfigParser:
    """Parser for AgentConfig files."""
    
    def parse_file(self, content: str) -> 'AgentConfig':
        """Parse an AgentConfig file."""
        # In production, this would be a proper TOML+Datalog parser
        # For now, we'll represent the structure
        
        return AgentConfig(
            name="example",
            facts=self._parse_facts_section(content),
            rules=self._parse_rules_section(content),
            constraints=self._parse_constraints_section(content),
            queries=self._parse_queries_section(content),
        )
    
    def _parse_facts_section(self, content: str) -> List[Fact]:
        """Extract facts from the content."""
        # Simplified parsing
        return []
    
    def _parse_rules_section(self, content: str) -> List[Rule]:
        """Extract rules from the content."""
        return []
    
    def _parse_constraints_section(self, content: str) -> List[Constraint]:
        """Extract constraints from the content."""
        return []
    
    def _parse_queries_section(self, content: str) -> List[Query]:
        """Extract queries from the content."""
        return []


# ============================================================================
# Datalog Engine (Semi-Naive Evaluation)
# ============================================================================

class DatalogEngine:
    """
    Semi-naive Datalog evaluation engine.
    
    Implements the standard algorithm for computing fixed points
    efficiently by only considering new facts in each iteration.
    """
    
    def __init__(self):
        self.edb: Set[Fact] = set()  # Extensional database (base facts)
        self.idb: Set[Fact] = set()  # Intensional database (derived facts)
        self.rules: List[Rule] = []
    
    def add_fact(self, fact: Fact):
        """Add a base fact to the EDB."""
        self.edb.add(fact)
    
    def add_rule(self, rule: Rule):
        """Add a rule for deriving facts."""
        self.rules.append(rule)
    
    def evaluate(self) -> Set[Fact]:
        """
        Evaluate all rules to compute the complete set of facts.
        Uses semi-naive evaluation for efficiency.
        """
        # Stratify rules to handle negation safely
        strata = self._stratify_rules(self.rules)
        
        all_facts = self.edb.copy()
        
        # Evaluate each stratum to fixed point
        for stratum in strata:
            delta = self.edb.copy()  # New facts to process
            
            while delta:
                new_facts = set()
                
                for rule in stratum:
                    # Evaluate rule with current facts and delta
                    derived = self._evaluate_rule(rule, all_facts, delta)
                    new_facts.update(derived)
                
                # Remove facts we've already seen
                delta = new_facts - all_facts
                all_facts.update(delta)
        
        self.idb = all_facts - self.edb
        return all_facts
    
    def _stratify_rules(self, rules: List[Rule]) -> List[List[Rule]]:
        """
        Stratify rules to handle negation.
        Rules with negation must come after rules that derive the negated facts.
        """
        # Simplified stratification
        # In production, this would do proper dependency analysis
        strata = []
        
        # Separate positive-only rules from those with negation
        positive_rules = [r for r in rules if not any(a.negated for a in r.body)]
        negation_rules = [r for r in rules if any(a.negated for a in r.body)]
        
        if positive_rules:
            strata.append(positive_rules)
        if negation_rules:
            strata.append(negation_rules)
        
        return strata
    
    def _evaluate_rule(
        self, 
        rule: Rule, 
        all_facts: Set[Fact], 
        delta: Set[Fact]
    ) -> Set[Fact]:
        """
        Evaluate a single rule to derive new facts.
        Only considers derivations that use at least one fact from delta.
        """
        new_facts = set()
        
        # Generate all possible variable bindings
        bindings = self._find_bindings(rule.body, all_facts, delta)
        
        # For each valid binding, generate the head fact
        for binding in bindings:
            head_fact = self._instantiate_atom(rule.head, binding)
            if head_fact not in all_facts:
                new_facts.add(head_fact)
        
        return new_facts
    
    def _find_bindings(
        self, 
        body: List[Atom], 
        all_facts: Set[Fact],
        delta: Set[Fact]
    ) -> List[Dict[str, Any]]:
        """
        Find all variable bindings that satisfy the rule body.
        At least one atom must match a fact in delta (semi-naive).
        """
        if not body:
            return [{}]
        
        # Recursive binding search
        return self._find_bindings_recursive(body, all_facts, delta, {}, False)
    
    def _find_bindings_recursive(
        self,
        atoms: List[Atom],
        all_facts: Set[Fact],
        delta: Set[Fact],
        current_binding: Dict[str, Any],
        used_delta: bool
    ) -> List[Dict[str, Any]]:
        """Recursively find bindings for atoms in the rule body."""
        if not atoms:
            return [current_binding] if used_delta else []
        
        atom = atoms[0]
        rest = atoms[1:]
        results = []
        
        # Try matching against all facts (and track if we use delta)
        for fact in all_facts:
            if fact.predicate != atom.predicate:
                continue
            
            # Try to unify atom with fact
            new_binding = self._unify(atom, fact, current_binding.copy())
            if new_binding is not None:
                # Check if this fact is in delta
                is_delta = fact in delta
                
                # Recurse with updated binding
                sub_results = self._find_bindings_recursive(
                    rest, all_facts, delta, new_binding, 
                    used_delta or is_delta
                )
                results.extend(sub_results)
        
        return results
    
    def _unify(
        self, 
        atom: Atom, 
        fact: Fact, 
        binding: Dict[str, Any]
    ) -> Optional[Dict[str, Any]]:
        """
        Unify an atom with a fact given current variable bindings.
        Returns updated bindings if unification succeeds, None otherwise.
        """
        if len(atom.args) != len(fact.args):
            return None
        
        new_binding = binding.copy()
        
        for atom_arg, fact_arg in zip(atom.args, fact.args):
            if isinstance(atom_arg, str) and atom_arg.isupper():
                # Variable
                if atom_arg in new_binding:
                    if new_binding[atom_arg] != fact_arg:
                        return None  # Conflict
                else:
                    new_binding[atom_arg] = fact_arg
            else:
                # Constant
                if atom_arg != fact_arg:
                    return None
        
        # Check attribute constraints
        for key, value in atom.attributes.items():
            if key not in fact.attributes or fact.attributes[key] != value:
                return None
        
        return new_binding
    
    def _instantiate_atom(self, atom: Atom, binding: Dict[str, Any]) -> Fact:
        """Create a concrete fact from an atom using variable bindings."""
        args = tuple(
            binding.get(arg, arg) if isinstance(arg, str) and arg.isupper() else arg
            for arg in atom.args
        )
        return Fact(atom.predicate, args, atom.attributes.copy())
    
    def query(self, query_atom: Atom) -> Set[Fact]:
        """Query for facts matching the given atom pattern."""
        all_facts = self.edb | self.idb
        
        results = set()
        for fact in all_facts:
            if fact.predicate != query_atom.predicate:
                continue
            
            # Check if fact matches query pattern
            binding = self._unify(query_atom, fact, {})
            if binding is not None:
                results.add(fact)
        
        return results


# ============================================================================
# Constraint Validator
# ============================================================================

class ConstraintValidator:
    """Validates constraints against facts and configuration."""
    
    def __init__(self, config: 'AgentConfig'):
        self.config = config
        self.violations: List[str] = []
    
    def validate_all(self) -> bool:
        """Validate all constraints. Returns True if all pass."""
        self.violations = []
        
        for constraint in self.config.constraints:
            if not self._validate_constraint(constraint):
                self.violations.append(
                    f"Constraint '{constraint.name}' violated: {constraint.expression}"
                )
        
        return len(self.violations) == 0
    
    def _validate_constraint(self, constraint: Constraint) -> bool:
        """Validate a single constraint."""
        # In production, this would parse and evaluate the constraint expression
        # For now, simplified
        
        if "temperature" in constraint.expression:
            # Example: validate temperature range
            temp = self.config.metadata.get("temperature", 0.5)
            if "< 0" in constraint.expression or "> 1.0" in constraint.expression:
                return 0.0 <= temp <= 1.0
        
        return True


# ============================================================================
# Agent Configuration
# ============================================================================

@dataclass
class AgentConfig:
    """Complete agent configuration."""
    name: str
    facts: List[Fact] = field(default_factory=list)
    rules: List[Rule] = field(default_factory=list)
    constraints: List[Constraint] = field(default_factory=list)
    queries: List[Query] = field(default_factory=list)
    metadata: Dict[str, Any] = field(default_factory=dict)
    
    def evaluate(self) -> DatalogEngine:
        """Evaluate the configuration and return the Datalog engine."""
        engine = DatalogEngine()
        
        # Add facts
        for fact in self.facts:
            engine.add_fact(fact)
        
        # Add rules
        for rule in self.rules:
            engine.add_rule(rule)
        
        # Evaluate to compute all derived facts
        engine.evaluate()
        
        return engine
    
    def validate(self) -> Tuple[bool, List[str]]:
        """Validate all constraints."""
        validator = ConstraintValidator(self)
        is_valid = validator.validate_all()
        return is_valid, validator.violations
    
    def export_for_agent(self, agent_type: str) -> Dict[str, Any]:
        """Export configuration in agent-specific format."""
        if agent_type == "claude-code":
            return self._export_claude_code()
        elif agent_type == "openai":
            return self._export_openai()
        else:
            raise ValueError(f"Unknown agent type: {agent_type}")
    
    def _export_claude_code(self) -> Dict[str, Any]:
        """Export for Claude Code."""
        engine = self.evaluate()
        
        # Convert rules to natural language instructions
        instructions = self._rules_to_instructions(self.rules)
        
        return {
            "model": self.metadata.get("model", "claude-sonnet-4-5"),
            "system_prompt": self.metadata.get("system_prompt", "") + "\n\n" + instructions,
            "context": self.metadata.get("context", {}),
            "tools": self._map_tools(self.metadata.get("tools", {})),
            "derived_facts": [str(f) for f in engine.idb]
        }
    
    def _export_openai(self) -> Dict[str, Any]:
        """Export for OpenAI Assistants API."""
        return {
            "model": self.metadata.get("model", "gpt-4-turbo"),
            "instructions": self._rules_to_instructions(self.rules),
            "tools": self._map_tools_openai(self.metadata.get("tools", {})),
        }
    
    def _rules_to_instructions(self, rules: List[Rule]) -> str:
        """Convert logical rules to natural language instructions."""
        instructions = ["Follow these rules:"]
        
        for rule in rules:
            # Simplified conversion
            head_str = f"When you need to {rule.head.predicate}"
            body_parts = []
            for atom in rule.body:
                if atom.negated:
                    body_parts.append(f"there is no {atom.predicate}")
                else:
                    body_parts.append(f"there is {atom.predicate}")
            body_str = " and ".join(body_parts)
            instructions.append(f"- {head_str}, ensure {body_str}")
        
        return "\n".join(instructions)
    
    def _map_tools(self, tools: Dict[str, Any]) -> List[Dict[str, Any]]:
        """Map internal tool representation to agent format."""
        # Simplified mapping
        return [
            {"name": name, "config": config}
            for name, config in tools.items()
        ]
    
    def _map_tools_openai(self, tools: Dict[str, Any]) -> List[Dict[str, Any]]:
        """Map tools to OpenAI format."""
        # Would map to OpenAI's function calling format
        return []


# ============================================================================
# Testing Framework
# ============================================================================

@dataclass
class TestCase:
    """A test case for validating configuration rules."""
    name: str
    given_facts: List[Fact]
    when_query: Atom
    then_assertions: List[str]


class ConfigTester:
    """Framework for testing agent configurations."""
    
    def __init__(self, config: AgentConfig):
        self.config = config
        self.test_results: List[Tuple[str, bool, str]] = []
    
    def run_test(self, test: TestCase) -> bool:
        """Run a single test case."""
        # Create temporary engine with test facts
        engine = DatalogEngine()
        
        # Add base facts
        for fact in self.config.facts:
            engine.add_fact(fact)
        
        # Add test-specific facts
        for fact in test.given_facts:
            engine.add_fact(fact)
        
        # Add rules
        for rule in self.config.rules:
            engine.add_rule(rule)
        
        # Evaluate
        engine.evaluate()
        
        # Query results
        results = engine.query(test.when_query)
        
        # Check assertions
        passed = self._check_assertions(test.then_assertions, results)
        
        message = "PASS" if passed else "FAIL"
        self.test_results.append((test.name, passed, message))
        
        return passed
    
    def _check_assertions(self, assertions: List[str], results: Set[Fact]) -> bool:
        """Check if results satisfy all assertions."""
        for assertion in assertions:
            if "exists" in assertion:
                if not results:
                    return False
            elif "count" in assertion:
                # Parse count assertion
                pass
        return True
    
    def run_all_tests(self, tests: List[TestCase]) -> bool:
        """Run all test cases."""
        all_passed = True
        for test in tests:
            passed = self.run_test(test)
            if not passed:
                all_passed = False
        return all_passed
    
    def print_results(self):
        """Print test results."""
        print("\nTest Results:")
        print("=" * 60)
        for name, passed, message in self.test_results:
            status = "✓" if passed else "✗"
            print(f"{status} {name}: {message}")
        print("=" * 60)


# ============================================================================
# Example Usage
# ============================================================================

def example_usage():
    """Demonstrate the AgentConfig system."""
    
    print("AgentConfig System Demo")
    print("=" * 60)
    
    # Create a simple configuration
    config = AgentConfig(
        name="example-agent",
        metadata={
            "model": "claude-sonnet-4-5",
            "temperature": 0.7,
            "system_prompt": "You are a helpful coding assistant."
        }
    )
    
    # Add facts
    config.facts = [
        Fact("language", ("python",), {"style": "pep8"}),
        Fact("language", ("rust",), {"style": "rustfmt"}),
        Fact("tool", ("pytest",), {"for_language": "python", "type": "testing"}),
        Fact("tool", ("black",), {"for_language": "python", "type": "formatting"}),
        Fact("current_file", ("test.py",)),
        Fact("file_extension", ("test.py", ".py")),
    ]
    
    # Add rules
    config.rules = [
        Rule(
            head=Atom("use_tool", ["T"]),
            body=[
                Atom("current_file", ["F"]),
                Atom("file_extension", ["F", ".py"]),
                Atom("tool", ["T"], attributes={"for_language": "python"}),
            ]
        ),
        Rule(
            head=Atom("apply_style", ["Lang", "Style"]),
            body=[
                Atom("language", ["Lang"], attributes={"style": "Style"}),
                Atom("current_file", ["F"]),
            ]
        ),
    ]
    
    # Add constraints
    config.constraints = [
        Constraint(
            name="temperature_range",
            expression="temperature >= 0.0 && temperature <= 1.0"
        )
    ]
    
    print("\n1. Validating configuration...")
    is_valid, violations = config.validate()
    if is_valid:
        print("   ✓ All constraints satisfied")
    else:
        print("   ✗ Validation failed:")
        for violation in violations:
            print(f"     - {violation}")
    
    print("\n2. Evaluating Datalog rules...")
    engine = config.evaluate()
    print(f"   Base facts (EDB): {len(engine.edb)}")
    print(f"   Derived facts (IDB): {len(engine.idb)}")
    
    print("\n3. Running queries...")
    
    # Query: What tools should be used?
    query = Atom("use_tool", ["T"])
    results = engine.query(query)
    print(f"   Query: use_tool(T)")
    print(f"   Results: {results}")
    
    # Query: What style should be applied?
    query = Atom("apply_style", ["Lang", "Style"])
    results = engine.query(query)
    print(f"   Query: apply_style(Lang, Style)")
    print(f"   Results: {results}")
    
    print("\n4. Testing with test cases...")
    tester = ConfigTester(config)
    
    test1 = TestCase(
        name="Python files use Python tools",
        given_facts=[
            Fact("current_file", ("script.py",)),
            Fact("file_extension", ("script.py", ".py")),
        ],
        when_query=Atom("use_tool", ["T"]),
        then_assertions=["exists T"]
    )
    
    tester.run_test(test1)
    tester.print_results()
    
    print("\n5. Exporting for Claude Code...")
    claude_config = config.export_for_agent("claude-code")
    print(f"   Model: {claude_config['model']}")
    print(f"   Derived facts: {len(claude_config['derived_facts'])} facts")
    print(f"   Instructions generated: {len(claude_config['system_prompt'])} chars")
    
    print("\n" + "=" * 60)
    print("Demo complete!")


if __name__ == "__main__":
    example_usage()
