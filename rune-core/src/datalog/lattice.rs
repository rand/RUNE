//! Lattice types for monotonic Datalog
//!
//! Lattices enable monotonic aggregation and CRDT-style distributed computation.
//! A lattice is a partially ordered set with a join operation (least upper bound).
//!
//! ## Properties
//!
//! For a lattice (L, ⊔), the join operation must satisfy:
//! - **Commutative**: a ⊔ b = b ⊔ a
//! - **Associative**: (a ⊔ b) ⊔ c = a ⊔ (b ⊔ c)
//! - **Idempotent**: a ⊔ a = a
//!
//! ## Use Cases
//!
//! - **Monotonic aggregation**: count, sum, min, max
//! - **Distributed computation**: CRDTs (Conflict-free Replicated Data Types)
//! - **Program analysis**: Abstract interpretation domains
//! - **Fixed-point computation**: Guaranteed termination with ascending chains
//!
//! ## Example
//!
//! ```rust
//! use rune_core::datalog::lattice::{Lattice, MaxLattice};
//!
//! let a = MaxLattice::new(5);
//! let b = MaxLattice::new(10);
//! let c = a.join(&b);
//! assert_eq!(c.value(), 10); // max(5, 10) = 10
//! ```

use crate::types::Value;
use std::collections::BTreeSet;
use std::sync::Arc;

/// Trait for lattice types with monotonic join operation
pub trait Lattice: Clone + PartialEq {
    /// Compute the least upper bound (join) of two lattice elements
    ///
    /// Properties:
    /// - Commutative: a.join(b) == b.join(a)
    /// - Associative: a.join(b).join(c) == a.join(b.join(c))
    /// - Idempotent: a.join(a) == a
    fn join(&self, other: &Self) -> Self;

    /// Check if this element is less than or equal to another (partial order)
    ///
    /// a ≤ b ⟺ a ⊔ b = b
    fn less_than_or_equal(&self, other: &Self) -> bool {
        self.join(other) == *other
    }

    /// Get the bottom element (least element) if it exists
    fn bottom() -> Option<Self>
    where
        Self: Sized,
    {
        None
    }
}

/// Lattice of integers with max as join operation
///
/// Useful for tracking maximum values, high-water marks, version numbers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MaxLattice<T: Ord + Clone> {
    value: T,
}

impl<T: Ord + Clone> MaxLattice<T> {
    /// Create a new MaxLattice with the given value
    pub fn new(value: T) -> Self {
        MaxLattice { value }
    }

    /// Get the underlying value
    pub fn value(&self) -> &T {
        &self.value
    }
}

impl<T: Ord + Clone> Lattice for MaxLattice<T> {
    fn join(&self, other: &Self) -> Self {
        MaxLattice {
            value: self.value.clone().max(other.value.clone()),
        }
    }
}

/// Lattice of integers with min as join operation
///
/// Useful for tracking minimum values, costs, distances.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MinLattice<T: Ord + Clone> {
    value: T,
}

impl<T: Ord + Clone> MinLattice<T> {
    /// Create a new MinLattice with the given value
    pub fn new(value: T) -> Self {
        MinLattice { value }
    }

    /// Get the underlying value
    pub fn value(&self) -> &T {
        &self.value
    }
}

impl<T: Ord + Clone> Lattice for MinLattice<T> {
    fn join(&self, other: &Self) -> Self {
        MinLattice {
            value: self.value.clone().min(other.value.clone()),
        }
    }
}

/// Lattice of sets with union as join operation
///
/// Useful for collecting facts, tracking dependencies, accumulating results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetLattice<T: Ord + Clone> {
    elements: BTreeSet<T>,
}

impl<T: Ord + Clone> SetLattice<T> {
    /// Create a new empty SetLattice
    pub fn new() -> Self {
        SetLattice {
            elements: BTreeSet::new(),
        }
    }

    /// Create a SetLattice from an iterator
    pub fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        SetLattice {
            elements: iter.into_iter().collect(),
        }
    }

    /// Insert an element into the set
    pub fn insert(&mut self, element: T) {
        self.elements.insert(element);
    }

    /// Check if the set contains an element
    pub fn contains(&self, element: &T) -> bool {
        self.elements.contains(element)
    }

    /// Get the number of elements in the set
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Check if the set is empty
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Iterate over the elements
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.elements.iter()
    }
}

