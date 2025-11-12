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
use crate::types::Value;
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
    /// All facts stored in the backend
    facts: HashSet<Fact>,
    /// Parent pointers for UnionFind structure
    parent: HashMap<Value, Value>,
    /// Rank for union by rank optimization
    rank: HashMap<Value, usize>,
    /// Component size tracking
    size: HashMap<Value, usize>,
}

impl UnionFindBackend {
    /// Create a new empty union-find backend
    pub fn new() -> Self {
        UnionFindBackend {
            facts: HashSet::new(),
            parent: HashMap::new(),
            rank: HashMap::new(),
            size: HashMap::new(),
        }
    }

    /// Create a new union-find backend with pre-allocated capacity
    pub fn with_capacity(capacity: usize) -> Self {
        UnionFindBackend {
            facts: HashSet::with_capacity(capacity),
            parent: HashMap::with_capacity(capacity * 2),
            rank: HashMap::with_capacity(capacity * 2),
            size: HashMap::with_capacity(capacity * 2),
        }
    }

    /// Find the root of the set containing x (with path compression)
    pub fn find(&mut self, x: &Value) -> Value {
        // If x is not in parent map, it's its own parent
        if !self.parent.contains_key(x) {
            self.parent.insert(x.clone(), x.clone());
            self.rank.insert(x.clone(), 0);
            self.size.insert(x.clone(), 1);
            return x.clone();
        }

        let parent = self.parent.get(x).unwrap().clone();
        if parent == *x {
            return x.clone();
        }

        // Path compression: make all nodes point directly to root
        let root = self.find(&parent);
        self.parent.insert(x.clone(), root.clone());
        root
    }

    /// Union two sets (by rank)
    pub fn union(&mut self, x: &Value, y: &Value) -> bool {
        let root_x = self.find(x);
        let root_y = self.find(y);

        if root_x == root_y {
            return false; // Already in same set
        }

        // Union by rank
        let rank_x = self.rank.get(&root_x).copied().unwrap_or(0);
        let rank_y = self.rank.get(&root_y).copied().unwrap_or(0);
        let size_x = self.size.get(&root_x).copied().unwrap_or(1);
        let size_y = self.size.get(&root_y).copied().unwrap_or(1);

        if rank_x < rank_y {
            self.parent.insert(root_x.clone(), root_y.clone());
            self.size.insert(root_y.clone(), size_x + size_y);
        } else if rank_x > rank_y {
            self.parent.insert(root_y.clone(), root_x.clone());
            self.size.insert(root_x.clone(), size_x + size_y);
        } else {
            self.parent.insert(root_y.clone(), root_x.clone());
            self.rank.insert(root_x.clone(), rank_x + 1);
            self.size.insert(root_x.clone(), size_x + size_y);
        }

        true
    }

    /// Check if two values are in the same component
    pub fn connected(&mut self, x: &Value, y: &Value) -> bool {
        self.find(x) == self.find(y)
    }

    /// Get all values in the same component as x
    pub fn get_component(&mut self, x: &Value) -> Vec<Value> {
        let root = self.find(x);
        let mut component = Vec::new();

        // Collect keys first to avoid borrow checker issues
        let keys: Vec<Value> = self.parent.keys().cloned().collect();
        for key in keys {
            if self.find(&key) == root {
                component.push(key);
            }
        }

        component
    }

    /// Get size of the component containing x
    pub fn component_size(&mut self, x: &Value) -> usize {
        let root = self.find(x);
        self.size.get(&root).copied().unwrap_or(1)
    }

    /// Process a fact for UnionFind operations
    fn process_fact(&mut self, fact: &Fact) {
        // For binary predicates (edges), perform union
        if fact.args.len() == 2 {
            self.union(&fact.args[0], &fact.args[1]);
        }
        // For unary predicates or other arities, just ensure the values are in the structure
        else {
            for arg in fact.args.iter() {
                self.find(arg);
            }
        }
    }
}

