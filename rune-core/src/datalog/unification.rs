//! Unification algorithm for Datalog
//!
//! Implements the unification algorithm for matching atoms with facts
//! and computing variable substitutions.

use super::types::{Atom, Substitution, Term};
use crate::facts::Fact;
use crate::types::Value;

/// Unify two terms, producing a substitution if successful
pub fn unify_terms(term1: &Term, term2: &Term, sub: &mut Substitution) -> bool {
    match (term1, term2) {
        // Variable-Variable
        (Term::Variable(v1), Term::Variable(v2)) => {
            // Check if both are already bound
            match (sub.get(v1), sub.get(v2)) {
                (Some(val1), Some(val2)) => val1 == val2,
                (Some(val), None) => {
                    sub.bind(v2.clone(), val.clone());
                    true
                }
                (None, Some(val)) => {
                    sub.bind(v1.clone(), val.clone());
                    true
                }
                (None, None) => {
                    // Bind one to the other (canonicalize to v1)
                    sub.bind(v2.clone(), Value::string(v1));
                    true
                }
            }
        }

        // Variable-Constant
        (Term::Variable(var), Term::Constant(val)) | (Term::Constant(val), Term::Variable(var)) => {
            if let Some(existing) = sub.get(var) {
                existing == val
            } else {
                sub.bind(var.clone(), val.clone());
                true
            }
        }

        // Constant-Constant
        (Term::Constant(val1), Term::Constant(val2)) => val1 == val2,
    }
}

/// Unify an atom with a fact, producing a substitution if successful
pub fn unify_atom_with_fact(atom: &Atom, fact: &Fact) -> Option<Substitution> {
    // Check predicate match
    if atom.predicate != fact.predicate {
        return None;
    }

    // Check arity
    if atom.terms.len() != fact.args.len() {
        return None;
    }

    let mut sub = Substitution::new();

    // Unify each term with corresponding fact argument
    for (term, fact_arg) in atom.terms.iter().zip(fact.args.iter()) {
        if !unify_terms(term, &Term::Constant(fact_arg.clone()), &mut sub) {
            return None;
        }
    }

    Some(sub)
}

/// Unify two atoms, producing a substitution if successful
pub fn unify_atoms(atom1: &Atom, atom2: &Atom) -> Option<Substitution> {
    // Check predicate match
    if atom1.predicate != atom2.predicate {
        return None;
    }

    // Check arity
    if atom1.terms.len() != atom2.terms.len() {
        return None;
    }

    let mut sub = Substitution::new();

    // Unify each pair of terms
    for (term1, term2) in atom1.terms.iter().zip(atom2.terms.iter()) {
        if !unify_terms(term1, term2, &mut sub) {
            return None;
        }
    }

    Some(sub)
}

/// Find all facts that unify with an atom
pub fn find_matching_facts<'a>(atom: &Atom, facts: &'a [Fact]) -> Vec<(&'a Fact, Substitution)> {
    facts
        .iter()
        .filter(|fact| fact.predicate == atom.predicate)
        .filter_map(|fact| unify_atom_with_fact(atom, fact).map(|sub| (fact, sub)))
        .collect()
}

/// Apply a substitution to an atom to produce a ground atom (fact)
pub fn ground_atom(atom: &Atom, sub: &Substitution) -> Option<Fact> {
    let grounded_atom = atom.apply_substitution(sub);

    // Check if all terms are ground
    if !grounded_atom.is_ground() {
        return None;
    }

    // Convert to Fact
    let args: Vec<Value> = grounded_atom
        .terms
        .iter()
        .filter_map(|t| t.as_constant().cloned())
        .collect();

    Some(Fact::new(
        grounded_atom.predicate.as_ref().to_string(),
        args,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unify_terms_constant_constant() {
        let mut sub = Substitution::new();
        let t1 = Term::Constant(Value::Integer(42));
        let t2 = Term::Constant(Value::Integer(42));

        assert!(unify_terms(&t1, &t2, &mut sub));
        assert!(sub.is_empty());
    }

    #[test]
    fn test_unify_terms_variable_constant() {
        let mut sub = Substitution::new();
        let var = Term::var("X");
        let const_term = Term::Constant(Value::Integer(42));

        assert!(unify_terms(&var, &const_term, &mut sub));
        assert_eq!(sub.get("X"), Some(&Value::Integer(42)));
    }

    #[test]
    fn test_unify_terms_variable_variable() {
        let mut sub = Substitution::new();
        let v1 = Term::var("X");
        let v2 = Term::var("Y");

        assert!(unify_terms(&v1, &v2, &mut sub));
        // One should be bound to the other
        assert!(!sub.is_empty());
    }

    #[test]
    fn test_unify_atom_with_fact() {
        let atom = Atom::new(
            "edge",
            vec![Term::var("X"), Term::constant(Value::Integer(2))],
        );
        let fact = Fact::binary("edge", Value::Integer(1), Value::Integer(2));

        let sub = unify_atom_with_fact(&atom, &fact).unwrap();
        assert_eq!(sub.get("X"), Some(&Value::Integer(1)));
    }

    #[test]
    fn test_unify_atom_with_fact_fail() {
        let atom = Atom::new(
            "edge",
            vec![Term::var("X"), Term::constant(Value::Integer(3))],
        );
        let fact = Fact::binary("edge", Value::Integer(1), Value::Integer(2));

        assert!(unify_atom_with_fact(&atom, &fact).is_none());
    }

    #[test]
    fn test_find_matching_facts() {
        let facts = vec![
            Fact::binary("edge", Value::Integer(1), Value::Integer(2)),
            Fact::binary("edge", Value::Integer(2), Value::Integer(3)),
            Fact::binary("edge", Value::Integer(1), Value::Integer(4)),
        ];

        let atom = Atom::new(
            "edge",
            vec![Term::constant(Value::Integer(1)), Term::var("Y")],
        );

        let matches = find_matching_facts(&atom, &facts);
        assert_eq!(matches.len(), 2);

        // Check substitutions
        let y_values: Vec<_> = matches.iter().filter_map(|(_, sub)| sub.get("Y")).collect();
        assert!(y_values.contains(&&Value::Integer(2)));
        assert!(y_values.contains(&&Value::Integer(4)));
    }

    #[test]
    fn test_ground_atom() {
        let atom = Atom::new("path", vec![Term::var("X"), Term::var("Y")]);

        let mut sub = Substitution::new();
        sub.bind("X".to_string(), Value::Integer(1));
        sub.bind("Y".to_string(), Value::Integer(2));

        let fact = ground_atom(&atom, &sub).unwrap();
        assert_eq!(fact.predicate.as_ref(), "path");
        assert_eq!(fact.args[0], Value::Integer(1));
        assert_eq!(fact.args[1], Value::Integer(2));
    }

    #[test]
    fn test_ground_atom_incomplete() {
        let atom = Atom::new("path", vec![Term::var("X"), Term::var("Y")]);

        let mut sub = Substitution::new();
        sub.bind("X".to_string(), Value::Integer(1));
        // Y not bound

        assert!(ground_atom(&atom, &sub).is_none());
    }
}
