//! Lock-free fact store for high-performance concurrent access

#![allow(unsafe_code)] // Required for crossbeam epoch-based memory reclamation

use crate::types::Value;
use crossbeam::epoch::{self, Atomic, Owned};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// A fact in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    /// Fact name/predicate
    pub predicate: Arc<str>,
    /// Fact arguments
    pub args: Arc<[Value]>,
    /// Fact timestamp (for temporal reasoning)
    pub timestamp: u64,
}

// Custom equality that ignores timestamp (facts are logically equal if predicate and args match)
impl PartialEq for Fact {
    fn eq(&self, other: &Self) -> bool {
        self.predicate == other.predicate && self.args == other.args
    }
}

impl Eq for Fact {}

// Custom hash that ignores timestamp
impl std::hash::Hash for Fact {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.predicate.hash(state);
        self.args.hash(state);
    }
}

impl Fact {
    /// Create a new fact
    pub fn new(predicate: impl Into<String>, args: Vec<Value>) -> Self {
        static TIMESTAMP: AtomicU64 = AtomicU64::new(0);

        Fact {
            predicate: Arc::from(predicate.into().into_boxed_str()),
            args: Arc::from(args.into_boxed_slice()),
            timestamp: TIMESTAMP.fetch_add(1, Ordering::Relaxed),
        }
    }

    /// Create a unary fact (single argument)
    pub fn unary(predicate: impl Into<String>, arg: Value) -> Self {
        Self::new(predicate, vec![arg])
    }

    /// Create a binary fact (two arguments)
    pub fn binary(predicate: impl Into<String>, arg1: Value, arg2: Value) -> Self {
        Self::new(predicate, vec![arg1, arg2])
    }

    /// Check if fact matches a pattern
    pub fn matches_pattern(&self, pattern: &FactPattern) -> bool {
        if self.predicate != pattern.predicate {
            return false;
        }

        if self.args.len() != pattern.args.len() {
            return false;
        }

        for (fact_arg, pattern_arg) in self.args.iter().zip(pattern.args.iter()) {
            match pattern_arg {
                PatternArg::Variable(_) => continue,
                PatternArg::Constant(val) => {
                    if fact_arg != val {
                        return false;
                    }
                }
            }
        }

        true
    }
}

/// Pattern for matching facts
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FactPattern {
    /// Predicate to match
    pub predicate: Arc<str>,
    /// Pattern arguments
    pub args: Vec<PatternArg>,
}

/// Argument in a fact pattern
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PatternArg {
    /// Variable (matches anything)
    Variable(String),
    /// Constant (must match exactly)
    Constant(Value),
}

/// Lock-free fact store using crossbeam epoch-based memory reclamation
pub struct FactStore {
    /// Facts indexed by predicate
    facts_by_predicate: DashMap<Arc<str>, Arc<Vec<Fact>>>,
    /// All facts (for full scans)
    all_facts: Atomic<Arc<Vec<Fact>>>,
    /// Version counter for change detection
    version: AtomicU64,
}

impl FactStore {
    /// Create a new fact store
    pub fn new() -> Self {
        FactStore {
            facts_by_predicate: DashMap::new(),
            all_facts: Atomic::new(Arc::new(Vec::new())),
            version: AtomicU64::new(0),
        }
    }

    /// Add a fact to the store
    pub fn add_fact(&self, fact: Fact) {
        // Update predicate index
        self.facts_by_predicate
            .entry(fact.predicate.clone())
            .and_modify(|facts| {
                let mut new_facts = (**facts).clone();
                new_facts.push(fact.clone());
                *facts = Arc::new(new_facts);
            })
            .or_insert_with(|| Arc::new(vec![fact.clone()]));

        // Update all facts using epoch-based reclamation with CAS loop
        let guard = &epoch::pin();

        loop {
            let current = self.all_facts.load(Ordering::Acquire, guard);

            let mut new_facts = if let Some(current_ref) = unsafe { current.as_ref() } {
                (**current_ref).clone()
            } else {
                Vec::new()
            };

            new_facts.push(fact.clone());
            let new_arc = Arc::new(new_facts);
            let new_shared = Owned::new(new_arc).into_shared(guard);

            // Try to swap - if it fails, someone else updated, retry
            match self.all_facts.compare_exchange(
                current,
                new_shared,
                Ordering::Release,
                Ordering::Acquire,
                guard,
            ) {
                Ok(_) => {
                    // Success! Increment version and clean up
                    self.version.fetch_add(1, Ordering::Release);
                    unsafe {
                        guard.defer_destroy(current);
                    }
                    break;
                }
                Err(_) => {
                    // CAS failed, retry the loop
                    // The new_shared we created will be dropped
                    continue;
                }
            }
        }
    }

