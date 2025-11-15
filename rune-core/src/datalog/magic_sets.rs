//! Magic Sets transformation for goal-directed query optimization
//!
//! Transforms Datalog programs to evaluate queries more efficiently by
//! focusing computation only on facts that could contribute to the answer.
//!
//! ## Algorithm Overview
//!
//! Given a query like `path(a, X)`, instead of computing all paths,
//! Magic Sets transformation creates "magic" predicates that track
//! which facts are relevant to the query.
//!
//! ## Example Transformation
//!
//! Original rules:
//! ```datalog
//! path(X, Y) :- edge(X, Y).
//! path(X, Z) :- path(X, Y), edge(Y, Z).
//! ```
//!
//! Query: `path(a, ?)`
//!
//! Transformed rules:
//! ```datalog
//! magic_path(a).
//! magic_path(Y) :- magic_path(X), edge(X, Y).
//! path(X, Y) :- magic_path(X), edge(X, Y).
//! path(X, Z) :- magic_path(X), path(X, Y), edge(Y, Z).
//! ```
//!
//! This ensures only paths from 'a' are computed, not all possible paths.

use super::types::{Atom, Rule, Term};
use crate::types::Value;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

/// Query specification for Magic Sets transformation
#[derive(Debug, Clone)]
pub struct Query {
    /// The predicate being queried
    pub predicate: Arc<str>,
    /// Bound arguments (constants) in the query
    pub bound_args: Vec<Option<Value>>,
}

impl Query {
    /// Create a new query
    pub fn new(predicate: impl Into<Arc<str>>, bound_args: Vec<Option<Value>>) -> Self {
        Query {
            predicate: predicate.into(),
            bound_args,
        }
    }

    /// Create a query with all arguments unbound
    pub fn unbound(predicate: impl Into<Arc<str>>, arity: usize) -> Self {
        Query {
            predicate: predicate.into(),
            bound_args: vec![None; arity],
        }
    }

    /// Check if an argument position is bound
    pub fn is_bound(&self, index: usize) -> bool {
        index < self.bound_args.len() && self.bound_args[index].is_some()
    }

    /// Get the binding pattern as a string (for magic predicate naming)
    pub fn binding_pattern(&self) -> String {
        self.bound_args
            .iter()
            .map(|arg| if arg.is_some() { "b" } else { "f" })
            .collect()
    }
}

/// Magic Sets transformer
pub struct MagicSetsTransformer {
    /// Original rules
    rules: Vec<Rule>,
    /// Magic predicates generated
    magic_predicates: HashSet<Arc<str>>,
    /// Adorned predicates (predicate + binding pattern)
    adorned_predicates: HashMap<(Arc<str>, String), Arc<str>>,
}

impl MagicSetsTransformer {
    /// Create a new transformer
    pub fn new(rules: Vec<Rule>) -> Self {
        MagicSetsTransformer {
            rules,
            magic_predicates: HashSet::new(),
            adorned_predicates: HashMap::new(),
        }
    }

    /// Transform rules for a specific query
    pub fn transform(&mut self, query: &Query) -> Vec<Rule> {
        // Step 1: Generate adorned program
        let adorned_rules = self.generate_adorned_program(query);

        // Step 2: Generate magic rules
        let magic_rules = self.generate_magic_rules(&adorned_rules, query);

        // Step 3: Modify adorned rules to use magic predicates
        let modified_rules = self.add_magic_filters(&adorned_rules);

        // Step 4: Generate seed facts for the query
        let seed_facts = self.generate_seed_facts(query);

        // Combine all rules
        let mut result = Vec::new();
        result.extend(seed_facts);
        result.extend(magic_rules);
        result.extend(modified_rules);
        result
    }

