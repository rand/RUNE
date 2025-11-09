//! Relation backends for optimized fact storage
//!
//! Implements the BYODS (Bring Your Own Data Structures) principle from
//! ascent, allowing different storage strategies for different relation types.
//!
//! Backend selection criteria:
//! - **VecBackend**: Small relations (<100 facts), append-only patterns
//! - **HashBackend**: General-purpose, fast lookups, deduplication
//! - **UnionFindBackend**: Transitive closure, equivalence classes
//! - **TrieBackend**: Prefix matching, hierarchical data

use crate::facts::Fact;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Trait for relation storage backends
///
/// Backends must support:
/// - Insert (with deduplication)
/// - Membership test
/// - Iteration over all facts
/// - Cloning for semi-naive evaluation
pub trait RelationBackend: Clone + Send + Sync {
    /// Insert a fact, returning true if it was newly inserted
    fn insert(&mut self, fact: Fact) -> bool;

    /// Check if a fact exists
    fn contains(&self, fact: &Fact) -> bool;

    /// Get all facts as a Vec
    fn iter(&self) -> Vec<Fact>;

    /// Number of facts stored
    fn len(&self) -> usize;

    /// Check if empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clear all facts
    fn clear(&mut self);

    /// Extend with facts from another backend
    fn extend(&mut self, other: &Self) {
        for fact in other.iter() {
            self.insert(fact);
        }
    }

    /// Get facts matching a predicate
    fn filter_by_predicate(&self, predicate: &str) -> Vec<Fact> {
        self.iter()
            .into_iter()
            .filter(|f| f.predicate.as_ref() == predicate)
            .collect()
    }
}

/// Vector-based backend for small, append-only relations
///
/// Best for:
/// - Relations with <100 facts
/// - Mostly append operations
/// - Sequential scans
///
/// Characteristics:
/// - O(n) insertion (linear scan for dedup)
/// - O(n) membership test
/// - O(1) iteration
/// - Minimal memory overhead
#[derive(Debug, Clone)]
pub struct VecBackend {
    facts: Vec<Fact>,
}

impl VecBackend {
    /// Create a new empty vector backend
    pub fn new() -> Self {
        VecBackend { facts: Vec::new() }
    }

    /// Create a new vector backend with pre-allocated capacity
    pub fn with_capacity(capacity: usize) -> Self {
        VecBackend {
            facts: Vec::with_capacity(capacity),
        }
    }
}

impl Default for VecBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl RelationBackend for VecBackend {
    fn insert(&mut self, fact: Fact) -> bool {
        // Linear scan for deduplication
        if self.facts.iter().any(|f| f == &fact) {
            return false;
        }
        self.facts.push(fact);
        true
    }

    fn contains(&self, fact: &Fact) -> bool {
        self.facts.iter().any(|f| f == fact)
    }

    fn iter(&self) -> Vec<Fact> {
        self.facts.clone()
    }

    fn len(&self) -> usize {
        self.facts.len()
    }

    fn clear(&mut self) {
        self.facts.clear();
    }
}

/// HashMap-based backend for general-purpose relations
///
/// Best for:
/// - Large relations (>100 facts)
/// - Random access patterns
/// - Frequent membership tests
///
/// Characteristics:
/// - O(1) insertion
/// - O(1) membership test
/// - O(n) iteration
/// - Higher memory overhead than Vec
#[derive(Debug, Clone)]
pub struct HashBackend {
    facts: HashSet<Fact>,
}

impl HashBackend {
    /// Create a new empty hash backend
    pub fn new() -> Self {
        HashBackend {
            facts: HashSet::new(),
        }
    }

    /// Create a new hash backend with pre-allocated capacity
    pub fn with_capacity(capacity: usize) -> Self {
        HashBackend {
            facts: HashSet::with_capacity(capacity),
        }
    }

    /// Create from existing HashSet (zero-copy)
    pub fn from_set(facts: HashSet<Fact>) -> Self {
        HashBackend { facts }
    }
}

impl Default for HashBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl RelationBackend for HashBackend {
    fn insert(&mut self, fact: Fact) -> bool {
        self.facts.insert(fact)
    }

    fn contains(&self, fact: &Fact) -> bool {
        self.facts.contains(fact)
    }

    fn iter(&self) -> Vec<Fact> {
        self.facts.iter().cloned().collect()
    }

    fn len(&self) -> usize {
        self.facts.len()
    }

    fn clear(&mut self) {
        self.facts.clear();
    }
}

