//! Core Datalog data structures
//!
//! This module defines the fundamental types for the custom Datalog engine:
//! - Terms (variables and constants)
//! - Atoms (predicates with terms)
//! - Rules (Horn clauses)
//! - Substitutions (variable bindings)
//!
//! Design principles:
//! - Arc-based for zero-copy sharing
//! - Compatible with existing FactStore
//! - Support for lock-free concurrent reads

use crate::types::Value;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

/// A term in Datalog (variable or constant)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Term {
    /// Variable (e.g., X, Person, ?x)
    Variable(String),
    /// Constant value
    Constant(Value),
}

impl Term {
    /// Create a variable term
    pub fn var(name: impl Into<String>) -> Self {
        Term::Variable(name.into())
    }

    /// Create a constant term
    pub fn constant(value: Value) -> Self {
        Term::Constant(value)
    }

    /// Check if term is a variable
    pub fn is_variable(&self) -> bool {
        matches!(self, Term::Variable(_))
    }

    /// Check if term is a constant
    pub fn is_constant(&self) -> bool {
        matches!(self, Term::Constant(_))
    }

    /// Get variable name if this is a variable
    pub fn as_variable(&self) -> Option<&str> {
        match self {
            Term::Variable(name) => Some(name),
            _ => None,
        }
    }

    /// Get constant value if this is a constant
    pub fn as_constant(&self) -> Option<&Value> {
        match self {
            Term::Constant(val) => Some(val),
            _ => None,
        }
    }
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Term::Variable(name) => write!(f, "?{}", name),
            Term::Constant(Value::String(s)) => write!(f, "\"{}\"", s),
            Term::Constant(Value::Integer(i)) => write!(f, "{}", i),
            Term::Constant(Value::Bool(b)) => write!(f, "{}", b),
            Term::Constant(Value::Null) => write!(f, "null"),
            Term::Constant(_) => write!(f, "<complex>"),
        }
    }
}

/// An atom in Datalog (predicate with terms)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Atom {
    /// Predicate name
    pub predicate: Arc<str>,
    /// Terms (arguments)
    pub terms: Vec<Term>,
    /// Whether this is a negated atom
    pub negated: bool,
}

impl Atom {
    /// Create a new atom
    pub fn new(predicate: impl Into<String>, terms: Vec<Term>) -> Self {
        Atom {
            predicate: Arc::from(predicate.into().into_boxed_str()),
            terms,
            negated: false,
        }
    }

    /// Create a negated atom
    pub fn negated(predicate: impl Into<String>, terms: Vec<Term>) -> Self {
        Atom {
            predicate: Arc::from(predicate.into().into_boxed_str()),
            terms,
            negated: true,
        }
    }

    /// Get the arity (number of terms)
    pub fn arity(&self) -> usize {
        self.terms.len()
    }

    /// Get all variables in this atom
    pub fn variables(&self) -> Vec<&str> {
        self.terms.iter().filter_map(|t| t.as_variable()).collect()
    }

    /// Check if atom is ground (no variables)
    pub fn is_ground(&self) -> bool {
        self.terms.iter().all(|t| t.is_constant())
    }

    /// Apply substitution to get a new atom
    pub fn apply_substitution(&self, sub: &Substitution) -> Atom {
        Atom {
            predicate: self.predicate.clone(),
            terms: self.terms.iter().map(|t| sub.apply_to_term(t)).collect(),
            negated: self.negated,
        }
    }
}

impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.negated {
            write!(f, "not ")?;
        }
        write!(f, "{}(", self.predicate)?;
        for (i, term) in self.terms.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", term)?;
        }
        write!(f, ")")
    }
}

/// A Datalog rule (Horn clause): head :- body
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// Head of the rule (consequent)
    pub head: Atom,
    /// Body of the rule (antecedents)
    pub body: Vec<Atom>,
    /// Stratification level (for negation)
    pub stratum: usize,
}

