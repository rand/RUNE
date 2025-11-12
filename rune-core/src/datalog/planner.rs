//! Query Planner with Cost-Based Optimization
//!
//! Analyzes Datalog queries and selects optimal evaluation strategies by:
//! - Analyzing query structure (join patterns, selectivity)
//! - Maintaining statistics about fact distributions
//! - Estimating costs for different backends
//! - Reordering joins for minimal intermediate results
//! - Selecting appropriate backends (Hash, Trie, WCOJ)
//!
//! ## Cost Model
//!
//! Each backend has different strengths:
//! - **Hash**: Fast for simple lookups, O(1) per fact
//! - **Trie**: Efficient for prefix queries, O(log n) per level
//! - **WCOJ**: Optimal for multi-way joins, O(n^(k/(k+1)))
//!
//! ## Join Ordering
//!
//! Uses dynamic programming to find optimal join order:
//! 1. Start with most selective predicates
//! 2. Minimize intermediate result sizes
//! 3. Prefer bound variables over unbound
//!
//! ## Example
//!
//! ```rust
//! use rune_core::datalog::planner::QueryPlanner;
//! use rune_core::datalog::types::Rule;
//! use rune_core::facts::FactStore;
//! use std::sync::Arc;
//!
//! let fact_store = Arc::new(FactStore::new());
//! let mut planner = QueryPlanner::new(fact_store);
//!
//! // Analyze rule and get optimal plan
//! let rule = /* ... */;
//! let plan = planner.plan_rule(&rule);
//!
//! println!("Using backend: {:?}", plan.backend);
//! println!("Join order: {:?}", plan.join_order);
//! println!("Estimated cost: {}", plan.estimated_cost);
//! ```

use crate::datalog::backends::BackendType;
use crate::datalog::types::{Atom, Rule, Term};
use crate::facts::FactStore;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Statistics about predicate distributions in the fact store
#[derive(Debug, Clone)]
pub struct PredicateStats {
    /// Predicate name
    pub predicate: Arc<str>,
    /// Total number of facts with this predicate
    pub count: usize,
    /// Average number of arguments
    pub arity: usize,
    /// Estimated selectivity (0.0 = highly selective, 1.0 = not selective)
    pub selectivity: f64,
}

impl PredicateStats {
    /// Create new predicate statistics
    pub fn new(predicate: Arc<str>, count: usize, arity: usize) -> Self {
        // Estimate selectivity based on count (lower count = more selective)
        let selectivity = if count == 0 {
            1.0
        } else {
            (count as f64).log10() / 10.0
        };

        PredicateStats {
            predicate,
            count,
            arity,
            selectivity: selectivity.min(1.0),
        }
    }
}

/// Analysis of a single atom in a rule body
#[derive(Debug, Clone)]
pub struct AtomAnalysis {
    /// The atom being analyzed
    pub atom: Atom,
    /// Position in the original rule body
    pub position: usize,
    /// Number of bound variables (from earlier atoms)
    pub bound_vars: usize,
    /// Number of unbound variables (new in this atom)
    pub unbound_vars: usize,
    /// Estimated output cardinality
    pub estimated_cardinality: usize,
    /// Selectivity of this atom
    pub selectivity: f64,
}

/// Query execution plan for a rule
#[derive(Debug, Clone)]
pub struct QueryPlan {
    /// Original rule
    pub rule: Rule,
    /// Recommended backend for execution
    pub backend: BackendType,
    /// Optimal join order (indices into rule.body)
    pub join_order: Vec<usize>,
    /// Estimated total cost
    pub estimated_cost: f64,
    /// Analysis of each atom
    pub atom_analyses: Vec<AtomAnalysis>,
    /// Explanation of plan decisions
    pub explanation: String,
}