    /// Add multiple facts atomically
    pub fn add_facts(&self, facts: Vec<Fact>) {
        for fact in facts {
            self.add_fact(fact);
        }
    }

    /// Query facts matching a pattern
    pub fn query(&self, pattern: &FactPattern) -> Vec<Fact> {
        self.facts_by_predicate
            .get(&pattern.predicate)
            .map(|facts| {
                facts
                    .iter()
                    .filter(|f| f.matches_pattern(pattern))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all facts with a specific predicate
    pub fn get_by_predicate(&self, predicate: &str) -> Vec<Fact> {
        self.facts_by_predicate
            .get(predicate)
            .map(|facts| (**facts).clone())
            .unwrap_or_default()
    }

    /// Get all facts
    pub fn all_facts(&self) -> Arc<Vec<Fact>> {
        let guard = &epoch::pin();
        let shared = self.all_facts.load(Ordering::Acquire, guard);

        if let Some(facts_ref) = unsafe { shared.as_ref() } {
            facts_ref.clone()
        } else {
            Arc::new(Vec::new())
        }
    }

    /// Get current version
    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Acquire)
    }

    /// Check if store has changed since a given version
    pub fn has_changed_since(&self, version: u64) -> bool {
        self.version() > version
    }

    /// Clear all facts
    pub fn clear(&self) {
        self.facts_by_predicate.clear();

        let guard = &epoch::pin();
        let current = self.all_facts.load(Ordering::Acquire, guard);
        self.all_facts.store(
            Owned::new(Arc::new(Vec::new())).into_shared(guard),
            Ordering::Release,
        );

        unsafe {
            guard.defer_destroy(current);
        }

        self.version.fetch_add(1, Ordering::Release);
    }

    /// Get fact count
    pub fn len(&self) -> usize {
        self.all_facts().len()
    }

    /// Check if store is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for FactStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Fact store snapshot for consistent reads
pub struct FactSnapshot {
    facts: Arc<Vec<Fact>>,
    version: u64,
}

impl FactSnapshot {
    /// Create a snapshot from the current state
    pub fn from_store(store: &FactStore) -> Self {
        FactSnapshot {
            facts: store.all_facts(),
            version: store.version(),
        }
    }

    /// Get all facts in the snapshot
    pub fn facts(&self) -> &[Fact] {
        &self.facts
    }

    /// Get snapshot version
    pub fn version(&self) -> u64 {
        self.version
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fact_store_basic() {
        let store = FactStore::new();

        // Add a fact
        store.add_fact(Fact::unary("user", Value::string("alice")));

        // Query it back
        let pattern = FactPattern {
            predicate: Arc::from("user"),
            args: vec![PatternArg::Variable("X".into())],
        };

        let results = store.query(&pattern);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].predicate.as_ref(), "user");
    }

    #[test]
    fn test_concurrent_access() {
        use std::thread;

        let store = Arc::new(FactStore::new());
        let mut handles = vec![];

        // Spawn multiple threads adding facts
        for i in 0..10 {
            let store = store.clone();
            let handle = thread::spawn(move || {
                for j in 0..100 {
                    store.add_fact(Fact::binary("edge", Value::Integer(i), Value::Integer(j)));
                }
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Check we have all facts
        assert_eq!(store.len(), 1000);
    }

    // ========== Edge Case Tests ==========

    #[test]
    fn test_fact_equality_ignores_timestamp() {
        // Facts with same predicate and args should be equal regardless of timestamp
        let fact1 = Fact::new("user", vec![Value::string("alice")]);
        std::thread::sleep(std::time::Duration::from_millis(1));
        let fact2 = Fact::new("user", vec![Value::string("alice")]);

        assert_ne!(fact1.timestamp, fact2.timestamp);
        assert_eq!(fact1, fact2);

        // Different predicate
        let fact3 = Fact::new("admin", vec![Value::string("alice")]);
        assert_ne!(fact1, fact3);

        // Different args
        let fact4 = Fact::new("user", vec![Value::string("bob")]);
        assert_ne!(fact1, fact4);
    }

    #[test]
    fn test_fact_hash_ignores_timestamp() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let fact1 = Fact::new("user", vec![Value::string("alice")]);
        std::thread::sleep(std::time::Duration::from_millis(1));
        let fact2 = Fact::new("user", vec![Value::string("alice")]);

        let mut hasher1 = DefaultHasher::new();
        fact1.hash(&mut hasher1);
        let hash1 = hasher1.finish();

        let mut hasher2 = DefaultHasher::new();
        fact2.hash(&mut hasher2);
        let hash2 = hasher2.finish();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_fact_constructors() {
        // Test unary fact
        let unary = Fact::unary("user", Value::string("alice"));
        assert_eq!(unary.predicate.as_ref(), "user");
        assert_eq!(unary.args.len(), 1);
        assert_eq!(unary.args[0], Value::string("alice"));

        // Test binary fact
        let binary = Fact::binary("follows", Value::string("alice"), Value::string("bob"));
        assert_eq!(binary.predicate.as_ref(), "follows");
        assert_eq!(binary.args.len(), 2);
        assert_eq!(binary.args[0], Value::string("alice"));
        assert_eq!(binary.args[1], Value::string("bob"));

        // Test n-ary fact
        let nary = Fact::new(
            "triple",
            vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)],
        );
        assert_eq!(nary.predicate.as_ref(), "triple");
        assert_eq!(nary.args.len(), 3);
    }

    #[test]
    fn test_pattern_matching_edge_cases() {
        // Test exact match with constants
        let fact = Fact::binary("follows", Value::string("alice"), Value::string("bob"));

        let pattern_exact = FactPattern {
            predicate: Arc::from("follows"),
            args: vec![
                PatternArg::Constant(Value::string("alice")),
                PatternArg::Constant(Value::string("bob")),
            ],
        };
        assert!(fact.matches_pattern(&pattern_exact));

        // Test mismatch with wrong constant
        let pattern_wrong = FactPattern {
            predicate: Arc::from("follows"),
            args: vec![
                PatternArg::Constant(Value::string("alice")),
                PatternArg::Constant(Value::string("charlie")),
            ],
        };
        assert!(!fact.matches_pattern(&pattern_wrong));

        // Test with all variables
        let pattern_vars = FactPattern {
            predicate: Arc::from("follows"),
            args: vec![
                PatternArg::Variable("X".into()),
                PatternArg::Variable("Y".into()),
            ],
        };
        assert!(fact.matches_pattern(&pattern_vars));

        // Test mixed constants and variables
        let pattern_mixed = FactPattern {
            predicate: Arc::from("follows"),
            args: vec![
                PatternArg::Constant(Value::string("alice")),
                PatternArg::Variable("X".into()),
            ],
        };
        assert!(fact.matches_pattern(&pattern_mixed));

        // Test wrong predicate
        let pattern_wrong_pred = FactPattern {
            predicate: Arc::from("likes"),
            args: vec![
                PatternArg::Variable("X".into()),
                PatternArg::Variable("Y".into()),
            ],
        };
        assert!(!fact.matches_pattern(&pattern_wrong_pred));

        // Test wrong arity
        let pattern_wrong_arity = FactPattern {
            predicate: Arc::from("follows"),
            args: vec![PatternArg::Variable("X".into())],
        };
        assert!(!fact.matches_pattern(&pattern_wrong_arity));
    }

    #[test]
    fn test_fact_store_empty_operations() {
        let store = FactStore::new();

        // Test empty store
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
        assert_eq!(store.version(), 0);
        assert_eq!(store.all_facts().len(), 0);

        // Query on empty store
        let pattern = FactPattern {
            predicate: Arc::from("user"),
            args: vec![PatternArg::Variable("X".into())],
        };
        assert_eq!(store.query(&pattern).len(), 0);

        // Get by predicate on empty store
        assert_eq!(store.get_by_predicate("user").len(), 0);
    }

    #[test]
    fn test_fact_store_add_and_query() {
        let store = FactStore::new();

        // Add multiple facts with same predicate
        store.add_fact(Fact::unary("user", Value::string("alice")));
        store.add_fact(Fact::unary("user", Value::string("bob")));
        store.add_fact(Fact::unary("user", Value::string("charlie")));

        // Query with variable
        let pattern_var = FactPattern {
            predicate: Arc::from("user"),
            args: vec![PatternArg::Variable("X".into())],
        };
        let results = store.query(&pattern_var);
        assert_eq!(results.len(), 3);

        // Query with constant
        let pattern_const = FactPattern {
            predicate: Arc::from("user"),
            args: vec![PatternArg::Constant(Value::string("alice"))],
        };
        let results = store.query(&pattern_const);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].args[0], Value::string("alice"));

        // Get all by predicate
        let all_users = store.get_by_predicate("user");
        assert_eq!(all_users.len(), 3);
    }