/// UnionFind-based backend for transitive closure relations
///
/// Best for:
/// - Equivalence relations
/// - Transitive closure (path, reachability)
/// - Connected components
///
/// Characteristics:
/// - O(α(n)) find/union (inverse Ackermann, effectively constant)
/// - Optimized for repeated reachability queries
/// - Automatically deduplicates equivalent paths
///
/// Note: Currently stores facts in HashSet but provides foundation
/// for future UnionFind optimization
#[derive(Debug, Clone)]
pub struct UnionFindBackend {
    // TODO: Replace with actual UnionFind structure
    // For now, use HashSet as baseline
    facts: HashSet<Fact>,
    // Future: HashMap<Value, Value> for parent pointers
    // Future: HashMap<Value, usize> for ranks
}

impl UnionFindBackend {
    /// Create a new empty union-find backend
    pub fn new() -> Self {
        UnionFindBackend {
            facts: HashSet::new(),
        }
    }

    /// Create a new union-find backend with pre-allocated capacity
    pub fn with_capacity(capacity: usize) -> Self {
        UnionFindBackend {
            facts: HashSet::with_capacity(capacity),
        }
    }

    // TODO: Implement actual UnionFind operations
    // pub fn find(&mut self, x: &Value) -> Value { ... }
    // pub fn union(&mut self, x: &Value, y: &Value) -> bool { ... }
}

impl Default for UnionFindBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl RelationBackend for UnionFindBackend {
    fn insert(&mut self, fact: Fact) -> bool {
        self.facts.insert(fact)
    }

    fn contains(&self, fact: &Fact) -> bool {
        self.facts.contains(fact)
    }

    fn iter(&self) -> Vec<Fact> {
        self.facts.iter().cloned().collect()
    }

    fn len(&self) -> usize {
        self.facts.len()
    }

    fn clear(&mut self) {
        self.facts.clear();
    }
}

/// Trie-based backend for prefix matching
///
/// Best for:
/// - Hierarchical data (file paths, resource trees)
/// - Prefix queries
/// - Wildcard matching
///
/// Characteristics:
/// - O(k) prefix lookup (k = key length)
/// - O(k) insertion
/// - Efficient for common prefixes
///
/// Note: Currently stores facts in HashMap but provides foundation
/// for future Trie optimization
#[derive(Debug, Clone)]
pub struct TrieBackend {
    // TODO: Replace with actual Trie structure
    // For now, use HashMap indexed by predicate for efficient filtering
    facts_by_predicate: HashMap<Arc<str>, Vec<Fact>>,
    total_count: usize,
}

impl TrieBackend {
    /// Create a new empty trie backend
    pub fn new() -> Self {
        TrieBackend {
            facts_by_predicate: HashMap::new(),
            total_count: 0,
        }
    }

    /// Create a new trie backend with estimated predicate capacity
    pub fn with_capacity(capacity: usize) -> Self {
        TrieBackend {
            facts_by_predicate: HashMap::with_capacity(capacity / 10), // Estimate predicates
            total_count: 0,
        }
    }

    // TODO: Implement actual Trie operations
    // pub fn insert_path(&mut self, path: Vec<Value>) -> bool { ... }
    // pub fn find_prefix(&self, prefix: &[Value]) -> Vec<Fact> { ... }
}

impl Default for TrieBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl RelationBackend for TrieBackend {
    fn insert(&mut self, fact: Fact) -> bool {
        let predicate = fact.predicate.clone();
        let facts = self.facts_by_predicate.entry(predicate).or_default();

        // Check for duplicates
        if facts.iter().any(|f| f == &fact) {
            return false;
        }

        facts.push(fact);
        self.total_count += 1;
        true
    }

    fn contains(&self, fact: &Fact) -> bool {
        self.facts_by_predicate
            .get(&fact.predicate)
            .map(|facts| facts.iter().any(|f| f == fact))
            .unwrap_or(false)
    }

    fn iter(&self) -> Vec<Fact> {
        self.facts_by_predicate
            .values()
            .flat_map(|facts| facts.iter().cloned())
            .collect()
    }

    fn len(&self) -> usize {
        self.total_count
    }

    fn clear(&mut self) {
        self.facts_by_predicate.clear();
        self.total_count = 0;
    }

    fn filter_by_predicate(&self, predicate: &str) -> Vec<Fact> {
        self.facts_by_predicate.get(predicate).cloned().unwrap_or_default()
    }
}

/// Backend selection based on relation characteristics
pub enum BackendType {
    /// Vector backend for small relations
    Vec,
    /// Hash backend for general-purpose relations
    Hash,
    /// UnionFind backend for transitive closure
    UnionFind,
    /// Trie backend for hierarchical data
    Trie,
}

impl BackendType {
    /// Automatically select backend based on relation name and expected size
    pub fn select_for_relation(predicate: &str, estimated_size: usize) -> Self {
        // Heuristics for backend selection
        match predicate {
            // Transitive closure predicates
            p if p.contains("path")
                || p.contains("reachable")
                || p.contains("ancestor")
                || p.contains("descendant") =>
            {
                BackendType::UnionFind
            }

            // Hierarchical predicates
            p if p.contains("parent")
                || p.contains("child")
                || p.contains("prefix")
                || p.contains("resource") =>
            {
                BackendType::Trie
            }

            // Small relations
            _ if estimated_size < 100 => BackendType::Vec,

            // Default: HashMap for general-purpose
            _ => BackendType::Hash,
        }
    }