impl QueryPlan {
    /// Format the query plan as a human-readable string
    pub fn format(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("Query Plan for: {}\n", self.rule));
        output.push_str(&format!("Backend: {:?}\n", self.backend));
        output.push_str(&format!("Estimated Cost: {:.2}\n", self.estimated_cost));
        output.push_str("Join Order:\n");
        for (i, &atom_idx) in self.join_order.iter().enumerate() {
            let atom = &self.rule.body[atom_idx];
            let analysis = &self.atom_analyses[atom_idx];
            output.push_str(&format!(
                "  {}. {} (bound: {}, unbound: {}, est. card: {})\n",
                i + 1,
                atom,
                analysis.bound_vars,
                analysis.unbound_vars,
                analysis.estimated_cardinality
            ));
        }
        output.push_str(&format!("\nExplanation: {}\n", self.explanation));
        output
    }
}

/// Query planner with cost-based optimization
pub struct QueryPlanner {
    /// Reference to fact store for statistics
    fact_store: Arc<FactStore>,
    /// Cached predicate statistics
    predicate_stats: HashMap<Arc<str>, PredicateStats>,
    /// Whether to enable join reordering
    enable_join_reordering: bool,
    /// Whether to use WCOJ for multi-way joins
    enable_wcoj: bool,
}

impl QueryPlanner {
    /// Create a new query planner
    pub fn new(fact_store: Arc<FactStore>) -> Self {
        let mut planner = QueryPlanner {
            fact_store,
            predicate_stats: HashMap::new(),
            enable_join_reordering: true,
            enable_wcoj: true,
        };
        planner.update_statistics();
        planner
    }

    /// Update statistics from the fact store
    pub fn update_statistics(&mut self) {
        self.predicate_stats.clear();

        let all_facts = self.fact_store.all_facts();
        let mut predicate_counts: HashMap<Arc<str>, (usize, usize)> = HashMap::new();

        for fact in all_facts.iter() {
            let entry = predicate_counts
                .entry(fact.predicate.clone())
                .or_insert((0, fact.args.len()));
            entry.0 += 1;
        }

        for (predicate, (count, arity)) in predicate_counts {
            let stats = PredicateStats::new(predicate.clone(), count, arity);
            self.predicate_stats.insert(predicate, stats);
        }
    }

    /// Plan execution for a single rule
    pub fn plan_rule(&self, rule: &Rule) -> QueryPlan {
        // Analyze each atom in the rule body
        let atom_analyses = self.analyze_atoms(rule);

        // Determine optimal join order
        let join_order = if self.enable_join_reordering {
            self.compute_optimal_join_order(&atom_analyses)
        } else {
            (0..rule.body.len()).collect()
        };

        // Select backend based on query characteristics
        let backend = self.select_backend(rule, &atom_analyses);

        // Estimate total execution cost
        let estimated_cost = self.estimate_cost(rule, &atom_analyses, &join_order, &backend);

        // Generate explanation
        let explanation = self.generate_explanation(rule, &atom_analyses, &backend);

        QueryPlan {
            rule: rule.clone(),
            backend,
            join_order,
            estimated_cost,
            atom_analyses,
            explanation,
        }
    }

    /// Analyze all atoms in a rule body
    fn analyze_atoms(&self, rule: &Rule) -> Vec<AtomAnalysis> {
        let mut analyses = Vec::new();

        // Analyze each atom independently (don't assume any ordering)
        for (position, atom) in rule.body.iter().enumerate() {
            // Count total variables in this atom
            let mut unbound_vars = 0;
            for term in &atom.terms {
                if let Term::Variable(_) = term {
                    unbound_vars += 1;
                }
            }

            // Get statistics for this predicate
            let stats = self
                .predicate_stats
                .get(&atom.predicate)
                .cloned()
                .unwrap_or_else(|| PredicateStats::new(atom.predicate.clone(), 0, atom.terms.len()));

            // Initial cardinality (no variables bound yet)
            let estimated_cardinality = stats.count;

            analyses.push(AtomAnalysis {
                atom: atom.clone(),
                position,
                bound_vars: 0, // Will be computed dynamically during join ordering
                unbound_vars,
                estimated_cardinality,
                selectivity: stats.selectivity,
            });
        }

        analyses
    }