impl<T: Ord + Clone> Default for SetLattice<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Ord + Clone> Lattice for SetLattice<T> {
    fn join(&self, other: &Self) -> Self {
        let mut elements = self.elements.clone();
        elements.extend(other.elements.iter().cloned());
        SetLattice { elements }
    }

    fn bottom() -> Option<Self> {
        Some(SetLattice::new())
    }
}

/// Lattice of natural numbers with addition as join operation
///
/// Useful for counting, accumulating weights, tracking multiplicities.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CounterLattice {
    count: u64,
}

impl CounterLattice {
    /// Create a new CounterLattice with the given count
    pub fn new(count: u64) -> Self {
        CounterLattice { count }
    }

    /// Get the current count
    pub fn count(&self) -> u64 {
        self.count
    }

    /// Increment the counter by 1
    pub fn increment(&mut self) {
        self.count = self.count.saturating_add(1);
    }

    /// Add a value to the counter
    pub fn add(&mut self, value: u64) {
        self.count = self.count.saturating_add(value);
    }
}

impl Lattice for CounterLattice {
    fn join(&self, other: &Self) -> Self {
        CounterLattice {
            count: self.count.saturating_add(other.count),
        }
    }

    fn bottom() -> Option<Self> {
        Some(CounterLattice::new(0))
    }
}

/// Lattice of boolean values with OR as join operation
///
/// Useful for tracking "has occurred" events, flags, existence checks.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BoolLattice {
    value: bool,
}

impl BoolLattice {
    /// Create a new BoolLattice with the given value
    pub fn new(value: bool) -> Self {
        BoolLattice { value }
    }

    /// Get the underlying boolean value
    pub fn value(&self) -> bool {
        self.value
    }
}

impl Lattice for BoolLattice {
    fn join(&self, other: &Self) -> Self {
        BoolLattice {
            value: self.value || other.value,
        }
    }

    fn bottom() -> Option<Self> {
        Some(BoolLattice::new(false))
    }
}

/// Lattice value that can be embedded in the Datalog type system
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LatticeValue {
    /// Maximum of integers
    Max(i64),
    /// Minimum of integers
    Min(i64),
    /// Set of values
    Set(Arc<BTreeSet<Value>>),
    /// Counter (natural number with addition)
    Counter(u64),
    /// Boolean with OR
    Bool(bool),
}

impl LatticeValue {
    /// Create a max lattice value
    pub fn max(value: i64) -> Self {
        LatticeValue::Max(value)
    }

    /// Create a min lattice value
    pub fn min(value: i64) -> Self {
        LatticeValue::Min(value)
    }

    /// Create a set lattice value
    pub fn set<I: IntoIterator<Item = Value>>(iter: I) -> Self {
        LatticeValue::Set(Arc::new(iter.into_iter().collect()))
    }

    /// Create a counter lattice value
    pub fn counter(count: u64) -> Self {
        LatticeValue::Counter(count)
    }

    /// Create a boolean lattice value
    pub fn bool(value: bool) -> Self {
        LatticeValue::Bool(value)
    }

    /// Join two lattice values
    pub fn join(&self, other: &Self) -> Option<Self> {
        match (self, other) {
            (LatticeValue::Max(a), LatticeValue::Max(b)) => Some(LatticeValue::Max(*a.max(b))),
            (LatticeValue::Min(a), LatticeValue::Min(b)) => Some(LatticeValue::Min(*a.min(b))),
            (LatticeValue::Set(a), LatticeValue::Set(b)) => {
                let mut elements = (**a).clone();
                elements.extend((**b).iter().cloned());
                Some(LatticeValue::Set(Arc::new(elements)))
            }
            (LatticeValue::Counter(a), LatticeValue::Counter(b)) => {
                Some(LatticeValue::Counter(a.saturating_add(*b)))
            }
            (LatticeValue::Bool(a), LatticeValue::Bool(b)) => Some(LatticeValue::Bool(*a || *b)),
            _ => None, // Incompatible lattice types
        }
    }

    /// Check if this value is less than or equal to another
    pub fn less_than_or_equal(&self, other: &Self) -> bool {
        match (self, other) {
            (LatticeValue::Max(a), LatticeValue::Max(b)) => a <= b,
            (LatticeValue::Min(a), LatticeValue::Min(b)) => a >= b,
            (LatticeValue::Set(a), LatticeValue::Set(b)) => a.is_subset(b),
            (LatticeValue::Counter(a), LatticeValue::Counter(b)) => a <= b,
            (LatticeValue::Bool(a), LatticeValue::Bool(b)) => !a || *b,
            _ => false,
        }
    }

