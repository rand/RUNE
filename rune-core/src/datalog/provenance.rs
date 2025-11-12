//! Lazy provenance tracking for Datalog evaluation
//!
//! Records derivation trees showing how facts were derived from rules and base facts.
//! Essential for debugging and explaining authorization decisions.
//!
//! Features:
//! - Lazy evaluation: only compute provenance when requested
//! - Compact representation: share common sub-derivations
//! - Query interface: find all derivations of a fact
//! - Explanation generation: produce human-readable explanations

use crate::facts::Fact;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

/// A derivation node in the provenance graph
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Derivation {
    /// The fact that was derived
    pub fact: Fact,
    /// How this fact was derived
    pub source: DerivationSource,
}

/// Source of a derivation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DerivationSource {
    /// Base fact (not derived)
    Base,
    /// Derived from a rule application
    Rule {
        /// Name of the rule that was applied
        rule_name: String,
        /// Rule ID for tracking
        rule_id: usize,
        /// Facts used as premises for this rule
        premises: Vec<Arc<Derivation>>,
    },
}

/// Provenance tracker that records derivation information
#[derive(Debug, Clone)]
pub struct ProvenanceTracker {
    /// Map from facts to their derivations
    derivations: HashMap<Fact, Vec<Arc<Derivation>>>,
    /// Cache of shared derivation nodes
    derivation_cache: HashMap<Derivation, Arc<Derivation>>,
    /// Whether provenance tracking is enabled
    enabled: bool,
}

impl ProvenanceTracker {
    /// Create a new provenance tracker
    pub fn new(enabled: bool) -> Self {
        ProvenanceTracker {
            derivations: HashMap::new(),
            derivation_cache: HashMap::new(),
            enabled,
        }
    }

    /// Record a base fact
    pub fn record_base(&mut self, fact: Fact) {
        if !self.enabled {
            return;
        }

        let derivation = Derivation {
            fact: fact.clone(),
            source: DerivationSource::Base,
        };

        let arc_derivation = self.get_or_cache_derivation(derivation);
        self.derivations
            .entry(fact)
            .or_insert_with(Vec::new)
            .push(arc_derivation);
    }

    /// Record a derived fact
    pub fn record_derived(
        &mut self,
        fact: Fact,
        rule_name: String,
        rule_id: usize,
        premises: Vec<Fact>,
    ) {
        if !self.enabled {
            return;
        }

        // Get derivations for each premise
        let premise_derivations: Vec<Arc<Derivation>> = premises
            .iter()
            .flat_map(|p| {
                self.derivations
                    .get(p)
                    .cloned()
                    .unwrap_or_else(|| {
                        // If no derivation found, treat as base fact
                        vec![self.get_or_cache_derivation(Derivation {
                            fact: p.clone(),
                            source: DerivationSource::Base,
                        })]
                    })
            })
            .collect();

        let derivation = Derivation {
            fact: fact.clone(),
            source: DerivationSource::Rule {
                rule_name,
                rule_id,
                premises: premise_derivations,
            },
        };

        let arc_derivation = self.get_or_cache_derivation(derivation);
        self.derivations
            .entry(fact)
            .or_insert_with(Vec::new)
            .push(arc_derivation);
    }

    /// Get or cache a derivation to avoid duplicates
    fn get_or_cache_derivation(&mut self, derivation: Derivation) -> Arc<Derivation> {
        if let Some(cached) = self.derivation_cache.get(&derivation) {
            cached.clone()
        } else {
            let arc = Arc::new(derivation.clone());
            self.derivation_cache.insert(derivation, arc.clone());
            arc
        }
    }

    /// Get all derivations for a fact
    pub fn get_derivations(&self, fact: &Fact) -> Vec<Arc<Derivation>> {
        self.derivations.get(fact).cloned().unwrap_or_default()
    }

    /// Check if a fact has any derivations
    pub fn has_derivation(&self, fact: &Fact) -> bool {
        self.derivations.contains_key(fact)
    }

    /// Get a proof tree for a fact
    pub fn get_proof_tree(&self, fact: &Fact) -> Option<ProofTree> {
        let derivations = self.get_derivations(fact);
        if derivations.is_empty() {
            return None;
        }

        // Use the first derivation as the proof
        // (In practice, might want to select "best" proof)
        Some(ProofTree {
            root: derivations[0].clone(),
        })
    }

    /// Generate a human-readable explanation for a fact
    pub fn explain(&self, fact: &Fact) -> String {
        if let Some(proof) = self.get_proof_tree(fact) {
            proof.to_explanation()
        } else {
            format!("No derivation found for: {:?}", fact)
        }
    }