    /// Compute optimal join order using greedy algorithm
    fn compute_optimal_join_order(&self, analyses: &[AtomAnalysis]) -> Vec<usize> {
        if analyses.is_empty() {
            return Vec::new();
        }

        let mut remaining: HashSet<usize> = (0..analyses.len()).collect();
        let mut ordered = Vec::new();
        let mut bound_variables = HashSet::new();

        // Greedy selection: pick atom with best cost/benefit ratio
        while !remaining.is_empty() {
            let mut best_idx = None;
            let mut best_score = f64::MAX;

            for &idx in &remaining {
                let analysis = &analyses[idx];

                // Count how many variables this atom would bind
                let mut newly_bound = 0;
                for term in &analysis.atom.terms {
                    if let Term::Variable(var) = term {
                        if !bound_variables.contains(var) {
                            newly_bound += 1;
                        }
                    }
                }

                // Score = estimated_cardinality / (1 + newly_bound + bound_vars)
                // Lower is better (prefer selective atoms that bind many variables)
                let score = analysis.estimated_cardinality as f64
                    / (1.0 + newly_bound as f64 + analysis.bound_vars as f64);

                if score < best_score {
                    best_score = score;
                    best_idx = Some(idx);
                }
            }

            if let Some(idx) = best_idx {
                remaining.remove(&idx);
                ordered.push(idx);

                // Update bound variables
                for term in &analyses[idx].atom.terms {
                    if let Term::Variable(var) = term {
                        bound_variables.insert(var.clone());
                    }
                }
            } else {
                break;
            }
        }

        ordered
    }

    /// Select optimal backend for the rule
    fn select_backend(&self, rule: &Rule, analyses: &[AtomAnalysis]) -> BackendType {
        let body_len = rule.body.len();

        // For single-atom queries, use hash backend (fastest)
        if body_len == 1 {
            return BackendType::Hash;
        }

        // Check if this is a good candidate for WCOJ
        if self.enable_wcoj && body_len >= 3 {
            // WCOJ is good for triangle/clique patterns and highly connected queries
            let total_vars = self.count_unique_variables(rule);
            let shared_vars = self.count_shared_variables(rule);

            // If many variables are shared across atoms, WCOJ is beneficial
            if shared_vars as f64 / total_vars as f64 > 0.5 {
                return BackendType::WCOJ;
            }
        }

        // Check if trie backend is beneficial (hierarchical queries)
        if self.is_hierarchical_query(analyses) {
            return BackendType::Trie;
        }

        // Default to hash backend for most queries
        BackendType::Hash
    }

    /// Count unique variables in a rule
    fn count_unique_variables(&self, rule: &Rule) -> usize {
        let mut vars = HashSet::new();
        for atom in &rule.body {
            for term in &atom.terms {
                if let Term::Variable(var) = term {
                    vars.insert(var);
                }
            }
        }
        vars.len()
    }

    /// Count variables shared between multiple atoms
    fn count_shared_variables(&self, rule: &Rule) -> usize {
        let mut var_occurrences: HashMap<String, usize> = HashMap::new();

        for atom in &rule.body {
            for term in &atom.terms {
                if let Term::Variable(var) = term {
                    *var_occurrences.entry(var.clone()).or_insert(0) += 1;
                }
            }
        }

        var_occurrences.values().filter(|&&count| count > 1).count()
    }

    /// Check if query has hierarchical pattern (good for trie)
    fn is_hierarchical_query(&self, analyses: &[AtomAnalysis]) -> bool {
        // Hierarchical if atoms progressively add one variable
        for i in 1..analyses.len() {
            if analyses[i].unbound_vars > 1 {
                return false;
            }
        }
        analyses.len() > 2
    }

    /// Estimate execution cost for a plan
    fn estimate_cost(
        &self,
        _rule: &Rule,
        analyses: &[AtomAnalysis],
        join_order: &[usize],
        backend: &BackendType,
    ) -> f64 {
        let mut total_cost = 0.0;
        let mut intermediate_size = 1.0;

        for &idx in join_order {
            let analysis = &analyses[idx];

            // Cost depends on backend
            let lookup_cost = match backend {
                BackendType::Hash => 1.0,                                // O(1)
                BackendType::Trie => analysis.atom.terms.len() as f64,  // O(log n) per level
                BackendType::WCOJ => 0.5,                                // Amortized cost
                _ => 1.0,
            };

            // Cost = intermediate_size * lookup_cost * output_cardinality
            let step_cost =
                intermediate_size * lookup_cost * (analysis.estimated_cardinality as f64);
            total_cost += step_cost;

            // Update intermediate result size
            intermediate_size *= analysis.estimated_cardinality as f64;
        }

        total_cost
    }

