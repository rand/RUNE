//! Integration test modules for RUNE
//!
//! Comprehensive end-to-end testing covering:
//! - Datalog-Cedar bidirectional integration
//! - Real-world authorization scenarios
//! - Performance under heavy load
//! - Error recovery and edge cases

// Test modules
pub mod datalog_cedar_integration;
pub mod error_scenarios;
pub mod performance_stress;
pub mod real_world_scenarios;

// Re-export common test utilities
use rune_core::facts::{Fact, FactStore};
use rune_core::types::Value;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Helper function to create test fact store with sample data
pub fn create_test_fact_store() -> Arc<FactStore> {
    let store = Arc::new(FactStore::new());

    // Add sample facts for testing
    store.add_fact(Fact::new(
        "user",
        vec![Value::string("test_user")],
    ));
    store.add_fact(Fact::new(
        "role",
        vec![Value::string("test_user"), Value::string("admin")],
    ));

    store
}

/// Helper to measure operation performance
pub fn measure_performance<F, R>(operation: F) -> (R, Duration)
where
    F: FnOnce() -> R,
{
    let start = Instant::now();
    let result = operation();
    let duration = start.elapsed();
    (result, duration)
}

/// Assert that an operation completes within a time limit
#[macro_export]
macro_rules! assert_completes_within {
    ($duration:expr, $operation:expr) => {{
        let start = std::time::Instant::now();
        let result = $operation;
        let elapsed = start.elapsed();
        assert!(
            elapsed <= $duration,
            "Operation took {:?}, expected <= {:?}",
            elapsed,
            $duration
        );
        result
    }};
}

/// Assert performance metrics
#[macro_export]
macro_rules! assert_performance {
    (throughput: $actual:expr, min: $min:expr) => {
        assert!(
            $actual >= $min,
            "Throughput {} ops/sec below minimum {} ops/sec",
            $actual,
            $min
        );
    };
    (latency_p99: $actual:expr, max: $max:expr) => {
        assert!(
            $actual <= $max,
            "P99 latency {:?} exceeds maximum {:?}",
            $actual,
            $max
        );
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_helper_functions() {
        // Test fact store creation
        let store = create_test_fact_store();
        assert!(store.size() > 0, "Test store should have facts");

        // Test performance measurement
        let (result, duration) = measure_performance(|| {
            std::thread::sleep(Duration::from_millis(10));
            42
        });
        assert_eq!(result, 42);
        assert!(duration >= Duration::from_millis(10));
    }

    #[test]
    fn test_performance_macros() {
        // Test completion assertion
        let result = assert_completes_within!(Duration::from_secs(1), {
            std::thread::sleep(Duration::from_millis(10));
            "completed"
        });
        assert_eq!(result, "completed");

        // Test performance assertions
        assert_performance!(throughput: 150_000.0, min: 100_000.0);
        assert_performance!(
            latency_p99: Duration::from_micros(500),
            max: Duration::from_millis(1)
        );
    }
}