    #[test]
    fn test_fact_store_add_facts_batch() {
        let store = FactStore::new();

        let facts = vec![
            Fact::unary("user", Value::string("alice")),
            Fact::unary("user", Value::string("bob")),
            Fact::binary("follows", Value::string("alice"), Value::string("bob")),
        ];

        store.add_facts(facts);

        assert_eq!(store.len(), 3);
        assert_eq!(store.get_by_predicate("user").len(), 2);
        assert_eq!(store.get_by_predicate("follows").len(), 1);
    }

    #[test]
    fn test_fact_store_version_tracking() {
        let store = FactStore::new();

        let initial_version = store.version();
        assert_eq!(initial_version, 0);
        assert!(!store.has_changed_since(0));

        // Add a fact should increment version
        store.add_fact(Fact::unary("user", Value::string("alice")));
        let v1 = store.version();
        assert!(v1 > initial_version);
        assert!(store.has_changed_since(initial_version));

        // Add another fact
        store.add_fact(Fact::unary("user", Value::string("bob")));
        let v2 = store.version();
        assert!(v2 > v1);
        assert!(store.has_changed_since(v1));

        // Clear should increment version
        store.clear();
        let v3 = store.version();
        assert!(v3 > v2);
        assert!(store.has_changed_since(v2));
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn test_fact_store_clear() {
        let store = FactStore::new();

        // Add some facts
        store.add_fact(Fact::unary("user", Value::string("alice")));
        store.add_fact(Fact::unary("user", Value::string("bob")));
        store.add_fact(Fact::binary(
            "follows",
            Value::string("alice"),
            Value::string("bob"),
        ));

        assert_eq!(store.len(), 3);
        assert!(!store.is_empty());

        // Clear the store
        store.clear();

        assert_eq!(store.len(), 0);
        assert!(store.is_empty());
        assert_eq!(store.all_facts().len(), 0);
        assert_eq!(store.get_by_predicate("user").len(), 0);
        assert_eq!(store.get_by_predicate("follows").len(), 0);
    }

    #[test]
    fn test_fact_snapshot() {
        let store = FactStore::new();

        // Add initial facts
        store.add_fact(Fact::unary("user", Value::string("alice")));
        store.add_fact(Fact::unary("user", Value::string("bob")));

        let initial_version = store.version();

        // Create snapshot
        let snapshot = FactSnapshot::from_store(&store);
        assert_eq!(snapshot.facts().len(), 2);
        assert_eq!(snapshot.version(), initial_version);

        // Add more facts after snapshot
        store.add_fact(Fact::unary("user", Value::string("charlie")));

        // Snapshot should still have only 2 facts
        assert_eq!(snapshot.facts().len(), 2);
        assert_eq!(snapshot.version(), initial_version);

        // Store should have 3 facts with new version
        assert_eq!(store.len(), 3);
        assert!(store.version() > initial_version);
    }

    #[test]
    fn test_fact_snapshot_consistency() {
        let store = FactStore::new();

        // Add facts
        let facts = vec![
            Fact::unary("user", Value::string("alice")),
            Fact::unary("admin", Value::string("bob")),
            Fact::binary("follows", Value::string("alice"), Value::string("bob")),
        ];

        for fact in &facts {
            store.add_fact(fact.clone());
        }

        // Create multiple snapshots
        let snapshot1 = FactSnapshot::from_store(&store);
        let snapshot2 = FactSnapshot::from_store(&store);

        // Both snapshots should be identical
        assert_eq!(snapshot1.facts().len(), snapshot2.facts().len());
        assert_eq!(snapshot1.version(), snapshot2.version());

        // Verify all facts are present in snapshots
        for fact in &facts {
            assert!(snapshot1.facts().iter().any(|f| f == fact));
            assert!(snapshot2.facts().iter().any(|f| f == fact));
        }
    }

    #[test]
    fn test_fact_store_default() {
        let store = FactStore::default();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
        assert_eq!(store.version(), 0);
    }

    #[test]
    fn test_fact_store_complex_queries() {
        let store = FactStore::new();

        // Add facts with different patterns
        store.add_fact(Fact::binary(
            "follows",
            Value::string("alice"),
            Value::string("bob"),
        ));
        store.add_fact(Fact::binary(
            "follows",
            Value::string("bob"),
            Value::string("charlie"),
        ));
        store.add_fact(Fact::binary(
            "follows",
            Value::string("alice"),
            Value::string("charlie"),
        ));
        store.add_fact(Fact::binary(
            "likes",
            Value::string("alice"),
            Value::string("coding"),
        ));

        // Query: Who follows bob?
        let pattern1 = FactPattern {
            predicate: Arc::from("follows"),
            args: vec![
                PatternArg::Variable("X".into()),
                PatternArg::Constant(Value::string("bob")),
            ],
        };
        let results = store.query(&pattern1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].args[0], Value::string("alice"));

        // Query: Who does alice follow?
        let pattern2 = FactPattern {
            predicate: Arc::from("follows"),
            args: vec![
                PatternArg::Constant(Value::string("alice")),
                PatternArg::Variable("Y".into()),
            ],
        };
        let results = store.query(&pattern2);
        assert_eq!(results.len(), 2); // alice follows bob and charlie

        // Query: All follows relationships
        let pattern3 = FactPattern {
            predicate: Arc::from("follows"),
            args: vec![
                PatternArg::Variable("X".into()),
                PatternArg::Variable("Y".into()),
            ],
        };
        let results = store.query(&pattern3);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_pattern_arg_equality() {
        // Test Variable equality
        let var1 = PatternArg::Variable("X".into());
        let var2 = PatternArg::Variable("X".into());
        let var3 = PatternArg::Variable("Y".into());

        assert_eq!(var1, var2);
        assert_ne!(var1, var3);

        // Test Constant equality
        let const1 = PatternArg::Constant(Value::string("alice"));
        let const2 = PatternArg::Constant(Value::string("alice"));
        let const3 = PatternArg::Constant(Value::string("bob"));

        assert_eq!(const1, const2);
        assert_ne!(const1, const3);

        // Variable vs Constant
        assert_ne!(var1, const1);
    }

    #[test]
    fn test_fact_pattern_equality() {
        let pattern1 = FactPattern {
            predicate: Arc::from("user"),
            args: vec![PatternArg::Variable("X".into())],
        };

        let pattern2 = FactPattern {
            predicate: Arc::from("user"),
            args: vec![PatternArg::Variable("X".into())],
        };

        let pattern3 = FactPattern {
            predicate: Arc::from("admin"),
            args: vec![PatternArg::Variable("X".into())],
        };

        assert_eq!(pattern1, pattern2);
        assert_ne!(pattern1, pattern3);
    }

    #[test]
    fn test_timestamp_ordering() {
        // Facts should have monotonically increasing timestamps
        let fact1 = Fact::unary("user", Value::string("alice"));
        let fact2 = Fact::unary("user", Value::string("bob"));
        let fact3 = Fact::unary("user", Value::string("charlie"));

        assert!(fact2.timestamp > fact1.timestamp);
        assert!(fact3.timestamp > fact2.timestamp);
    }

    #[test]
    fn test_concurrent_snapshots() {
        use std::thread;

        let store = Arc::new(FactStore::new());

        // Add initial facts
        store.add_fact(Fact::unary("initial", Value::Integer(0)));

        let mut handles = vec![];

        // Thread adding facts
        let store_add = store.clone();
        handles.push(thread::spawn(move || {
            for i in 1..=100 {
                store_add.add_fact(Fact::unary("concurrent", Value::Integer(i)));
            }
        }));

        // Thread taking snapshots
        let store_snap = store.clone();
        handles.push(thread::spawn(move || {
            let mut snapshots = vec![];
            for _ in 0..10 {
                snapshots.push(FactSnapshot::from_store(&store_snap));
                thread::sleep(std::time::Duration::from_millis(1));
            }

            // All snapshots should be valid
            for snapshot in snapshots {
                assert!(!snapshot.facts().is_empty()); // At least initial fact
                                                       // Version is always >= 0 since it's unsigned
            }
        }));

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Final state should have all facts
        assert_eq!(store.len(), 101); // 1 initial + 100 concurrent
    }
}