    /// Generate adorned program based on query binding pattern
    fn generate_adorned_program(&mut self, query: &Query) -> Vec<Rule> {
        let mut adorned_rules = Vec::new();
        let mut processed = HashSet::new();
        let mut queue = VecDeque::new();

        // Start with query predicate and its binding pattern
        queue.push_back((query.predicate.clone(), query.binding_pattern()));

        while let Some((pred, pattern)) = queue.pop_front() {
            if processed.contains(&(pred.clone(), pattern.clone())) {
                continue;
            }
            processed.insert((pred.clone(), pattern.clone()));

            // Find rules with this predicate in the head
            let matching_rules: Vec<_> = self
                .rules
                .iter()
                .filter(|rule| rule.head.predicate == pred)
                .cloned()
                .collect();

            for rule in matching_rules {
                // Create adorned version of this rule
                let adorned_rule = self.adorn_rule(&rule, &pattern);

                // Add body predicates to queue with their binding patterns
                for body_atom in &adorned_rule.body {
                    if !body_atom.negated {
                        let body_pattern =
                            self.compute_binding_pattern(body_atom, &adorned_rule, &pattern);
                        queue.push_back((body_atom.predicate.clone(), body_pattern));
                    }
                }

                adorned_rules.push(adorned_rule);
            }
        }

        adorned_rules
    }

    /// Adorn a rule based on the head's binding pattern
    fn adorn_rule(&mut self, rule: &Rule, head_pattern: &str) -> Rule {
        // Create adorned head predicate
        let adorned_head_pred = self.get_adorned_predicate(&rule.head.predicate, head_pattern);

        let adorned_head = Atom {
            predicate: adorned_head_pred,
            terms: rule.head.terms.clone(),
            negated: false,
        };

        // Body atoms remain the same for now (will be adorned later)
        Rule::new(adorned_head, rule.body.clone())
    }

    /// Compute binding pattern for a body atom
    fn compute_binding_pattern(&self, atom: &Atom, rule: &Rule, head_pattern: &str) -> String {
        let mut pattern = String::new();

        for term in &atom.terms {
            let is_bound = match term {
                Term::Constant(_) => true,
                Term::Variable(var) => {
                    // Check if this variable appears in a bound position in the head
                    // or in earlier body atoms
                    self.is_variable_bound(var, rule, head_pattern, atom)
                }
            };
            pattern.push(if is_bound { 'b' } else { 'f' });
        }

        pattern
    }

    /// Check if a variable is bound at a given point in rule evaluation
    fn is_variable_bound(
        &self,
        var: &str,
        rule: &Rule,
        head_pattern: &str,
        current_atom: &Atom,
    ) -> bool {
        // Check if bound in head
        for (i, term) in rule.head.terms.iter().enumerate() {
            if let Term::Variable(v) = term {
                if v == var && i < head_pattern.len() {
                    if &head_pattern[i..i + 1] == "b" {
                        return true;
                    }
                }
            }
        }

        // Check if bound in earlier body atoms (left-to-right evaluation)
        for body_atom in &rule.body {
            if body_atom == current_atom {
                break; // Stop at current atom
            }

            for term in &body_atom.terms {
                if let Term::Variable(v) = term {
                    if v == var {
                        return true; // Bound by earlier atom
                    }
                }
            }
        }

        false
    }

    /// Get or create an adorned predicate name
    fn get_adorned_predicate(&mut self, pred: &Arc<str>, pattern: &str) -> Arc<str> {
        let key = (pred.clone(), pattern.to_string());

        if let Some(adorned) = self.adorned_predicates.get(&key) {
            adorned.clone()
        } else {
            let adorned_name = format!("{}_{}", pred, pattern);
            let adorned: Arc<str> = Arc::from(adorned_name);
            self.adorned_predicates.insert(key, adorned.clone());
            adorned
        }
    }