    /// Generate human-readable explanation of plan
    fn generate_explanation(
        &self,
        rule: &Rule,
        analyses: &[AtomAnalysis],
        backend: &BackendType,
    ) -> String {
        let mut parts = Vec::new();

        // Backend selection rationale
        match backend {
            BackendType::Hash => {
                parts.push("Using hash backend for fast lookups".to_string());
            }
            BackendType::Trie => {
                parts.push("Using trie backend for hierarchical query pattern".to_string());
            }
            BackendType::WCOJ => {
                let shared_vars = self.count_shared_variables(rule);
                parts.push(format!(
                    "Using WCOJ for multi-way join with {} shared variables",
                    shared_vars
                ));
            }
            _ => {
                parts.push(format!("Using {:?} backend", backend));
            }
        }

        // Join ordering rationale
        if analyses.len() > 1 {
            let most_selective = analyses
                .iter()
                .min_by(|a, b| {
                    a.estimated_cardinality
                        .partial_cmp(&b.estimated_cardinality)
                        .unwrap()
                })
                .unwrap();

            parts.push(format!(
                "Starting with most selective atom: {} (est. {} results)",
                most_selective.atom, most_selective.estimated_cardinality
            ));
        }

        parts.join(". ")
    }

    /// Enable or disable join reordering
    pub fn set_join_reordering(&mut self, enabled: bool) {
        self.enable_join_reordering = enabled;
    }

    /// Enable or disable WCOJ
    pub fn set_wcoj(&mut self, enabled: bool) {
        self.enable_wcoj = enabled;
    }

    /// Get statistics for a predicate
    pub fn get_predicate_stats(&self, predicate: &str) -> Option<&PredicateStats> {
        self.predicate_stats.get(predicate)
    }