impl Rule {
    /// Create a new rule
    pub fn new(head: Atom, body: Vec<Atom>) -> Self {
        Rule {
            head,
            body,
            stratum: 0, // Will be computed during stratification
        }
    }

    /// Create a fact (rule with empty body)
    pub fn fact(head: Atom) -> Self {
        Rule::new(head, vec![])
    }

    /// Check if this is a fact (empty body)
    pub fn is_fact(&self) -> bool {
        self.body.is_empty()
    }

    /// Check if this is a recursive rule
    pub fn is_recursive(&self) -> bool {
        self.body
            .iter()
            .any(|atom| atom.predicate == self.head.predicate)
    }

    /// Get all variables in the rule
    pub fn variables(&self) -> Vec<String> {
        let mut vars = std::collections::HashSet::new();

        // Head variables
        for var in self.head.variables() {
            vars.insert(var.to_string());
        }

        // Body variables
        for atom in &self.body {
            for var in atom.variables() {
                vars.insert(var.to_string());
            }
        }

        vars.into_iter().collect()
    }

    /// Check if rule is safe (all head variables appear in positive body atoms)
    pub fn is_safe(&self) -> bool {
        let head_vars: std::collections::HashSet<_> = self.head.variables().into_iter().collect();

        let positive_body_vars: std::collections::HashSet<_> = self
            .body
            .iter()
            .filter(|a| !a.negated)
            .flat_map(|a| a.variables())
            .collect();

        // All head variables must appear in positive body atoms
        head_vars.is_subset(&positive_body_vars)
    }

    /// Get dependencies (predicates this rule depends on)
    pub fn dependencies(&self) -> Vec<Arc<str>> {
        self.body
            .iter()
            .map(|atom| atom.predicate.clone())
            .collect()
    }
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.head)?;
        if !self.body.is_empty() {
            write!(f, " :- ")?;
            for (i, atom) in self.body.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", atom)?;
            }
        }
        write!(f, ".")
    }
}

/// Variable substitution (binding)
#[derive(Debug, Clone, Default)]
pub struct Substitution {
    /// Variable bindings
    bindings: HashMap<String, Value>,
}

impl Substitution {
    /// Create an empty substitution
    pub fn new() -> Self {
        Substitution {
            bindings: HashMap::new(),
        }
    }

    /// Add a binding
    pub fn bind(&mut self, variable: String, value: Value) {
        self.bindings.insert(variable, value);
    }

    /// Get binding for a variable
    pub fn get(&self, variable: &str) -> Option<&Value> {
        self.bindings.get(variable)
    }

    /// Check if variable is bound
    pub fn contains(&self, variable: &str) -> bool {
        self.bindings.contains_key(variable)
    }

    /// Apply substitution to a term
    pub fn apply_to_term(&self, term: &Term) -> Term {
        match term {
            Term::Variable(name) => {
                if let Some(value) = self.bindings.get(name) {
                    Term::Constant(value.clone())
                } else {
                    term.clone()
                }
            }
            Term::Constant(_) => term.clone(),
        }
    }

    /// Merge two substitutions (returns None if incompatible)
    pub fn merge(&self, other: &Substitution) -> Option<Substitution> {
        let mut result = self.clone();

        for (var, val) in &other.bindings {
            if let Some(existing) = result.bindings.get(var) {
                // Check compatibility
                if existing != val {
                    return None; // Incompatible substitutions
                }
            } else {
                result.bindings.insert(var.clone(), val.clone());
            }
        }

        Some(result)
    }

    /// Get all bindings
    pub fn bindings(&self) -> &HashMap<String, Value> {
        &self.bindings
    }

    /// Number of bindings
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    /// Check if substitution is empty
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
}

impl fmt::Display for Substitution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        for (i, (var, val)) in self.bindings.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{} = {:?}", var, val)?;
        }
        write!(f, "}}")
    }
}

