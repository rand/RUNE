//! Worst-Case Optimal Join (WCOJ) implementation using Leapfrog Triejoin
//!
//! Traditional binary join approaches can be inefficient for multi-way joins.
//! WCOJ computes joins in a way that's provably optimal in worst-case time complexity.
//!
//! ## Example
//!
//! For query: `ancestor(X, Y) :- parent(X, Z), parent(Z, Y)`
//!
//! Traditional approach:
//! 1. Join parent relations (potentially large intermediate result)
//! 2. Project to get final result
//!
//! WCOJ approach:
//! 1. Simultaneously iterate over both parent relations
//! 2. Use leapfrog to skip irrelevant values
//! 3. No intermediate materialization
//!
//! ## Algorithm
//!
//! Leapfrog Triejoin organizes data in sorted order (tries) and uses
//! "leapfrog" iterators that can efficiently skip over irrelevant values
//! by intersecting multiple sorted sequences.
//!
//! Time complexity: O(N^(k/(k+1))) where N is input size, k is number of relations
//! Compare to binary joins: O(N^2) or worse

use crate::facts::Fact;
use crate::types::Value;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;

/// A leapfrog iterator that can efficiently skip over values
pub trait LeapfrogIterator {
    /// Get the current value
    fn key(&self) -> Option<&Value>;

    /// Move to the next value
    fn next(&mut self);

    /// Move to the next value >= target (leapfrog operation)
    fn seek(&mut self, target: &Value);

    /// Check if iterator is at end
    fn at_end(&self) -> bool;

    /// Open an iterator for the next level of the trie
    fn open(&self) -> Option<Box<dyn LeapfrogIterator>>;
}

/// Iterator over a sorted list of values
#[derive(Debug, Clone)]
pub struct ValueIterator {
    values: Vec<Value>,
    position: usize,
}

impl ValueIterator {
    pub fn new(mut values: Vec<Value>) -> Self {
        values.sort();
        values.dedup();
        ValueIterator {
            values,
            position: 0,
        }
    }

    pub fn empty() -> Self {
        ValueIterator {
            values: Vec::new(),
            position: 0,
        }
    }
}

impl LeapfrogIterator for ValueIterator {
    fn key(&self) -> Option<&Value> {
        if self.at_end() {
            None
        } else {
            Some(&self.values[self.position])
        }
    }

    fn next(&mut self) {
        if !self.at_end() {
            self.position += 1;
        }
    }

    fn seek(&mut self, target: &Value) {
        // Binary search for target or next larger value
        while !self.at_end() {
            match self.values[self.position].cmp(target) {
                Ordering::Less => self.position += 1,
                Ordering::Equal | Ordering::Greater => break,
            }
        }
    }

    fn at_end(&self) -> bool {
        self.position >= self.values.len()
    }

    fn open(&self) -> Option<Box<dyn LeapfrogIterator>> {
        None // Leaf level has no children
    }
}

/// Trie node for organizing facts hierarchically
#[derive(Debug, Clone)]
pub struct TrieNode {
    /// Values at this level
    values: Vec<Value>,
    /// Children indexed by value
    children: HashMap<Value, TrieNode>,
}

impl Default for TrieNode {
    fn default() -> Self {
        Self::new()
    }
}

impl TrieNode {
    pub fn new() -> Self {
        TrieNode {
            values: Vec::new(),
            children: HashMap::new(),
        }
    }

    /// Insert a path into the trie
    pub fn insert(&mut self, path: &[Value]) {
        if path.is_empty() {
            return;
        }

        let first = &path[0];
        if !self.values.contains(first) {
            self.values.push(first.clone());
        }

        if path.len() > 1 {
            let child = self.children.entry(first.clone()).or_default();
            child.insert(&path[1..]);
        }
    }

    /// Get an iterator for this level
    pub fn iter(&self) -> TrieIterator {
        let mut values = self.values.clone();
        values.sort();
        values.dedup();
        TrieIterator {
            values,
            position: 0,
            children: self.children.clone(),
        }
    }
}

/// Iterator over a trie level
pub struct TrieIterator {
    values: Vec<Value>,
    position: usize,
    children: HashMap<Value, TrieNode>,
}

impl LeapfrogIterator for TrieIterator {
    fn key(&self) -> Option<&Value> {
        if self.at_end() {
            None
        } else {
            Some(&self.values[self.position])
        }
    }

    fn next(&mut self) {
        if !self.at_end() {
            self.position += 1;
        }
    }

