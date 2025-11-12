//! Prometheus metrics collection for RUNE server

use metrics::{counter, describe_counter, describe_gauge, describe_histogram, gauge, histogram};
use std::time::Instant;

/// Initialize all metric descriptions
pub fn init_metrics() {
    // Counters
    describe_counter!(
        "rune_authorization_requests_total",
        "Total number of authorization requests"
    );
    describe_counter!("rune_cache_hits_total", "Total number of cache hits");
    describe_counter!("rune_cache_misses_total", "Total number of cache misses");
    describe_counter!(
        "rune_rule_evaluations_total",
        "Total number of rule evaluations"
    );
    describe_counter!(
        "rune_policy_evaluations_total",
        "Total number of policy evaluations"
    );
    describe_counter!(
        "rune_reload_events_total",
        "Total number of configuration reload events"
    );
    describe_counter!("rune_errors_total", "Total number of errors");

    // Histograms
    describe_histogram!(
        "rune_authorization_latency_seconds",
        "Authorization request latency in seconds"
    );
    describe_histogram!(
        "rune_datalog_evaluation_latency_seconds",
        "Datalog evaluation latency in seconds"
    );
    describe_histogram!(
        "rune_cedar_evaluation_latency_seconds",
        "Cedar evaluation latency in seconds"
    );
    describe_histogram!(
        "rune_cache_lookup_latency_seconds",
        "Cache lookup latency in seconds"
    );
    describe_histogram!("rune_batch_size", "Batch authorization request size");

    // Gauges
    describe_gauge!("rune_loaded_rules_count", "Number of loaded Datalog rules");
    describe_gauge!(
        "rune_loaded_policies_count",
        "Number of loaded Cedar policies"
    );
    describe_gauge!("rune_cache_size_bytes", "Cache size in bytes");
    describe_gauge!(
        "rune_fact_store_entries",
        "Number of entries in the fact store"
    );
    describe_gauge!(
        "rune_active_connections",
        "Number of active HTTP connections"
    );
}

/// Record an authorization request
pub fn record_authorization(decision: &str, latency_seconds: f64, cached: bool) {
    counter!("rune_authorization_requests_total", 1, "decision" => decision.to_string());
    histogram!("rune_authorization_latency_seconds", latency_seconds);

    if cached {
        counter!("rune_cache_hits_total", 1);
    } else {
        counter!("rune_cache_misses_total", 1);
    }
}

/// Record a batch authorization request
pub fn record_batch_authorization(count: usize, latency_seconds: f64) {
    histogram!("rune_batch_size", count as f64);
    histogram!("rune_authorization_latency_seconds", latency_seconds, "type" => "batch");
}

/// Record rule evaluations
pub fn record_rule_evaluations(count: usize) {
    counter!("rune_rule_evaluations_total", count as u64);
}

/// Record policy evaluations
pub fn record_policy_evaluations(count: usize) {
    counter!("rune_policy_evaluations_total", count as u64);
}

/// Record an error
pub fn record_error(error_type: &str) {
    counter!("rune_errors_total", 1, "type" => error_type.to_string());
}

/// Update gauge metrics
pub fn update_engine_metrics(rules: usize, policies: usize, facts: usize, cache_size: usize) {
    gauge!("rune_loaded_rules_count", rules as f64);
    gauge!("rune_loaded_policies_count", policies as f64);
    gauge!("rune_fact_store_entries", facts as f64);
    gauge!("rune_cache_size_bytes", cache_size as f64);
}

/// Update connection count
pub fn update_connections(count: usize) {
    gauge!("rune_active_connections", count as f64);
}

/// Timer for measuring operation latency
pub struct LatencyTimer {
    start: Instant,
    metric_name: &'static str,
}

impl LatencyTimer {
    pub fn new(metric_name: &'static str) -> Self {
        Self {
            start: Instant::now(),
            metric_name,
        }
    }

    pub fn record(self) {
        let elapsed = self.start.elapsed().as_secs_f64();
        histogram!(self.metric_name, elapsed);
    }
}

/// Storage for Prometheus handle
static PROMETHEUS_HANDLE: std::sync::OnceLock<metrics_exporter_prometheus::PrometheusHandle> =
    std::sync::OnceLock::new();

/// Initialize Prometheus exporter and return the handle
pub fn init_prometheus() -> anyhow::Result<()> {
    let builder = metrics_exporter_prometheus::PrometheusBuilder::new();
    let handle = builder.install_recorder()?;
    PROMETHEUS_HANDLE
        .set(handle)
        .map_err(|_| anyhow::anyhow!("Failed to set Prometheus handle"))?;
    Ok(())
}