impl Default for UnionFindBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl RelationBackend for UnionFindBackend {
    fn insert(&mut self, fact: Fact) -> bool {
        let is_new = self.facts.insert(fact.clone());
        if is_new {
            self.process_fact(&fact);
        }
        is_new
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
        self.parent.clear();
        self.rank.clear();
        self.size.clear();
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
/// Implementation uses a trie data structure for efficient prefix operations
#[derive(Debug, Clone)]
pub struct TrieBackend {
    root: TrieNode,
    total_count: usize,
}

/// A node in the trie structure
#[derive(Debug, Clone)]
struct TrieNode {
    /// Children nodes indexed by value
    children: HashMap<Value, TrieNode>,
    /// Facts that terminate at this node
    facts: Vec<Fact>,
    /// Count of all facts in this subtree
    subtree_count: usize,
}

impl TrieNode {
    fn new() -> Self {
        TrieNode {
            children: HashMap::new(),
            facts: Vec::new(),
            subtree_count: 0,
        }
    }

    /// Insert a fact into the trie starting from this node
    fn insert(&mut self, fact: &Fact, path: &[Value]) -> bool {
        if path.is_empty() {
            // Check for duplicates
            if self.facts.iter().any(|f| f == fact) {
                return false;
            }
            self.facts.push(fact.clone());
            self.subtree_count += 1;
            return true;
        }

        // Navigate to child or create it
        let first = &path[0];
        let child = self.children.entry(first.clone()).or_insert_with(TrieNode::new);

        // Recursively insert
        if child.insert(fact, &path[1..]) {
            self.subtree_count += 1;
            true
        } else {
            false
        }
    }

    /// Find all facts with the given prefix
    fn find_prefix(&self, prefix: &[Value]) -> Vec<Fact> {
        if prefix.is_empty() {
            // Return all facts in this subtree
            return self.collect_all();
        }

        // Navigate to the prefix node
        if let Some(child) = self.children.get(&prefix[0]) {
            child.find_prefix(&prefix[1..])
        } else {
            Vec::new()
        }
    }

    /// Collect all facts in this subtree
    fn collect_all(&self) -> Vec<Fact> {
        let mut result = self.facts.clone();
        for child in self.children.values() {
            result.extend(child.collect_all());
        }
        result
    }

    /// Check if a fact exists in the trie
    fn contains(&self, fact: &Fact, path: &[Value]) -> bool {
        if path.is_empty() {
            return self.facts.iter().any(|f| f == fact);
        }

        if let Some(child) = self.children.get(&path[0]) {
            child.contains(fact, &path[1..])
        } else {
            false
        }
    }
}

impl TrieBackend {
    /// Create a new empty trie backend
    pub fn new() -> Self {
        TrieBackend {
            root: TrieNode::new(),
            total_count: 0,
        }
    }

    /// Create a new trie backend with estimated capacity (for compatibility)
    pub fn with_capacity(_capacity: usize) -> Self {
        // Trie doesn't pre-allocate, but we keep the method for API compatibility
        Self::new()
    }

    /// Build path from fact's predicate and arguments
    fn fact_to_path(fact: &Fact) -> Vec<Value> {
        let mut path = Vec::with_capacity(fact.args.len() + 1);
        path.push(Value::String(fact.predicate.clone()));
        path.extend_from_slice(&fact.args);
        path
    }

    /// Insert a fact using its arguments as the path
    pub fn insert_path(&mut self, fact: &Fact) -> bool {
        let path = Self::fact_to_path(fact);
        if self.root.insert(fact, &path) {
            self.total_count += 1;
            true
        } else {
            false
        }
    }

    /// Find all facts matching a prefix pattern
    pub fn find_prefix(&self, prefix: &[Value]) -> Vec<Fact> {
        self.root.find_prefix(prefix)
    }

    /// Find all facts with a given predicate prefix
    pub fn find_predicate_prefix(&self, predicate: &str) -> Vec<Fact> {
        let prefix = vec![Value::String(Arc::from(predicate))];
        self.find_prefix(&prefix)
    }

    /// Find facts matching a path pattern (with wildcards represented as None)
    pub fn find_pattern(&self, pattern: &[Option<Value>]) -> Vec<Fact> {
        self.find_pattern_helper(&self.root, pattern, 0)
    }

    fn find_pattern_helper(&self, node: &TrieNode, pattern: &[Option<Value>], depth: usize) -> Vec<Fact> {
        if depth >= pattern.len() {
            return node.facts.clone();
        }

        let mut results = Vec::new();

        match &pattern[depth] {
            Some(value) => {
                // Exact match required
                if let Some(child) = node.children.get(value) {
                    results.extend(self.find_pattern_helper(child, pattern, depth + 1));
                }
            }
            None => {
                // Wildcard - explore all children
                for child in node.children.values() {
                    results.extend(self.find_pattern_helper(child, pattern, depth + 1));
                }
            }
        }

        // Also check if pattern terminates here
        if depth == pattern.len() - 1 {
            results.extend(node.facts.clone());
        }

        results
    }
}

impl Default for TrieBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl RelationBackend for TrieBackend {
    fn insert(&mut self, fact: Fact) -> bool {
        self.insert_path(&fact)
    }

    fn contains(&self, fact: &Fact) -> bool {
        let path = Self::fact_to_path(fact);
        self.root.contains(fact, &path)
    }

    fn iter(&self) -> Vec<Fact> {
        self.root.collect_all()
    }

    fn len(&self) -> usize {
        self.total_count
    }

    fn clear(&mut self) {
        self.root = TrieNode::new();
        self.total_count = 0;
    }

    fn filter_by_predicate(&self, predicate: &str) -> Vec<Fact> {
        self.find_predicate_prefix(predicate)
    }
}

/// Backend selection based on relation characteristics
#[derive(Debug, Clone, PartialEq)]
pub enum BackendType {
    /// Vector backend for small relations
    Vec,
    /// Hash backend for general-purpose relations
    Hash,
    /// UnionFind backend for transitive closure
    UnionFind,
    /// Trie backend for hierarchical data
    Trie,
    /// Worst-case optimal join backend for multi-way joins
    WCOJ,
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

    #[test]
    fn test_vec_backend_with_capacity() {
        let backend = VecBackend::with_capacity(100);
        assert_eq!(backend.len(), 0);
        assert!(backend.is_empty());
    }

    #[test]
    fn test_vec_backend_default() {
        let backend = VecBackend::default();
        assert_eq!(backend.len(), 0);
        assert!(backend.is_empty());
    }

    #[test]
    fn test_hash_backend_with_capacity() {
        let mut backend = HashBackend::with_capacity(100);
        backend.insert(test_fact("test", 1));
        assert_eq!(backend.len(), 1);
    }

    #[test]
    fn test_hash_backend_from_set() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(test_fact("edge", 1));
        set.insert(test_fact("edge", 2));

        let backend = HashBackend::from_set(set);
        assert_eq!(backend.len(), 2);
        assert!(backend.contains(&test_fact("edge", 1)));
        assert!(backend.contains(&test_fact("edge", 2)));
    }

    #[test]
    fn test_hash_backend_default() {
        let backend = HashBackend::default();
        assert_eq!(backend.len(), 0);
        assert!(backend.is_empty());
    }

    #[test]
    fn test_unionfind_backend_with_capacity() {
        let backend = UnionFindBackend::with_capacity(100);
        assert_eq!(backend.len(), 0);
        assert!(backend.is_empty());
    }

    #[test]
    fn test_unionfind_backend_default() {
        let backend = UnionFindBackend::default();
        assert_eq!(backend.len(), 0);
        assert!(backend.is_empty());
    }

    #[test]
    fn test_trie_backend_with_capacity() {
        let backend = TrieBackend::with_capacity(100);
        assert_eq!(backend.len(), 0);
        assert!(backend.is_empty());
    }

    #[test]
    fn test_trie_backend_default() {
        let backend = TrieBackend::default();
        assert_eq!(backend.len(), 0);
        assert!(backend.is_empty());
    }

    #[test]
    fn test_backend_extend() {
        let mut backend1 = VecBackend::new();
        backend1.insert(test_fact("test", 1));
        backend1.insert(test_fact("test", 2));

        let mut backend2 = VecBackend::new();
        backend2.insert(test_fact("test", 3));
        backend2.insert(test_fact("test", 4));

        backend1.extend(&backend2);
        assert_eq!(backend1.len(), 4);
        assert!(backend1.contains(&test_fact("test", 3)));
        assert!(backend1.contains(&test_fact("test", 4)));
    }

    #[test]
    fn test_backend_extend_with_duplicates() {
        let mut backend1 = HashBackend::new();
        backend1.insert(test_fact("test", 1));
        backend1.insert(test_fact("test", 2));

        let mut backend2 = HashBackend::new();
        backend2.insert(test_fact("test", 2)); // Duplicate
        backend2.insert(test_fact("test", 3));

        backend1.extend(&backend2);
        assert_eq!(backend1.len(), 3); // Should not add duplicate
    }

    #[test]
    fn test_filter_by_predicate_empty() {
        let backend = VecBackend::new();
        let facts = backend.filter_by_predicate("nonexistent");
        assert!(facts.is_empty());
    }

    #[test]
    fn test_filter_by_predicate_multiple() {
        let mut backend = HashBackend::new();
        backend.insert(test_fact("edge", 1));
        backend.insert(test_fact("edge", 2));
        backend.insert(test_fact("node", 3));
        backend.insert(test_fact("node", 4));
        backend.insert(test_fact("path", 5));

        let edges = backend.filter_by_predicate("edge");
        assert_eq!(edges.len(), 2);

        let nodes = backend.filter_by_predicate("node");
        assert_eq!(nodes.len(), 2);

        let paths = backend.filter_by_predicate("path");
        assert_eq!(paths.len(), 1);
    }

    #[test]
    fn test_trie_backend_predicate_filtering() {
        let mut backend = TrieBackend::new();

        // Insert facts with different predicates
        for i in 0..5 {
            backend.insert(test_fact("pred_a", i));
        }
        for i in 5..10 {
            backend.insert(test_fact("pred_b", i));
        }

        assert_eq!(backend.len(), 10);

        let pred_a_facts = backend.filter_by_predicate("pred_a");
        assert_eq!(pred_a_facts.len(), 5);

        let pred_b_facts = backend.filter_by_predicate("pred_b");
        assert_eq!(pred_b_facts.len(), 5);

        let nonexistent = backend.filter_by_predicate("pred_c");
        assert_eq!(nonexistent.len(), 0);
    }

    #[test]
    fn test_clear_operations() {
        // Test clear on all backend types
        let mut vec_backend = VecBackend::new();
        vec_backend.insert(test_fact("test", 1));
        vec_backend.clear();
        assert_eq!(vec_backend.len(), 0);

        let mut hash_backend = HashBackend::new();
        hash_backend.insert(test_fact("test", 1));
        hash_backend.clear();
        assert_eq!(hash_backend.len(), 0);

        let mut unionfind_backend = UnionFindBackend::new();
        unionfind_backend.insert(test_fact("test", 1));
        unionfind_backend.clear();
        assert_eq!(unionfind_backend.len(), 0);

        let mut trie_backend = TrieBackend::new();
        trie_backend.insert(test_fact("test", 1));
        trie_backend.clear();
        assert_eq!(trie_backend.len(), 0);
    }

    #[test]
    fn test_backend_selection_edge_cases() {
        // Test backend selection with various edge cases
        let backend_type = BackendType::select_for_relation("", 50);
        assert!(matches!(backend_type, BackendType::Vec));

        let backend_type = BackendType::select_for_relation("complex_path_relation", 10000);
        assert!(matches!(backend_type, BackendType::UnionFind));

        let backend_type = BackendType::select_for_relation("parent_child_tree", 5000);
        assert!(matches!(backend_type, BackendType::Trie));

        let backend_type = BackendType::select_for_relation("descendant", 1000);
        assert!(matches!(backend_type, BackendType::UnionFind));

        let backend_type = BackendType::select_for_relation("prefix_search", 1000);
        assert!(matches!(backend_type, BackendType::Trie));

        let backend_type = BackendType::select_for_relation("resource_hierarchy", 1000);
        assert!(matches!(backend_type, BackendType::Trie));
    }

    #[test]
    fn test_large_dataset() {
        // Test with larger dataset to ensure performance
        let mut backend = HashBackend::new();

        // Insert 1000 facts
        for i in 0..1000 {
            backend.insert(test_fact("large", i));
        }

        assert_eq!(backend.len(), 1000);

        // Test contains on various facts
        assert!(backend.contains(&test_fact("large", 0)));
        assert!(backend.contains(&test_fact("large", 500)));
        assert!(backend.contains(&test_fact("large", 999)));
        assert!(!backend.contains(&test_fact("large", 1000)));
    }

    #[test]
    fn test_vec_backend_iteration_order() {
        let mut backend = VecBackend::new();

        // Insert facts in specific order
        backend.insert(test_fact("test", 3));
        backend.insert(test_fact("test", 1));
        backend.insert(test_fact("test", 2));

        let facts = backend.iter();
        assert_eq!(facts.len(), 3);
        // VecBackend should preserve insertion order
        assert_eq!(facts[0], test_fact("test", 3));
        assert_eq!(facts[1], test_fact("test", 1));
        assert_eq!(facts[2], test_fact("test", 2));
    }

    #[test]
    fn test_clone_backends() {
        // Test that cloning works correctly for all backends
        let mut original = VecBackend::new();
        original.insert(test_fact("test", 1));

        let mut cloned = original.clone();
        cloned.insert(test_fact("test", 2));

        // Original should not be affected
        assert_eq!(original.len(), 1);
        assert_eq!(cloned.len(), 2);
    }

    #[test]
    fn test_trie_backend_prefix_lookup() {
        let mut backend = TrieBackend::new();

        // Insert facts with hierarchical structure
        backend.insert(Fact::new(
            "file".to_string(),
            vec![
                Value::String(Arc::from("/usr")),
                Value::String(Arc::from("dir")),
            ],
        ));
        backend.insert(Fact::new(
            "file".to_string(),
            vec![
                Value::String(Arc::from("/usr/bin")),
                Value::String(Arc::from("dir")),
            ],
        ));
        backend.insert(Fact::new(
            "file".to_string(),
            vec![
                Value::String(Arc::from("/usr/bin/ls")),
                Value::String(Arc::from("file")),
            ],
        ));
        backend.insert(Fact::new(
            "file".to_string(),
            vec![
                Value::String(Arc::from("/usr/local")),
                Value::String(Arc::from("dir")),
            ],
        ));
        backend.insert(Fact::new(
            "permission".to_string(),
            vec![
                Value::String(Arc::from("/usr")),
                Value::String(Arc::from("read")),
            ],
        ));

        // Test predicate prefix lookup
        let file_facts = backend.find_predicate_prefix("file");
        assert_eq!(file_facts.len(), 4);

        // Test full prefix lookup
        let usr_prefix = vec![
            Value::String(Arc::from("file")),
            Value::String(Arc::from("/usr")),
        ];
        let usr_facts = backend.find_prefix(&usr_prefix);
        assert_eq!(usr_facts.len(), 1); // Only exact "/usr" fact

        // Test pattern matching with wildcards
        let pattern = vec![
            Some(Value::String(Arc::from("file"))),
            None, // Any first argument
            Some(Value::String(Arc::from("dir"))),
        ];
        let dir_facts = backend.find_pattern(&pattern);
        assert_eq!(dir_facts.len(), 3); // Three directory facts

        // Verify the directory facts
        for fact in &dir_facts {
            assert_eq!(fact.predicate, Arc::from("file"));
            assert_eq!(fact.args[1], Value::String(Arc::from("dir")));
        }
    }

    #[test]
    fn test_trie_backend_pattern_matching() {
        let mut backend = TrieBackend::new();

        // Insert various facts
        backend.insert(Fact::new(
            "edge".to_string(),
            vec![
                Value::Integer(1),
                Value::Integer(2),
            ],
        ));
        backend.insert(Fact::new(
            "edge".to_string(),
            vec![
                Value::Integer(2),
                Value::Integer(3),
            ],
        ));
        backend.insert(Fact::new(
            "edge".to_string(),
            vec![
                Value::Integer(1),
                Value::Integer(3),
            ],
        ));
        backend.insert(Fact::new(
            "node".to_string(),
            vec![
                Value::Integer(1),
                Value::String(Arc::from("start")),
            ],
        ));

        // Pattern: edge(1, ?)
        let pattern = vec![
            Some(Value::String(Arc::from("edge"))),
            Some(Value::Integer(1)),
            None,
        ];
        let edges_from_1 = backend.find_pattern(&pattern);
        assert_eq!(edges_from_1.len(), 2); // edge(1,2) and edge(1,3)

        // Pattern: edge(?, 3)
        let pattern = vec![
            Some(Value::String(Arc::from("edge"))),
            None,
            Some(Value::Integer(3)),
        ];
        let edges_to_3 = backend.find_pattern(&pattern);
        assert_eq!(edges_to_3.len(), 2); // edge(2,3) and edge(1,3)

        // Pattern: node(?, ?)
        let pattern = vec![
            Some(Value::String(Arc::from("node"))),
            None,
            None,
        ];
        let all_nodes = backend.find_pattern(&pattern);
        assert_eq!(all_nodes.len(), 1);
    }

    #[test]
    fn test_trie_backend_duplicate_handling() {
        let mut backend = TrieBackend::new();

        let fact = Fact::new(
            "test".to_string(),
            vec![Value::Integer(1), Value::Integer(2)],
        );

        // First insert should succeed
        assert!(backend.insert(fact.clone()));
        assert_eq!(backend.len(), 1);

        // Duplicate insert should return false
        assert!(!backend.insert(fact.clone()));
        assert_eq!(backend.len(), 1);
    }

    #[test]
    fn test_trie_backend_empty_patterns() {
        let mut backend = TrieBackend::new();

        backend.insert(Fact::new(
            "fact1".to_string(),
            vec![],
        ));
        backend.insert(Fact::new(
            "fact2".to_string(),
            vec![Value::Integer(1)],
        ));

        // Empty prefix should return all facts
        let all = backend.find_prefix(&[]);
        assert_eq!(all.len(), 2);

        // Pattern with just predicate
        let pattern = vec![Some(Value::String(Arc::from("fact1")))];
        let results = backend.find_pattern(&pattern);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_unionfind_backend_transitive_closure() {
        let mut backend = UnionFindBackend::new();

        // Create a graph: 1->2, 2->3, 4->5
        backend.insert(Fact::new(
            "edge".to_string(),
            vec![Value::Integer(1), Value::Integer(2)],
        ));
        backend.insert(Fact::new(
            "edge".to_string(),
            vec![Value::Integer(2), Value::Integer(3)],
        ));
        backend.insert(Fact::new(
            "edge".to_string(),
            vec![Value::Integer(4), Value::Integer(5)],
        ));

        // Check connectivity
        assert!(backend.connected(&Value::Integer(1), &Value::Integer(2)));
        assert!(backend.connected(&Value::Integer(2), &Value::Integer(3)));
        assert!(backend.connected(&Value::Integer(1), &Value::Integer(3))); // Transitive
        assert!(backend.connected(&Value::Integer(4), &Value::Integer(5)));
        assert!(!backend.connected(&Value::Integer(1), &Value::Integer(4))); // Different components

        // Check component sizes
        assert_eq!(backend.component_size(&Value::Integer(1)), 3);
        assert_eq!(backend.component_size(&Value::Integer(4)), 2);
    }

    #[test]
    fn test_unionfind_backend_find_and_union() {
        let mut backend = UnionFindBackend::new();

        // Initially, each value is its own parent
        let a = Value::Integer(1);
        let b = Value::Integer(2);
        let c = Value::Integer(3);

        assert_eq!(backend.find(&a), a);
        assert_eq!(backend.find(&b), b);

        // Union a and b
        assert!(backend.union(&a, &b));
        assert!(backend.connected(&a, &b));

        // Union should return false for already connected elements
        assert!(!backend.union(&a, &b));

        // Union b and c (should connect all three)
        assert!(backend.union(&b, &c));
        assert!(backend.connected(&a, &c));
    }

    #[test]
    fn test_unionfind_backend_get_component() {
        let mut backend = UnionFindBackend::new();

        // Create two separate components
        backend.insert(Fact::new(
            "edge".to_string(),
            vec![Value::Integer(1), Value::Integer(2)],
        ));
        backend.insert(Fact::new(
            "edge".to_string(),
            vec![Value::Integer(2), Value::Integer(3)],
        ));
        backend.insert(Fact::new(
            "edge".to_string(),
            vec![Value::Integer(4), Value::Integer(5)],
        ));

        // Get component containing 1
        let component1 = backend.get_component(&Value::Integer(1));
        assert_eq!(component1.len(), 3);
        assert!(component1.contains(&Value::Integer(1)));
        assert!(component1.contains(&Value::Integer(2)));
        assert!(component1.contains(&Value::Integer(3)));

        // Get component containing 4
        let component2 = backend.get_component(&Value::Integer(4));
        assert_eq!(component2.len(), 2);
        assert!(component2.contains(&Value::Integer(4)));
        assert!(component2.contains(&Value::Integer(5)));
    }

    #[test]
    fn test_unionfind_backend_equivalence_relation() {
        let mut backend = UnionFindBackend::new();

        // Create equivalence classes
        backend.insert(Fact::new(
            "equal".to_string(),
            vec![
                Value::String(Arc::from("x")),
                Value::String(Arc::from("y")),
            ],
        ));
        backend.insert(Fact::new(
            "equal".to_string(),
            vec![
                Value::String(Arc::from("y")),
                Value::String(Arc::from("z")),
            ],
        ));
        backend.insert(Fact::new(
            "equal".to_string(),
            vec![
                Value::String(Arc::from("a")),
                Value::String(Arc::from("b")),
            ],
        ));

        // Check equivalence transitivity
        assert!(backend.connected(&Value::String(Arc::from("x")), &Value::String(Arc::from("z"))));
        assert!(!backend.connected(&Value::String(Arc::from("x")), &Value::String(Arc::from("a"))));
    }

    #[test]
    fn test_unionfind_backend_path_compression() {
        let mut backend = UnionFindBackend::new();

        // Create a long chain: 1->2->3->4->5
        for i in 1..5 {
            backend.insert(Fact::new(
                "edge".to_string(),
                vec![Value::Integer(i), Value::Integer(i + 1)],
            ));
        }

        // Multiple finds should trigger path compression
        let root = backend.find(&Value::Integer(1));
        assert_eq!(backend.find(&Value::Integer(1)), root);
        assert_eq!(backend.find(&Value::Integer(2)), root);
        assert_eq!(backend.find(&Value::Integer(5)), root);

        // All should be in same component
        assert!(backend.connected(&Value::Integer(1), &Value::Integer(5)));
        assert_eq!(backend.component_size(&Value::Integer(1)), 5);
    }

    #[test]
    fn test_unionfind_backend_non_binary_facts() {
        let mut backend = UnionFindBackend::new();

        // Non-binary facts should just add values to the structure
        backend.insert(Fact::new(
            "node".to_string(),
            vec![Value::Integer(1)],
        ));
        backend.insert(Fact::new(
            "triple".to_string(),
            vec![Value::Integer(2), Value::Integer(3), Value::Integer(4)],
        ));

        // Values should exist but be in separate components
        assert_eq!(backend.find(&Value::Integer(1)), Value::Integer(1));
        assert_eq!(backend.find(&Value::Integer(2)), Value::Integer(2));
        assert!(!backend.connected(&Value::Integer(1), &Value::Integer(2)));
    }
}