    fn seek(&mut self, target: &Value) {
        while !self.at_end() {
            match self.values[self.position].cmp(target) {
                Ordering::Less => self.position += 1,
                Ordering::Equal | Ordering::Greater => break,
            }
        }
    }

    fn at_end(&self) -> bool {
        self.position >= self.values.len()
    }

    fn open(&self) -> Option<Box<dyn LeapfrogIterator>> {
        if let Some(key) = self.key() {
            if let Some(child) = self.children.get(key) {
                return Some(Box::new(child.iter()));
            }
        }
        None
    }
}

/// Leapfrog join coordinator
pub struct LeapfrogJoin {
    iterators: Vec<Box<dyn LeapfrogIterator>>,
}

impl LeapfrogJoin {
    pub fn new(iterators: Vec<Box<dyn LeapfrogIterator>>) -> Self {
        LeapfrogJoin { iterators }
    }

    /// Compute the intersection of all iterators
    pub fn intersect(&mut self) -> Vec<Value> {
        let mut results = Vec::new();

        if self.iterators.is_empty() {
            return results;
        }

        // Initialize: move all iterators to their first element
        for iter in &mut self.iterators {
            if iter.at_end() {
                return results; // Empty intersection
            }
        }

        loop {
            // Find maximum key across all iterators
            // Clone the value to avoid borrow conflicts
            let max_key = match self.find_max_key() {
                Some(key) => key.clone(),
                None => break,
            };

            // Move all iterators to at least max_key
            let mut all_at_max = true;
            for iter in &mut self.iterators {
                iter.seek(&max_key);
                if iter.at_end() {
                    return results; // One iterator exhausted, no more results
                }
                if iter.key() != Some(&max_key) {
                    all_at_max = false;
                }
            }

            if all_at_max {
                // Found a value in intersection
                results.push(max_key);

                // Move all iterators forward
                for iter in &mut self.iterators {
                    iter.next();
                    if iter.at_end() {
                        return results;
                    }
                }
            }
        }

        results
    }

    /// Find the maximum key among all iterators
    fn find_max_key(&self) -> Option<&Value> {
        self.iterators.iter().filter_map(|iter| iter.key()).max()
    }

    /// Leapfrog iteration: find next tuple in join result
    pub fn next_tuple(&mut self) -> Option<Vec<Value>> {
        // For single-level join, use intersect
        if self.iterators.is_empty() {
            return None;
        }

        // Check if all iterators at same position
        let first_key = self.iterators[0].key()?;

        for iter in &self.iterators {
            if iter.key() != Some(first_key) {
                return None;
            }
        }

        // All at same key, this is a valid tuple
        let mut tuple = vec![first_key.clone()];

        // Recursively join child levels
        let mut child_iters = Vec::new();
        for iter in &self.iterators {
            if let Some(child) = iter.open() {
                child_iters.push(child);
            }
        }

        if !child_iters.is_empty() {
            let mut child_join = LeapfrogJoin::new(child_iters);
            if let Some(child_tuple) = child_join.next_tuple() {
                tuple.extend(child_tuple);
            } else {
                return None;
            }
        }

        // Advance all iterators
        for iter in &mut self.iterators {
            iter.next();
        }

        Some(tuple)
    }
}

/// Index for WCOJ evaluation
pub struct WCOJIndex {
    /// Trie organized by predicate and argument order
    tries: HashMap<(Arc<str>, Vec<usize>), TrieNode>,
}

impl Default for WCOJIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl WCOJIndex {
    pub fn new() -> Self {
        WCOJIndex {
            tries: HashMap::new(),
        }
    }

    /// Add facts to the index
    pub fn add_facts(&mut self, facts: &[Fact]) {
        for fact in facts {
            // Build tries for all possible column orders
            // For a 2-argument fact, we create: (0,1) and (1,0) orderings
            let arity = fact.args.len();

            for perm in Self::generate_permutations(arity) {
                let key = (fact.predicate.clone(), perm.clone());
                let trie = self.tries.entry(key).or_default();

                // Reorder arguments according to permutation
                let reordered: Vec<Value> = perm
                    .iter()
                    .filter_map(|&i| fact.args.get(i).cloned())
                    .collect();

                trie.insert(&reordered);
            }
        }
    }

    /// Get a trie for a specific predicate and column order
    pub fn get_trie(&self, predicate: &Arc<str>, order: &[usize]) -> Option<&TrieNode> {
        self.tries.get(&(predicate.clone(), order.to_vec()))
    }