    /// Generate magic rules from adorned rules
    fn generate_magic_rules(&mut self, adorned_rules: &[Rule], query: &Query) -> Vec<Rule> {
        let mut magic_rules = Vec::new();

        for rule in adorned_rules {
            // For each adorned rule, generate magic rules for its body predicates
            let mut cumulative_bindings = Vec::new();

            // Start with magic predicate for head
            let head_magic = self.get_magic_predicate(&rule.head.predicate);

            for (i, body_atom) in rule.body.iter().enumerate() {
                if body_atom.negated {
                    continue; // Skip negated atoms for magic generation
                }

                let body_magic = self.get_magic_predicate(&body_atom.predicate);

                // Create magic rule: magic_body(...) :- magic_head(...), earlier_atoms
                let mut magic_body = Vec::new();

                // Add magic predicate for head (with bound arguments)
                let head_magic_atom = self.create_magic_atom(&rule.head, &head_magic);
                magic_body.push(head_magic_atom);

                // Add earlier body atoms
                magic_body.extend(cumulative_bindings.clone());

                // Create magic head for this body predicate
                let magic_head = self.create_magic_atom(body_atom, &body_magic);

                // Only create rule if it's not trivial
                if !magic_body.is_empty() {
                    magic_rules.push(Rule::new(magic_head, magic_body));
                }

                cumulative_bindings.push(body_atom.clone());
            }
        }

        // Deduplicate magic rules
        let mut unique_rules = Vec::new();
        let mut seen = HashSet::new();
        for rule in magic_rules {
            let key = format!("{}", rule);
            if seen.insert(key) {
                unique_rules.push(rule);
            }
        }

        unique_rules
    }

    /// Get or create a magic predicate name
    fn get_magic_predicate(&mut self, adorned_pred: &Arc<str>) -> Arc<str> {
        let magic_name = format!("magic_{}", adorned_pred);
        let magic_pred: Arc<str> = Arc::from(magic_name);
        self.magic_predicates.insert(magic_pred.clone());
        magic_pred
    }

    /// Create a magic atom from an adorned atom
    fn create_magic_atom(&self, atom: &Atom, magic_pred: &Arc<str>) -> Atom {
        // Magic atoms only include bound arguments
        let mut magic_terms = Vec::new();

        // Extract binding pattern from adorned predicate name
        let pattern = atom.predicate.rsplit('_').next().unwrap_or("");

        for (i, term) in atom.terms.iter().enumerate() {
            if i < pattern.len() && &pattern[i..i + 1] == "b" {
                magic_terms.push(term.clone());
            }
        }

        Atom::new(magic_pred.as_ref(), magic_terms)
    }

    /// Add magic filters to adorned rules
    fn add_magic_filters(&mut self, adorned_rules: &[Rule]) -> Vec<Rule> {
        let mut result = Vec::new();
        for rule in adorned_rules {
            let magic_pred = self.get_magic_predicate(&rule.head.predicate);
            let magic_atom = self.create_magic_atom(&rule.head, &magic_pred);

            // Add magic atom as first body atom
            let mut new_body = vec![magic_atom];
            new_body.extend(rule.body.clone());

            result.push(Rule::new(rule.head.clone(), new_body));
        }
        result
    }

    /// Generate seed facts for the query
    fn generate_seed_facts(&mut self, query: &Query) -> Vec<Rule> {
        let adorned_pred = self.get_adorned_predicate(&query.predicate, &query.binding_pattern());
        let magic_pred = self.get_magic_predicate(&adorned_pred);

        // Create seed fact with bound arguments
        let mut seed_terms = Vec::new();
        for arg in &query.bound_args {
            if let Some(value) = arg {
                seed_terms.push(Term::Constant(value.clone()));
            }
        }

        if seed_terms.is_empty() {
            return Vec::new(); // No seed facts for fully unbound queries
        }

        vec![Rule::fact(Atom::new(magic_pred.as_ref(), seed_terms))]
    }

    /// Get the transformed rules
    pub fn get_transformed_rules(&self) -> Vec<Rule> {
        self.rules.clone()
    }