/// Get Prometheus metrics string
pub fn get_prometheus_metrics() -> String {
    PROMETHEUS_HANDLE
        .get()
        .map(|handle| handle.render())
        .unwrap_or_else(|| "# Prometheus metrics not initialized\n".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn setup() {
        INIT.call_once(|| {
            // Initialize Prometheus once for all tests
            let _ = init_prometheus();
            init_metrics();
        });
    }

    #[test]
    fn test_init_prometheus() {
        // Test that init_prometheus can be called (already called in setup)
        setup();
        // Second call should fail since handle is already set
        let result = init_prometheus();
        assert!(result.is_err());
    }

    #[test]
    fn test_get_prometheus_metrics() {
        setup();
        let metrics = get_prometheus_metrics();
        // In test environment, render() may return empty string
        // Just verify it doesn't panic
        let _ = metrics;
    }

    #[test]
    fn test_record_authorization_permitted_cached() {
        setup();
        record_authorization("permit", 0.001, true);
        // Verify metrics were recorded (no panic)
    }

    #[test]
    fn test_record_authorization_denied_not_cached() {
        setup();
        record_authorization("deny", 0.002, false);
        // Verify metrics were recorded (no panic)
    }

    #[test]
    fn test_record_batch_authorization() {
        setup();
        record_batch_authorization(10, 0.005);
        record_batch_authorization(50, 0.015);
        record_batch_authorization(100, 0.025);
    }

    #[test]
    fn test_record_rule_evaluations() {
        setup();
        record_rule_evaluations(0);
        record_rule_evaluations(1);
        record_rule_evaluations(10);
        record_rule_evaluations(100);
    }

    #[test]
    fn test_record_policy_evaluations() {
        setup();
        record_policy_evaluations(0);
        record_policy_evaluations(5);
        record_policy_evaluations(25);
    }

    #[test]
    fn test_record_error() {
        setup();
        record_error("validation");
        record_error("timeout");
        record_error("internal");
        record_error("unauthorized");
    }

    #[test]
    fn test_update_engine_metrics() {
        setup();
        update_engine_metrics(0, 0, 0, 0);
        update_engine_metrics(10, 5, 100, 1024);
        update_engine_metrics(50, 25, 1000, 10240);
    }

    #[test]
    fn test_update_connections() {
        setup();
        update_connections(0);
        update_connections(1);
        update_connections(10);
        update_connections(100);
    }

    #[test]
    fn test_latency_timer() {
        setup();
        let timer = LatencyTimer::new("rune_test_latency");
        // Simulate some work
        std::thread::sleep(std::time::Duration::from_millis(1));
        timer.record();
    }

    #[test]
    fn test_latency_timer_immediate() {
        setup();
        let timer = LatencyTimer::new("rune_authorization_latency_seconds");
        timer.record(); // Should record immediately
    }

    #[test]
    fn test_latency_timer_different_metrics() {
        setup();
        let timer1 = LatencyTimer::new("rune_datalog_evaluation_latency_seconds");
        timer1.record();

        let timer2 = LatencyTimer::new("rune_cedar_evaluation_latency_seconds");
        timer2.record();

        let timer3 = LatencyTimer::new("rune_cache_lookup_latency_seconds");
        timer3.record();
    }

    #[test]
    fn test_multiple_authorization_decisions() {
        setup();
        // Test various decision types
        for decision in &["permit", "deny", "indeterminate"] {
            for cached in &[true, false] {
                record_authorization(decision, 0.001, *cached);
            }
        }
    }

    #[test]
    fn test_batch_authorization_various_sizes() {
        setup();
        // Test edge cases for batch sizes
        let batch_sizes = vec![0, 1, 2, 10, 50, 100, 1000];
        for size in batch_sizes {
            record_batch_authorization(size, size as f64 * 0.0001);
        }
    }

    #[test]
    fn test_error_types() {
        setup();
        // Test various error types
        let error_types = vec![
            "parse_error",
            "validation_error",
            "timeout_error",
            "network_error",
            "database_error",
            "authorization_error",
            "configuration_error",
        ];
        for error_type in error_types {
            record_error(error_type);
        }
    }

    #[test]
    fn test_engine_metrics_edge_cases() {
        setup();
        // Test with maximum values
        update_engine_metrics(usize::MAX, usize::MAX, usize::MAX, usize::MAX);
        // Test with zero values
        update_engine_metrics(0, 0, 0, 0);
    }

    #[test]
    fn test_connection_count_changes() {
        setup();
        // Simulate connection count changes
        update_connections(0);
        for i in 1..=10 {
            update_connections(i);
        }
        for i in (0..=10).rev() {
            update_connections(i);
        }
    }

    #[test]
    fn test_concurrent_metric_updates() {
        setup();
        use std::thread;

        let handles: Vec<_> = (0..10)
            .map(|i| {
                thread::spawn(move || {
                    record_authorization(
                        if i % 2 == 0 { "permit" } else { "deny" },
                        0.001,
                        i % 3 == 0,
                    );
                    record_rule_evaluations(i);
                    record_policy_evaluations(i * 2);
                    update_connections(i);
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
    }
}