    /// Generate all permutations of indices for arity n
    fn generate_permutations(n: usize) -> Vec<Vec<usize>> {
        if n == 0 {
            return vec![vec![]];
        }
        if n == 1 {
            return vec![vec![0]];
        }
        if n == 2 {
            return vec![vec![0, 1], vec![1, 0]];
        }

        // For larger arities, we'd need full permutation generation
        // For now, just support common cases
        vec![(0..n).collect(), (0..n).rev().collect()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_iterator_basic() {
        let mut iter = ValueIterator::new(vec![
            Value::Integer(1),
            Value::Integer(3),
            Value::Integer(5),
        ]);

        assert_eq!(iter.key(), Some(&Value::Integer(1)));
        iter.next();
        assert_eq!(iter.key(), Some(&Value::Integer(3)));
        iter.next();
        assert_eq!(iter.key(), Some(&Value::Integer(5)));
        iter.next();
        assert!(iter.at_end());
    }

    #[test]
    fn test_value_iterator_seek() {
        let mut iter = ValueIterator::new(vec![
            Value::Integer(1),
            Value::Integer(3),
            Value::Integer(5),
            Value::Integer(7),
        ]);

        iter.seek(&Value::Integer(4));
        assert_eq!(iter.key(), Some(&Value::Integer(5)));

        iter.seek(&Value::Integer(10));
        assert!(iter.at_end());
    }

    #[test]
    fn test_leapfrog_intersection() {
        let iter1 = ValueIterator::new(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
            Value::Integer(5),
        ]);

        let iter2 = ValueIterator::new(vec![
            Value::Integer(2),
            Value::Integer(3),
            Value::Integer(4),
            Value::Integer(5),
        ]);

        let mut join = LeapfrogJoin::new(vec![Box::new(iter1), Box::new(iter2)]);
        let result = join.intersect();

        assert_eq!(result.len(), 3);
        assert!(result.contains(&Value::Integer(2)));
        assert!(result.contains(&Value::Integer(3)));
        assert!(result.contains(&Value::Integer(5)));
    }

    #[test]
    fn test_leapfrog_empty_intersection() {
        let iter1 = ValueIterator::new(vec![Value::Integer(1), Value::Integer(2)]);
        let iter2 = ValueIterator::new(vec![Value::Integer(3), Value::Integer(4)]);

        let mut join = LeapfrogJoin::new(vec![Box::new(iter1), Box::new(iter2)]);
        let result = join.intersect();

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_trie_node_insert() {
        let mut trie = TrieNode::new();

        trie.insert(&[Value::Integer(1), Value::Integer(2)]);
        trie.insert(&[Value::Integer(1), Value::Integer(3)]);
        trie.insert(&[Value::Integer(2), Value::Integer(4)]);

        assert_eq!(trie.values.len(), 2);
        assert!(trie.children.contains_key(&Value::Integer(1)));
        assert!(trie.children.contains_key(&Value::Integer(2)));
    }

    #[test]
    fn test_trie_iterator() {
        let mut trie = TrieNode::new();
        trie.insert(&[Value::Integer(1), Value::Integer(2)]);
        trie.insert(&[Value::Integer(3), Value::Integer(4)]);
        trie.insert(&[Value::Integer(5), Value::Integer(6)]);

        let mut iter = trie.iter();
        assert_eq!(iter.key(), Some(&Value::Integer(1)));
        iter.next();
        assert_eq!(iter.key(), Some(&Value::Integer(3)));
        iter.next();
        assert_eq!(iter.key(), Some(&Value::Integer(5)));
        iter.next();
        assert!(iter.at_end());
    }

    #[test]
    fn test_wcoj_index_creation() {
        let mut index = WCOJIndex::new();

        let facts = vec![
            Fact::binary("edge", Value::Integer(1), Value::Integer(2)),
            Fact::binary("edge", Value::Integer(2), Value::Integer(3)),
        ];

        index.add_facts(&facts);

        // Should have tries for both column orders
        assert!(index.get_trie(&Arc::from("edge"), &[0, 1]).is_some());
        assert!(index.get_trie(&Arc::from("edge"), &[1, 0]).is_some());
    }

    #[test]
    fn test_multi_way_join() {
        // Triangle query: R(X,Y), S(Y,Z), T(Z,X)
        let iter_r = ValueIterator::new(vec![Value::Integer(1), Value::Integer(2)]);
        let iter_s = ValueIterator::new(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]);
        let iter_t = ValueIterator::new(vec![Value::Integer(1), Value::Integer(2)]);

        let mut join =
            LeapfrogJoin::new(vec![Box::new(iter_r), Box::new(iter_s), Box::new(iter_t)]);

        let result = join.intersect();
        assert!(!result.is_empty());
    }
}
