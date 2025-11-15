//! Custom Datalog engine for RUNE
//!
//! This module provides a from-scratch Datalog implementation designed
//! specifically for RUNE's requirements:
//!
//! - **Lock-free concurrent reads**: Arc-based fact storage
//! - **Hot-reload capable**: Interpreted rules (not compile-time macros)
//! - **Cedar integration**: Bridge between Datalog facts and Cedar entities
//! - **Semi-naive evaluation**: Efficient fixpoint computation with delta tracking
//! - **Stratified negation**: Safe handling of negated atoms
//! - **Aggregation support**: count, sum, min, max operations
//!
//! Design rationale:
//! Existing Rust Datalog crates (datafrog, ascent, crepe) use compile-time
//! code generation which prevents runtime rule modification. RUNE needs to
//! hot-reload policies without recompilation, so we implement a custom
//! interpreter-based engine.
//!
//! Evaluation strategy:
//! - Bottom-up semi-naive evaluation (from datafrog/ascent)
//! - Stratification for safe negation (from ascent/crepe)
//! - BYODS principle for future optimization (from ascent)

pub mod aggregation;
pub mod backends;
pub mod bridge;
pub mod diagnostics;
pub mod evaluation;
pub mod incremental;
pub mod lattice;
pub mod magic_sets;
pub mod planner;
pub mod provenance;
pub mod semi_naive;
pub mod types;
pub mod unification;
pub mod wcoj;

// Re-export main types
pub use aggregation::{evaluate_aggregate, AggregationResult};
pub use backends::{
    BackendType, HashBackend, RelationBackend, TrieBackend, UnionFindBackend, VecBackend,
};
pub use bridge::CedarDatalogBridge;
pub use diagnostics::{DatalogDiagnostics, Diagnostic, DiagnosticBag, Severity, Span, Suggestion};
pub use evaluation::{EvaluationResult, Evaluator};
pub use incremental::{
    compute_fact_diff, Delta, IncrementalEvaluator, IncrementalResult, IncrementalStats,
};
pub use lattice::{
    BoolLattice, CounterLattice, Lattice, LatticeValue, MaxLattice, MinLattice, SetLattice,
};
pub use magic_sets::{MagicSetsTransformer, Query};
pub use planner::{AtomAnalysis, PredicateStats, QueryPlan, QueryPlanner};
pub use provenance::{ProofTree, ProvenanceQuery, ProvenanceTracker};
pub use types::{AggregateAtom, AggregateOp, Atom, Rule, Substitution, Term};
pub use unification::{find_matching_facts, ground_atom, unify_atom_with_fact, unify_atoms};
pub use wcoj::{LeapfrogIterator, LeapfrogJoin, TrieNode, WCOJIndex};

use crate::engine::{AuthorizationResult, Decision};
use crate::error::Result;
use crate::facts::FactStore;
use crate::request::Request;
use std::sync::Arc;
use std::time::Instant;

/// Datalog evaluation engine
pub struct DatalogEngine {
    /// Compiled Datalog rules
    rules: Arc<Vec<Rule>>,
    /// Fact store reference
    fact_store: Arc<FactStore>,
}

impl DatalogEngine {
    /// Create a new Datalog engine with rules
    pub fn new(rules: Vec<Rule>, fact_store: Arc<FactStore>) -> Self {
        DatalogEngine {
            rules: Arc::new(rules),
            fact_store,
        }
    }

    /// Create an empty Datalog engine (no rules)
    pub fn empty(fact_store: Arc<FactStore>) -> Self {
        Self::new(vec![], fact_store)
    }

    /// Evaluate a request against Datalog rules
    pub fn evaluate(&self, _request: &Request, _facts: &FactStore) -> Result<AuthorizationResult> {
        let start = Instant::now();

        // Create evaluator with current rules
        // Use the engine's fact store which is already Arc-wrapped
        let evaluator = Evaluator::new((*self.rules).clone(), self.fact_store.clone());

        // Run evaluation
        let result = evaluator.evaluate();

        // Convert to AuthorizationResult
        // For now, always permit if we have derived facts
        let decision = if result.facts.is_empty() {
            Decision::Deny
        } else {
            Decision::Permit
        };

        let explanation = format!(
            "Datalog evaluation completed in {} iterations, derived {} facts",
            result.iterations,
            result.facts.len()
        );

        let evaluated_rules: Vec<String> = self.rules.iter().map(|r| format!("{}", r)).collect();

        let facts_used: Vec<String> = result
            .facts
            .iter()
            .map(|f| format!("{}({:?})", f.predicate, f.args))
            .collect();

        Ok(AuthorizationResult {
            decision,
            explanation,
            evaluated_rules,
            facts_used,
            evaluation_time_ns: start.elapsed().as_nanos() as u64,
            cached: false,
        })
    }

    /// Add rules to the engine (for hot-reload)
    pub fn update_rules(&mut self, rules: Vec<Rule>) {
        self.rules = Arc::new(rules);
    }

    /// Get current rules
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    /// Evaluate rules and return derived facts
    pub fn derive_facts(&self) -> Result<Vec<crate::facts::Fact>> {
        let evaluator = Evaluator::new((*self.rules).clone(), self.fact_store.clone());
        let result = evaluator.evaluate();
        Ok(result.facts)
    }
}
