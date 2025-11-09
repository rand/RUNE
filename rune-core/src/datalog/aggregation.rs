//! Aggregation support for Datalog
//!
//! Implements aggregation operations (count, sum, min, max, mean) over
//! sets of facts matching a pattern.

use super::types::{AggregateAtom, AggregateOp, Atom, Substitution};
use super::unification::unify_atom_with_fact;
use crate::facts::Fact;
use crate::types::Value;
use std::collections::HashSet;

/// Result of an aggregation operation
#[derive(Debug, Clone)]
pub struct AggregationResult {
    /// The aggregated value
    pub value: Value,
    /// Number of facts aggregated over
    pub count: usize,
}

/// Evaluate an aggregate atom against a set of facts
pub fn evaluate_aggregate(
    aggregate: &AggregateAtom,
    facts: &[Fact],
) -> Option<AggregationResult> {
    // Find all facts that match the body atoms
    let mut matching_values: Vec<Value> = Vec::new();

    // For each combination of facts that satisfies the body
    let all_substitutions = find_all_substitutions(&aggregate.body, facts);

    // Extract the aggregate variable value from each substitution
    for sub in &all_substitutions {
        if let Some(val) = sub.get(&aggregate.aggregate_var) {
            matching_values.push(val.clone());
        }
    }

    if matching_values.is_empty() {
        return None;
    }

    // Apply the aggregation operation
    let value = match aggregate.op {
        AggregateOp::Count => Value::Integer(matching_values.len() as i64),

        AggregateOp::Sum => {
            let mut sum: i64 = 0;
            for val in &matching_values {
                match val {
                    Value::Integer(i) => sum += i,
                    _ => return None, // Can only sum integers
                }
            }
            Value::Integer(sum)
        }

        AggregateOp::Min => {
            let mut min_val: Option<i64> = None;
            for val in &matching_values {
                match val {
                    Value::Integer(i) => {
                        min_val = Some(min_val.map_or(*i, |m| m.min(*i)));
                    }
                    _ => return None,
                }
            }
            Value::Integer(min_val?)
        }

        AggregateOp::Max => {
            let mut max_val: Option<i64> = None;
            for val in &matching_values {
                match val {
                    Value::Integer(i) => {
                        max_val = Some(max_val.map_or(*i, |m| m.max(*i)));
                    }
                    _ => return None,
                }
            }
            Value::Integer(max_val?)
        }

        AggregateOp::Mean => {
            let mut sum: i64 = 0;
            let count = matching_values.len() as i64;
            for val in &matching_values {
                match val {
                    Value::Integer(i) => sum += i,
                    _ => return None,
                }
            }
            Value::Integer(sum / count)
        }
    };

    Some(AggregationResult {
        value,
        count: matching_values.len(),
    })
}

/// Find all substitutions that satisfy a conjunction of atoms
fn find_all_substitutions(body: &[Atom], facts: &[Fact]) -> Vec<Substitution> {
    if body.is_empty() {
        return vec![Substitution::new()];
    }

    // Start with empty substitution
    let mut current_subs = vec![Substitution::new()];

    // Process each atom in the body
    for atom in body {
        let mut next_subs = Vec::new();

        for sub in current_subs {
            // Apply current substitution to atom
            let partial_atom = atom.apply_substitution(&sub);

            // Find all facts that unify with this atom
            for fact in facts {
                if let Some(new_bindings) = unify_atom_with_fact(&partial_atom, fact) {
                    // Try to merge substitutions
                    if let Some(merged) = sub.merge(&new_bindings) {
                        next_subs.push(merged);
                    }
                }
            }
        }

        current_subs = next_subs;

        // Early termination if no substitutions remain
        if current_subs.is_empty() {
            return vec![];
        }
    }

    // Remove duplicates
    let mut unique_subs = Vec::new();
    let mut seen = HashSet::new();

    for sub in current_subs {
        let key = format!("{:?}", sub.bindings());
        if seen.insert(key) {
            unique_subs.push(sub);
        }
    }

    unique_subs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datalog::types::Term;

    #[test]
    fn test_count_aggregation() {
        let facts = vec![
            Fact::binary("edge", Value::Integer(1), Value::Integer(2)),
            Fact::binary("edge", Value::Integer(2), Value::Integer(3)),
            Fact::binary("edge", Value::Integer(3), Value::Integer(4)),
        ];

        // count(X) where edge(X, Y)
        let aggregate = AggregateAtom::new(
            AggregateOp::Count,
            "X".to_string(),
            "Count".to_string(),
            vec![Atom::new("edge", vec![Term::var("X"), Term::var("Y")])],
        );

        let result = evaluate_aggregate(&aggregate, &facts).unwrap();
        assert_eq!(result.value, Value::Integer(3));
        assert_eq!(result.count, 3);
    }

    #[test]
    fn test_sum_aggregation() {
        let facts = vec![
            Fact::binary("score", Value::string("alice"), Value::Integer(10)),
            Fact::binary("score", Value::string("bob"), Value::Integer(20)),
            Fact::binary("score", Value::string("charlie"), Value::Integer(30)),
        ];

        // sum(Score) where score(Person, Score)
        let aggregate = AggregateAtom::new(
            AggregateOp::Sum,
            "Score".to_string(),
            "Total".to_string(),
            vec![Atom::new(
                "score",
                vec![Term::var("Person"), Term::var("Score")],
            )],
        );

        let result = evaluate_aggregate(&aggregate, &facts).unwrap();
        assert_eq!(result.value, Value::Integer(60));
    }

    #[test]
    fn test_min_max_aggregation() {
        let facts = vec![
            Fact::binary("value", Value::string("a"), Value::Integer(5)),
            Fact::binary("value", Value::string("b"), Value::Integer(10)),
            Fact::binary("value", Value::string("c"), Value::Integer(3)),
        ];

        // min(V) where value(_, V)
        let min_aggregate = AggregateAtom::new(
            AggregateOp::Min,
            "V".to_string(),
            "Min".to_string(),
            vec![Atom::new("value", vec![Term::var("_"), Term::var("V")])],
        );

        let min_result = evaluate_aggregate(&min_aggregate, &facts).unwrap();
        assert_eq!(min_result.value, Value::Integer(3));

        // max(V) where value(_, V)
        let max_aggregate = AggregateAtom::new(
            AggregateOp::Max,
            "V".to_string(),
            "Max".to_string(),
            vec![Atom::new("value", vec![Term::var("_"), Term::var("V")])],
        );

        let max_result = evaluate_aggregate(&max_aggregate, &facts).unwrap();
        assert_eq!(max_result.value, Value::Integer(10));
    }

    #[test]
    fn test_mean_aggregation() {
        let facts = vec![
            Fact::binary("score", Value::string("test1"), Value::Integer(10)),
            Fact::binary("score", Value::string("test2"), Value::Integer(20)),
            Fact::binary("score", Value::string("test3"), Value::Integer(30)),
        ];

        // mean(Score) where score(_, Score)
        let aggregate = AggregateAtom::new(
            AggregateOp::Mean,
            "Score".to_string(),
            "Avg".to_string(),
            vec![Atom::new(
                "score",
                vec![Term::var("_"), Term::var("Score")],
            )],
        );

        let result = evaluate_aggregate(&aggregate, &facts).unwrap();
        assert_eq!(result.value, Value::Integer(20)); // (10 + 20 + 30) / 3 = 20
    }
}
