//! Monitoring and metrics infrastructure for RUNE
//!
//! Provides comprehensive observability through:
//! - Performance metrics (latency, throughput)
//! - Business metrics (authorization decisions)
//! - System health indicators
//! - Prometheus export
//! - OpenTelemetry support

pub mod collector;
pub mod exporter;
pub mod health;
pub mod metrics;
pub mod tracing_setup;

use metrics::{describe_counter, describe_gauge, describe_histogram, Unit};
use std::sync::Arc;
use std::time::Duration;

pub use collector::{MetricsCollector, MetricsSnapshot};
pub use exporter::{PrometheusExporter, PrometheusRegistry};
pub use health::{HealthCheck, HealthStatus, SystemHealth};
pub use metrics::{MetricType, MetricsRecorder};

/// Global metrics instance
static mut METRICS: Option<Arc<MetricsCollector>> = None;
static INIT: std::sync::Once = std::sync::Once::new();

/// Initialize the monitoring system
pub fn init() -> Arc<MetricsCollector> {
    unsafe {
        INIT.call_once(|| {
            // Register metric descriptions
            register_metrics();

            // Create collector
            let collector = Arc::new(MetricsCollector::new());
            METRICS = Some(collector.clone());

            // Initialize tracing
            tracing_setup::init_tracing();
        });

        METRICS.as_ref().unwrap().clone()
    }
}

/// Get the global metrics collector
pub fn metrics() -> Arc<MetricsCollector> {
    unsafe {
        METRICS
            .as_ref()
            .expect("Metrics not initialized. Call monitoring::init() first")
            .clone()
    }
}

/// Register all metric descriptions for Prometheus
fn register_metrics() {
    // Performance metrics
    describe_histogram!(
        "rune_authorization_latency",
        Unit::Seconds,
        "Authorization request latency"
    );
    describe_histogram!(
        "rune_datalog_evaluation_latency",
        Unit::Seconds,
        "Datalog evaluation latency"
    );
    describe_histogram!(
        "rune_policy_evaluation_latency",
        Unit::Seconds,
        "Cedar policy evaluation latency"
    );
    describe_counter!(
        "rune_authorization_requests_total",
        Unit::Count,
        "Total authorization requests"
    );
    describe_gauge!(
        "rune_active_evaluations",
        Unit::Count,
        "Currently active evaluations"
    );

    // Business metrics
    describe_counter!(
        "rune_authorization_decisions",
        Unit::Count,
        "Authorization decisions by result (allow/deny)"
    );
    describe_counter!(
        "rune_policy_evaluations",
        Unit::Count,
        "Policy evaluations by policy ID"
    );
    describe_counter!(
        "rune_datalog_facts_derived",
        Unit::Count,
        "Number of facts derived from Datalog rules"
    );
    describe_histogram!(
        "rune_request_context_size",
        Unit::Bytes,
        "Size of request context data"
    );

    // System health metrics
    describe_gauge!(
        "rune_fact_store_size",
        Unit::Count,
        "Number of facts in the fact store"
    );
    describe_gauge!(
        "rune_policy_count",
        Unit::Count,
        "Number of loaded policies"
    );
    describe_gauge!(
        "rune_cache_size",
        Unit::Bytes,
        "Size of various caches"
    );
    describe_gauge!(
        "rune_cache_hit_rate",
        Unit::Percent,
        "Cache hit rate percentage"
    );
    describe_gauge!(
        "rune_memory_usage",
        Unit::Bytes,
        "Memory usage of the RUNE engine"
    );

    // Error metrics
    describe_counter!(
        "rune_errors_total",
        Unit::Count,
        "Total errors by error type"
    );
    describe_counter!(
        "rune_policy_conflicts",
        Unit::Count,
        "Policy conflicts detected"
    );
    describe_counter!(
        "rune_datalog_cycles_detected",
        Unit::Count,
        "Cycles detected in Datalog evaluation"
    );

    // Hot-reload metrics
    describe_counter!(
        "rune_hot_reloads",
        Unit::Count,
        "Number of hot-reload events"
    );
    describe_histogram!(
        "rune_hot_reload_duration",
        Unit::Seconds,
        "Duration of hot-reload operations"
    );
}

/// Record authorization latency
pub fn record_authorization_latency(duration: Duration) {
    metrics::histogram!("rune_authorization_latency", duration.as_secs_f64());
}

/// Record authorization decision
pub fn record_authorization_decision(allowed: bool) {
    let label = if allowed { "allow" } else { "deny" };
    metrics::counter!("rune_authorization_decisions", 1, "result" => label);
}

/// Record Datalog evaluation
pub fn record_datalog_evaluation(duration: Duration, facts_derived: usize) {
    metrics::histogram!("rune_datalog_evaluation_latency", duration.as_secs_f64());
    metrics::counter!("rune_datalog_facts_derived", facts_derived as u64);
}

/// Record policy evaluation
pub fn record_policy_evaluation(policy_id: &str, duration: Duration) {
    metrics::histogram!("rune_policy_evaluation_latency", duration.as_secs_f64());
    metrics::counter!("rune_policy_evaluations", 1, "policy_id" => policy_id.to_string());
}

/// Update fact store size
pub fn update_fact_store_size(size: usize) {
    metrics::gauge!("rune_fact_store_size", size as f64);
}

/// Update cache metrics
pub fn update_cache_metrics(cache_name: &str, size: usize, hit_rate: f64) {
    metrics::gauge!("rune_cache_size", size as f64, "cache" => cache_name.to_string());
    metrics::gauge!("rune_cache_hit_rate", hit_rate, "cache" => cache_name.to_string());
}

/// Record an error
pub fn record_error(error_type: &str) {
    metrics::counter!("rune_errors_total", 1, "type" => error_type.to_string());
}

/// Record hot-reload event
pub fn record_hot_reload(duration: Duration) {
    metrics::counter!("rune_hot_reloads", 1);
    metrics::histogram!("rune_hot_reload_duration", duration.as_secs_f64());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_initialization() {
        let collector = init();
        assert!(Arc::strong_count(&collector) > 0);

        // Should return same instance
        let collector2 = metrics();
        assert!(Arc::ptr_eq(&collector, &collector2));
    }

    #[test]
    fn test_recording_metrics() {
        init();

        // Record various metrics
        record_authorization_latency(Duration::from_millis(5));
        record_authorization_decision(true);
        record_authorization_decision(false);
        record_datalog_evaluation(Duration::from_millis(10), 100);
        record_policy_evaluation("policy_123", Duration::from_millis(2));
        update_fact_store_size(10000);
        update_cache_metrics("authorization", 500, 0.85);
        record_error("parse_error");
        record_hot_reload(Duration::from_millis(50));

        // Metrics should be recorded without panic
    }
}