/// Aggregate operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AggregateOp {
    /// Count aggregation
    Count,
    /// Sum aggregation
    Sum,
    /// Minimum
    Min,
    /// Maximum
    Max,
    /// Average (mean)
    Mean,
}

impl fmt::Display for AggregateOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AggregateOp::Count => write!(f, "count"),
            AggregateOp::Sum => write!(f, "sum"),
            AggregateOp::Min => write!(f, "min"),
            AggregateOp::Max => write!(f, "max"),
            AggregateOp::Mean => write!(f, "mean"),
        }
    }
}

/// Aggregate atom (e.g., count(?X, R) :- edge(?X, ?Y))
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateAtom {
    /// Aggregate operation
    pub op: AggregateOp,
    /// Variable to aggregate over
    pub aggregate_var: String,
    /// Result variable
    pub result_var: String,
    /// Body atoms
    pub body: Vec<Atom>,
}

impl AggregateAtom {
    /// Create a new aggregate atom
    pub fn new(
        op: AggregateOp,
        aggregate_var: String,
        result_var: String,
        body: Vec<Atom>,
    ) -> Self {
        AggregateAtom {
            op,
            aggregate_var,
            result_var,
            body,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_term_creation() {
        let var = Term::var("X");
        assert!(var.is_variable());
        assert_eq!(var.as_variable(), Some("X"));

        let const_term = Term::constant(Value::Integer(42));
        assert!(const_term.is_constant());
        assert_eq!(const_term.as_constant(), Some(&Value::Integer(42)));
    }

    #[test]
    fn test_atom_creation() {
        let atom = Atom::new(
            "edge",
            vec![Term::var("X"), Term::constant(Value::string("alice"))],
        );

        assert_eq!(atom.predicate.as_ref(), "edge");
        assert_eq!(atom.arity(), 2);
        assert_eq!(atom.variables(), vec!["X"]);
        assert!(!atom.is_ground());
    }

    #[test]
    fn test_rule_safety() {
        // Safe rule: path(X, Y) :- edge(X, Y)
        let rule = Rule::new(
            Atom::new("path", vec![Term::var("X"), Term::var("Y")]),
            vec![Atom::new("edge", vec![Term::var("X"), Term::var("Y")])],
        );
        assert!(rule.is_safe());

        // Unsafe rule: path(X, Y) :- edge(Z, W)
        let unsafe_rule = Rule::new(
            Atom::new("path", vec![Term::var("X"), Term::var("Y")]),
            vec![Atom::new("edge", vec![Term::var("Z"), Term::var("W")])],
        );
        assert!(!unsafe_rule.is_safe());
    }

    #[test]
    fn test_substitution() {
        let mut sub = Substitution::new();
        sub.bind("X".to_string(), Value::Integer(42));
        sub.bind("Y".to_string(), Value::string("hello"));

        assert_eq!(sub.get("X"), Some(&Value::Integer(42)));
        assert_eq!(sub.get("Y"), Some(&Value::string("hello")));
        assert_eq!(sub.get("Z"), None);

        // Apply to term
        let var_term = Term::var("X");
        let applied = sub.apply_to_term(&var_term);
        assert_eq!(applied, Term::Constant(Value::Integer(42)));
    }

    #[test]
    fn test_substitution_merge() {
        let mut sub1 = Substitution::new();
        sub1.bind("X".to_string(), Value::Integer(1));

        let mut sub2 = Substitution::new();
        sub2.bind("Y".to_string(), Value::Integer(2));

        let merged = sub1.merge(&sub2).unwrap();
        assert_eq!(merged.get("X"), Some(&Value::Integer(1)));
        assert_eq!(merged.get("Y"), Some(&Value::Integer(2)));

        // Incompatible substitutions
        let mut sub3 = Substitution::new();
        sub3.bind("X".to_string(), Value::Integer(99));

        assert!(sub1.merge(&sub3).is_none());
    }
}