    /// Clear all provenance information
    pub fn clear(&mut self) {
        self.derivations.clear();
        self.derivation_cache.clear();
    }

    /// Enable or disable provenance tracking
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.clear();
        }
    }

    /// Get statistics about provenance tracking
    pub fn stats(&self) -> ProvenanceStats {
        ProvenanceStats {
            total_facts: self.derivations.len(),
            total_derivations: self.derivations.values().map(|v| v.len()).sum(),
            cached_nodes: self.derivation_cache.len(),
            enabled: self.enabled,
        }
    }
}

/// A proof tree showing how a fact was derived
#[derive(Debug, Clone)]
pub struct ProofTree {
    /// Root derivation
    pub root: Arc<Derivation>,
}

impl ProofTree {
    /// Convert proof tree to human-readable explanation
    pub fn to_explanation(&self) -> String {
        self.format_derivation(&self.root, 0)
    }

    fn format_derivation(&self, derivation: &Derivation, indent: usize) -> String {
        let indent_str = "  ".repeat(indent);
        let fact_str = format!("{:?}", derivation.fact);

        match &derivation.source {
            DerivationSource::Base => {
                format!("{}• {} (base fact)", indent_str, fact_str)
            }
            DerivationSource::Rule {
                rule_name,
                premises,
                ..
            } => {
                let mut result = format!("{}• {} (by {})", indent_str, fact_str, rule_name);
                if !premises.is_empty() {
                    result.push_str("\n");
                    result.push_str(&format!("{}  because:", indent_str));
                    for premise in premises {
                        result.push_str("\n");
                        result.push_str(&self.format_derivation(premise, indent + 1));
                    }
                }
                result
            }
        }
    }

    /// Get the depth of the proof tree
    pub fn depth(&self) -> usize {
        self.compute_depth(&self.root)
    }

    fn compute_depth(&self, derivation: &Derivation) -> usize {
        match &derivation.source {
            DerivationSource::Base => 1,
            DerivationSource::Rule { premises, .. } => {
                1 + premises
                    .iter()
                    .map(|p| self.compute_depth(p))
                    .max()
                    .unwrap_or(0)
            }
        }
    }

    /// Count total nodes in the proof tree
    pub fn node_count(&self) -> usize {
        let mut visited = HashSet::new();
        self.count_nodes(&self.root, &mut visited)
    }

    fn count_nodes(&self, derivation: &Derivation, visited: &mut HashSet<Fact>) -> usize {
        if visited.contains(&derivation.fact) {
            return 0;
        }
        visited.insert(derivation.fact.clone());

        match &derivation.source {
            DerivationSource::Base => 1,
            DerivationSource::Rule { premises, .. } => {
                1 + premises
                    .iter()
                    .map(|p| self.count_nodes(p, visited))
                    .sum::<usize>()
            }
        }
    }
}

/// Statistics about provenance tracking
#[derive(Debug, Clone)]
pub struct ProvenanceStats {
    /// Total number of facts with provenance
    pub total_facts: usize,
    /// Total number of derivations across all facts
    pub total_derivations: usize,
    /// Number of cached derivation nodes
    pub cached_nodes: usize,
    /// Whether tracking is enabled
    pub enabled: bool,
}

/// Query interface for provenance
pub struct ProvenanceQuery<'a> {
    tracker: &'a ProvenanceTracker,
}

impl<'a> ProvenanceQuery<'a> {
    /// Create a new query interface
    pub fn new(tracker: &'a ProvenanceTracker) -> Self {
        ProvenanceQuery { tracker }
    }

    /// Find all facts that contributed to deriving the target fact
    pub fn contributing_facts(&self, target: &Fact) -> HashSet<Fact> {
        let mut result = HashSet::new();
        let mut queue = VecDeque::new();

        if let Some(proof) = self.tracker.get_proof_tree(target) {
            queue.push_back(proof.root);
        }

        while let Some(derivation) = queue.pop_front() {
            result.insert(derivation.fact.clone());

            if let DerivationSource::Rule { premises, .. } = &derivation.source {
                for premise in premises {
                    if !result.contains(&premise.fact) {
                        queue.push_back(premise.clone());
                    }
                }
            }
        }

        result
    }

    /// Find all rules that were used in deriving the target fact
    pub fn contributing_rules(&self, target: &Fact) -> HashSet<(String, usize)> {
        let mut result = HashSet::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        if let Some(proof) = self.tracker.get_proof_tree(target) {
            queue.push_back(proof.root);
        }

        while let Some(derivation) = queue.pop_front() {
            if visited.contains(&derivation.fact) {
                continue;
            }
            visited.insert(derivation.fact.clone());

            if let DerivationSource::Rule {
                rule_name,
                rule_id,
                premises,
                ..
            } = &derivation.source
            {
                result.insert((rule_name.clone(), *rule_id));
                for premise in premises {
                    queue.push_back(premise.clone());
                }
            }
        }

        result
    }