    /// Create a hash backend instance
    pub fn create_hash_backend(&self) -> HashBackend {
        match self {
            BackendType::Hash => HashBackend::new(),
            _ => HashBackend::new(), // Default fallback
        }
    }

    /// Create a vector backend instance
    pub fn create_vec_backend(&self) -> VecBackend {
        match self {
            BackendType::Vec => VecBackend::new(),
            _ => VecBackend::new(), // Default fallback
        }
    }

    /// Create a union-find backend instance
    pub fn create_unionfind_backend(&self) -> UnionFindBackend {
        match self {
            BackendType::UnionFind => UnionFindBackend::new(),
            _ => UnionFindBackend::new(), // Default fallback
        }
    }

    /// Create a trie backend instance
    pub fn create_trie_backend(&self) -> TrieBackend {
        match self {
            BackendType::Trie => TrieBackend::new(),
            _ => TrieBackend::new(), // Default fallback
        }
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
    fn test_vec_backend() {
        let mut backend = VecBackend::new();

        assert_eq!(backend.len(), 0);
        assert!(backend.is_empty());

        // Insert new fact
        assert!(backend.insert(test_fact("test", 1)));
        assert_eq!(backend.len(), 1);

        // Duplicate insert should return false
        assert!(!backend.insert(test_fact("test", 1)));
        assert_eq!(backend.len(), 1);

        // Contains check
        assert!(backend.contains(&test_fact("test", 1)));
        assert!(!backend.contains(&test_fact("test", 2)));

        // Insert different fact
        assert!(backend.insert(test_fact("test", 2)));
        assert_eq!(backend.len(), 2);

        // Iteration
        let facts = backend.iter();
        assert_eq!(facts.len(), 2);
    }

    #[test]
    fn test_hash_backend() {
        let mut backend = HashBackend::new();

        assert_eq!(backend.len(), 0);

        // Insert and deduplication
        assert!(backend.insert(test_fact("edge", 1)));
        assert!(!backend.insert(test_fact("edge", 1)));
        assert_eq!(backend.len(), 1);

        // Contains
        assert!(backend.contains(&test_fact("edge", 1)));

        // Multiple inserts
        for i in 2..=10 {
            backend.insert(test_fact("edge", i));
        }
        assert_eq!(backend.len(), 10);
    }

    #[test]
    fn test_unionfind_backend() {
        let mut backend = UnionFindBackend::new();

        // Basic operations
        backend.insert(Fact::binary("path", Value::Integer(1), Value::Integer(2)));
        backend.insert(Fact::binary("path", Value::Integer(2), Value::Integer(3)));

        assert_eq!(backend.len(), 2);
        assert!(backend.contains(&Fact::binary("path", Value::Integer(1), Value::Integer(2))));
    }

    #[test]
    fn test_trie_backend() {
        let mut backend = TrieBackend::new();

        // Insert facts with different predicates
        backend.insert(test_fact("parent", 1));
        backend.insert(test_fact("parent", 2));
        backend.insert(test_fact("child", 3));

        assert_eq!(backend.len(), 3);

        // Predicate filtering
        let parents = backend.filter_by_predicate("parent");
        assert_eq!(parents.len(), 2);

        let children = backend.filter_by_predicate("child");
        assert_eq!(children.len(), 1);
    }

    #[test]
    fn test_backend_selection() {
        // Transitive closure should use UnionFind
        let backend_type = BackendType::select_for_relation("path", 1000);
        assert!(matches!(backend_type, BackendType::UnionFind));

        // Hierarchical should use Trie
        let backend_type = BackendType::select_for_relation("parent_resource", 1000);
        assert!(matches!(backend_type, BackendType::Trie));

        // Small relation should use Vec
        let backend_type = BackendType::select_for_relation("foo", 50);
        assert!(matches!(backend_type, BackendType::Vec));

        // Large general relation should use Hash
        let backend_type = BackendType::select_for_relation("general", 500);
        assert!(matches!(backend_type, BackendType::Hash));
    }

    #[test]
    fn test_relation_backend_trait() {
        // Test that all backends implement the trait correctly
        fn test_backend<B: RelationBackend>(mut backend: B) {
            assert!(backend.is_empty());

            backend.insert(test_fact("test", 1));
            assert_eq!(backend.len(), 1);
            assert!(!backend.is_empty());

            backend.clear();
            assert!(backend.is_empty());
        }

        test_backend(VecBackend::new());
        test_backend(HashBackend::new());
        test_backend(UnionFindBackend::new());
        test_backend(TrieBackend::new());
    }
}