    /// Get all predicate statistics
    pub fn all_stats(&self) -> &HashMap<Arc<str>, PredicateStats> {
        &self.predicate_stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datalog::types::Atom;
    use crate::facts::Fact;
    use crate::types::Value;

    fn test_fact(pred: &str, args: Vec<i64>) -> Fact {
        Fact::new(
            pred.to_string(),
            args.into_iter().map(Value::Integer).collect(),
        )
    }

    fn test_atom(pred: &str, vars: Vec<&str>) -> Atom {
        Atom {
            predicate: pred.to_string().into(),
            terms: vars.into_iter().map(|v| Term::Variable(v.to_string())).collect(),
            negated: false,
        }
    }

    fn test_rule(head: Atom, body: Vec<Atom>) -> Rule {
        Rule {
            head,
            body,
            stratum: 0,
        }
    }

    #[test]
    fn test_planner_creation() {
        let fact_store = Arc::new(FactStore::new());
        let planner = QueryPlanner::new(fact_store);

        assert_eq!(planner.predicate_stats.len(), 0);
        assert!(planner.enable_join_reordering);
        assert!(planner.enable_wcoj);
    }

    #[test]
    fn test_statistics_update() {
        let fact_store = Arc::new(FactStore::new());
        fact_store.add_fact(test_fact("edge", vec![1, 2]));
        fact_store.add_fact(test_fact("edge", vec![2, 3]));
        fact_store.add_fact(test_fact("node", vec![1]));

        let mut planner = QueryPlanner::new(fact_store);
        planner.update_statistics();

        assert_eq!(planner.predicate_stats.len(), 2);
        assert_eq!(planner.predicate_stats.get("edge").unwrap().count, 2);
        assert_eq!(planner.predicate_stats.get("node").unwrap().count, 1);
    }

    #[test]
    fn test_single_atom_uses_hash() {
        let fact_store = Arc::new(FactStore::new());
        fact_store.add_fact(test_fact("edge", vec![1, 2]));

        let planner = QueryPlanner::new(fact_store);

        let rule = test_rule(
            test_atom("result", vec!["X", "Y"]),
            vec![test_atom("edge", vec!["X", "Y"])],
        );

        let plan = planner.plan_rule(&rule);
        assert_eq!(plan.backend, BackendType::Hash);
    }

    #[test]
    fn test_join_ordering() {
        let fact_store = Arc::new(FactStore::new());
        // Add many edge facts (less selective)
        for i in 0..100 {
            fact_store.add_fact(test_fact("edge", vec![i, i + 1]));
        }
        // Add few node facts (more selective)
        for i in 0..10 {
            fact_store.add_fact(test_fact("node", vec![i]));
        }

        let mut planner = QueryPlanner::new(fact_store);
        planner.update_statistics();

        // Rule: result(X, Y) :- node(X), edge(X, Y)
        let rule = test_rule(
            test_atom("result", vec!["X", "Y"]),
            vec![
                test_atom("node", vec!["X"]),
                test_atom("edge", vec!["X", "Y"]),
            ],
        );

        let plan = planner.plan_rule(&rule);

        // Should start with more selective atom (node)
        assert_eq!(plan.join_order[0], 0); // node(X) is index 0
    }

    #[test]
    fn test_wcoj_selection_for_triangle() {
        let fact_store = Arc::new(FactStore::new());
        for i in 0..10 {
            fact_store.add_fact(test_fact("edge", vec![i, (i + 1) % 10]));
        }

        let planner = QueryPlanner::new(fact_store);

        // Triangle query: result(X, Y, Z) :- edge(X, Y), edge(Y, Z), edge(Z, X)
        let rule = test_rule(
            test_atom("result", vec!["X", "Y", "Z"]),
            vec![
                test_atom("edge", vec!["X", "Y"]),
                test_atom("edge", vec!["Y", "Z"]),
                test_atom("edge", vec!["Z", "X"]),
            ],
        );

        let plan = planner.plan_rule(&rule);

        // Should select WCOJ for triangle pattern
        assert_eq!(plan.backend, BackendType::WCOJ);
    }

    #[test]
    fn test_cost_estimation() {
        let fact_store = Arc::new(FactStore::new());
        for i in 0..100 {
            fact_store.add_fact(test_fact("edge", vec![i, i + 1]));
        }

        let planner = QueryPlanner::new(fact_store);

        let rule = test_rule(
            test_atom("result", vec!["X", "Y"]),
            vec![test_atom("edge", vec!["X", "Y"])],
        );

        let plan = planner.plan_rule(&rule);

        // Cost should be positive
        assert!(plan.estimated_cost > 0.0);
    }

    #[test]
    fn test_plan_formatting() {
        let fact_store = Arc::new(FactStore::new());
        fact_store.add_fact(test_fact("edge", vec![1, 2]));

        let planner = QueryPlanner::new(fact_store);

        let rule = test_rule(
            test_atom("result", vec!["X", "Y"]),
            vec![test_atom("edge", vec!["X", "Y"])],
        );

        let plan = planner.plan_rule(&rule);
        let formatted = plan.format();

        assert!(formatted.contains("Backend:"));
        assert!(formatted.contains("Estimated Cost:"));
        assert!(formatted.contains("Join Order:"));
    }

    #[test]
    fn test_hierarchical_query_detection() {
        let fact_store = Arc::new(FactStore::new());
        let planner = QueryPlanner::new(fact_store);

        // Hierarchical: each atom adds one variable
        let analyses = vec![
            AtomAnalysis {
                atom: test_atom("a", vec!["X"]),
                position: 0,
                bound_vars: 0,
                unbound_vars: 1,
                estimated_cardinality: 10,
                selectivity: 0.5,
            },
            AtomAnalysis {
                atom: test_atom("b", vec!["X", "Y"]),
                position: 1,
                bound_vars: 1,
                unbound_vars: 1,
                estimated_cardinality: 10,
                selectivity: 0.5,
            },
            AtomAnalysis {
                atom: test_atom("c", vec!["Y", "Z"]),
                position: 2,
                bound_vars: 1,
                unbound_vars: 1,
                estimated_cardinality: 10,
                selectivity: 0.5,
            },
        ];

        assert!(planner.is_hierarchical_query(&analyses));
    }
}