    /// Check if a predicate is a magic predicate
    pub fn is_magic_predicate(&self, pred: &str) -> bool {
        self.magic_predicates.iter().any(|p| p.as_ref() == pred)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datalog::types::Term;

    #[test]
    fn test_query_binding_pattern() {
        // Fully bound query
        let query = Query::new(
            "path",
            vec![
                Some(Value::String(Arc::from("a"))),
                Some(Value::String(Arc::from("b"))),
            ],
        );
        assert_eq!(query.binding_pattern(), "bb");

        // Partially bound query
        let query = Query::new("path", vec![Some(Value::String(Arc::from("a"))), None]);
        assert_eq!(query.binding_pattern(), "bf");

        // Unbound query
        let query = Query::unbound("path", 2);
        assert_eq!(query.binding_pattern(), "ff");
    }

    #[test]
    fn test_simple_magic_sets_transformation() {
        // Create simple transitive closure rules
        let rules = vec![
            Rule::new(
                Atom::new("path", vec![Term::var("X"), Term::var("Y")]),
                vec![Atom::new("edge", vec![Term::var("X"), Term::var("Y")])],
            ),
            Rule::new(
                Atom::new("path", vec![Term::var("X"), Term::var("Z")]),
                vec![
                    Atom::new("path", vec![Term::var("X"), Term::var("Y")]),
                    Atom::new("edge", vec![Term::var("Y"), Term::var("Z")]),
                ],
            ),
        ];

        // Query: path(a, ?)
        let query = Query::new("path", vec![Some(Value::String(Arc::from("a"))), None]);

        let mut transformer = MagicSetsTransformer::new(rules);
        let transformed = transformer.transform(&query);

        // Check that magic predicates were generated
        assert!(!transformer.magic_predicates.is_empty());

        // Check that we have seed facts
        let seed_facts: Vec<_> = transformed.iter().filter(|r| r.is_fact()).collect();
        assert!(!seed_facts.is_empty());

        // Check that magic predicates appear in transformed rules
        let has_magic = transformed.iter().any(|rule| {
            rule.body
                .iter()
                .any(|atom| transformer.is_magic_predicate(atom.predicate.as_ref()))
        });
        assert!(has_magic);
    }

    #[test]
    fn test_adorned_predicate_generation() {
        let rules = vec![Rule::new(
            Atom::new("ancestor", vec![Term::var("X"), Term::var("Y")]),
            vec![Atom::new("parent", vec![Term::var("X"), Term::var("Y")])],
        )];

        let mut transformer = MagicSetsTransformer::new(rules);

        let adorned = transformer.get_adorned_predicate(&Arc::from("ancestor"), "bf");
        assert_eq!(adorned.as_ref(), "ancestor_bf");

        let adorned = transformer.get_adorned_predicate(&Arc::from("parent"), "bb");
        assert_eq!(adorned.as_ref(), "parent_bb");
    }

    #[test]
    fn test_magic_predicate_generation() {
        let rules = vec![];
        let mut transformer = MagicSetsTransformer::new(rules);

        let adorned = Arc::from("path_bf");
        let magic = transformer.get_magic_predicate(&adorned);
        assert_eq!(magic.as_ref(), "magic_path_bf");
        assert!(transformer.is_magic_predicate("magic_path_bf"));
    }

    #[test]
    fn test_no_transformation_for_unbound_query() {
        let rules = vec![Rule::new(
            Atom::new("node", vec![Term::var("X")]),
            vec![Atom::new("edge", vec![Term::var("X"), Term::var("_")])],
        )];

        let query = Query::unbound("node", 1);

        let mut transformer = MagicSetsTransformer::new(rules);
        let transformed = transformer.transform(&query);

        // Should have minimal transformation for fully unbound query
        let seed_facts: Vec<_> = transformed.iter().filter(|r| r.is_fact()).collect();
        assert!(seed_facts.is_empty()); // No seed facts for unbound query
    }

    #[test]
    fn test_negation_handling() {
        // Rules with negation
        let rules = vec![Rule::new(
            Atom::new("unreachable", vec![Term::var("X")]),
            vec![
                Atom::new("node", vec![Term::var("X")]),
                Atom::negated(
                    "path",
                    vec![
                        Term::constant(Value::String(Arc::from("root"))),
                        Term::var("X"),
                    ],
                ),
            ],
        )];

        let query = Query::new("unreachable", vec![None]);

        let mut transformer = MagicSetsTransformer::new(rules);
        let transformed = transformer.transform(&query);

        // Check that transformation handles negation correctly
        // Negated atoms shouldn't generate magic rules
        let magic_for_negated = transformed
            .iter()
            .any(|rule| rule.head.predicate.as_ref().contains("magic_path"));
        assert!(
            !magic_for_negated,
            "Should not generate magic rules for negated atoms"
        );
    }
}