    /// Convert to a regular Value for storage
    pub fn to_value(&self) -> Value {
        match self {
            LatticeValue::Max(v) => Value::Integer(*v),
            LatticeValue::Min(v) => Value::Integer(*v),
            LatticeValue::Set(s) => Value::Array(Arc::from(s.iter().cloned().collect::<Vec<_>>())),
            LatticeValue::Counter(c) => Value::Integer(*c as i64),
            LatticeValue::Bool(b) => Value::Bool(*b),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_lattice() {
        let a = MaxLattice::new(5);
        let b = MaxLattice::new(10);
        let c = a.join(&b);
        assert_eq!(c.value(), &10);

        // Commutative
        assert_eq!(a.join(&b), b.join(&a));

        // Idempotent
        assert_eq!(a.join(&a), a);

        // Partial order
        assert!(a.less_than_or_equal(&c));
        assert!(b.less_than_or_equal(&c));
    }

    #[test]
    fn test_min_lattice() {
        let a = MinLattice::new(5);
        let b = MinLattice::new(10);
        let c = a.join(&b);
        assert_eq!(c.value(), &5);

        // Commutative
        assert_eq!(a.join(&b), b.join(&a));

        // Idempotent
        assert_eq!(a.join(&a), a);
    }

    #[test]
    fn test_set_lattice() {
        let a = SetLattice::from_iter([1, 2, 3]);
        let b = SetLattice::from_iter([3, 4, 5]);
        let c = a.join(&b);

        assert_eq!(c.len(), 5);
        assert!(c.contains(&1));
        assert!(c.contains(&5));

        // Commutative
        assert_eq!(a.join(&b), b.join(&a));

        // Idempotent
        assert_eq!(a.join(&a), a);

        // Bottom
        let bottom = SetLattice::<i32>::bottom().unwrap();
        assert!(bottom.is_empty());
        assert_eq!(a.join(&bottom), a);
    }

    #[test]
    fn test_counter_lattice() {
        let a = CounterLattice::new(5);
        let b = CounterLattice::new(10);
        let c = a.join(&b);
        assert_eq!(c.count(), 15);

        // Commutative
        assert_eq!(a.join(&b), b.join(&a));

        // Bottom
        let bottom = CounterLattice::bottom().unwrap();
        assert_eq!(bottom.count(), 0);
        assert_eq!(a.join(&bottom), a);
    }

    #[test]
    fn test_bool_lattice() {
        let t = BoolLattice::new(true);
        let f = BoolLattice::new(false);

        assert_eq!(t.join(&f).value(), true);
        assert_eq!(f.join(&f).value(), false);

        // Commutative
        assert_eq!(t.join(&f), f.join(&t));

        // Idempotent
        assert_eq!(t.join(&t), t);

        // Bottom
        let bottom = BoolLattice::bottom().unwrap();
        assert!(!bottom.value());
    }

    #[test]
    fn test_lattice_value_max() {
        let a = LatticeValue::max(5);
        let b = LatticeValue::max(10);
        let c = a.join(&b).unwrap();

        assert_eq!(c, LatticeValue::max(10));
        assert!(a.less_than_or_equal(&c));
    }

    #[test]
    fn test_lattice_value_set() {
        let a = LatticeValue::set([Value::Integer(1), Value::Integer(2)]);
        let b = LatticeValue::set([Value::Integer(2), Value::Integer(3)]);
        let c = a.join(&b).unwrap();

        if let LatticeValue::Set(s) = c {
            assert_eq!(s.len(), 3);
        } else {
            panic!("Expected Set");
        }
    }

    #[test]
    fn test_lattice_value_counter() {
        let a = LatticeValue::counter(5);
        let b = LatticeValue::counter(10);
        let c = a.join(&b).unwrap();

        assert_eq!(c, LatticeValue::counter(15));
    }

    #[test]
    fn test_lattice_value_incompatible() {
        let a = LatticeValue::max(5);
        let b = LatticeValue::min(10);
        assert!(a.join(&b).is_none());
    }

    #[test]
    fn test_lattice_associativity() {
        let a = MaxLattice::new(3);
        let b = MaxLattice::new(7);
        let c = MaxLattice::new(5);

        // (a ⊔ b) ⊔ c = a ⊔ (b ⊔ c)
        assert_eq!(a.join(&b).join(&c), a.join(&b.join(&c)));
    }
}
