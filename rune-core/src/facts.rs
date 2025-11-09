//! Lock-free fact store for high-performance concurrent access

#![allow(unsafe_code)] // Required for crossbeam epoch-based memory reclamation

use crate::types::Value;
use crossbeam::epoch::{self, Atomic, Owned};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// A fact in the system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Fact {
    /// Fact name/predicate
    pub predicate: Arc<str>,
    /// Fact arguments
    pub args: Arc<[Value]>,
    /// Fact timestamp (for temporal reasoning)
    pub timestamp: u64,
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
}