    /// Find the shortest derivation path for a fact
    pub fn shortest_proof(&self, target: &Fact) -> Option<ProofTree> {
        let derivations = self.tracker.get_derivations(target);
        derivations
            .into_iter()
            .map(|d| ProofTree { root: d })
            .min_by_key(|proof| proof.depth())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Value;

    fn test_fact(pred: &str, arg: i64) -> Fact {
        Fact::new(pred.to_string(), vec![Value::Integer(arg)])
    }

    #[test]
    fn test_provenance_base_fact() {
        let mut tracker = ProvenanceTracker::new(true);
        let fact = test_fact("node", 1);

        tracker.record_base(fact.clone());

        let derivations = tracker.get_derivations(&fact);
        assert_eq!(derivations.len(), 1);
        assert_eq!(derivations[0].source, DerivationSource::Base);
    }

    #[test]
    fn test_provenance_derived_fact() {
        let mut tracker = ProvenanceTracker::new(true);

        let base1 = test_fact("edge", 1);
        let base2 = test_fact("edge", 2);
        let derived = test_fact("path", 3);

        tracker.record_base(base1.clone());
        tracker.record_base(base2.clone());
        tracker.record_derived(
            derived.clone(),
            "transitive".to_string(),
            1,
            vec![base1, base2],
        );

        let derivations = tracker.get_derivations(&derived);
        assert_eq!(derivations.len(), 1);

        if let DerivationSource::Rule { rule_name, premises, .. } = &derivations[0].source {
            assert_eq!(rule_name, "transitive");
            assert_eq!(premises.len(), 2);
        } else {
            panic!("Expected Rule derivation");
        }
    }

    #[test]
    fn test_provenance_disabled() {
        let mut tracker = ProvenanceTracker::new(false);
        let fact = test_fact("node", 1);

        tracker.record_base(fact.clone());

        let derivations = tracker.get_derivations(&fact);
        assert_eq!(derivations.len(), 0);
    }

    #[test]
    fn test_proof_tree() {
        let mut tracker = ProvenanceTracker::new(true);

        let base = test_fact("edge", 1);
        let derived = test_fact("path", 2);

        tracker.record_base(base.clone());
        tracker.record_derived(
            derived.clone(),
            "rule1".to_string(),
            1,
            vec![base],
        );

        let proof = tracker.get_proof_tree(&derived).unwrap();
        assert_eq!(proof.depth(), 2);
        assert_eq!(proof.node_count(), 2);
    }

    #[test]
    fn test_explanation_generation() {
        let mut tracker = ProvenanceTracker::new(true);

        let base = test_fact("user", 1);
        let derived = test_fact("authorized", 2);

        tracker.record_base(base.clone());
        tracker.record_derived(
            derived.clone(),
            "auth_rule".to_string(),
            1,
            vec![base],
        );

        let explanation = tracker.explain(&derived);
        assert!(explanation.contains("auth_rule"));
        assert!(explanation.contains("base fact"));
    }

    #[test]
    fn test_provenance_query() {
        let mut tracker = ProvenanceTracker::new(true);

        let base1 = test_fact("A", 1);
        let base2 = test_fact("B", 2);
        let intermediate = test_fact("C", 3);
        let final_fact = test_fact("D", 4);

        tracker.record_base(base1.clone());
        tracker.record_base(base2.clone());
        tracker.record_derived(
            intermediate.clone(),
            "rule1".to_string(),
            1,
            vec![base1.clone(), base2.clone()],
        );
        tracker.record_derived(
            final_fact.clone(),
            "rule2".to_string(),
            2,
            vec![intermediate],
        );

        let query = ProvenanceQuery::new(&tracker);

        // Test contributing facts
        let contributing = query.contributing_facts(&final_fact);
        assert_eq!(contributing.len(), 4);
        assert!(contributing.contains(&base1));
        assert!(contributing.contains(&base2));

        // Test contributing rules
        let rules = query.contributing_rules(&final_fact);
        assert_eq!(rules.len(), 2);
        assert!(rules.contains(&("rule1".to_string(), 1)));
        assert!(rules.contains(&("rule2".to_string(), 2)));
    }

    #[test]
    fn test_provenance_stats() {
        let mut tracker = ProvenanceTracker::new(true);

        for i in 0..5 {
            tracker.record_base(test_fact("base", i));
        }

        let stats = tracker.stats();
        assert_eq!(stats.total_facts, 5);
        assert_eq!(stats.total_derivations, 5);
        assert!(stats.enabled);
    }
}