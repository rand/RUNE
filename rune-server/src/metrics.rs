//! Prometheus metrics collection for RUNE server

use metrics::{counter, gauge, histogram, describe_counter, describe_histogram, describe_gauge};
use std::time::Instant;

/// Initialize all metric descriptions
pub fn init_metrics() {
    // Counters
    describe_counter!("rune_authorization_requests_total", "Total number of authorization requests");
    describe_counter!("rune_cache_hits_total", "Total number of cache hits");
    describe_counter!("rune_cache_misses_total", "Total number of cache misses");
    describe_counter!("rune_rule_evaluations_total", "Total number of rule evaluations");
    describe_counter!("rune_policy_evaluations_total", "Total number of policy evaluations");
    describe_counter!("rune_reload_events_total", "Total number of configuration reload events");
    describe_counter!("rune_errors_total", "Total number of errors");

    // Histograms
    describe_histogram!("rune_authorization_latency_seconds", "Authorization request latency in seconds");
    describe_histogram!("rune_datalog_evaluation_latency_seconds", "Datalog evaluation latency in seconds");
    describe_histogram!("rune_cedar_evaluation_latency_seconds", "Cedar evaluation latency in seconds");
    describe_histogram!("rune_cache_lookup_latency_seconds", "Cache lookup latency in seconds");
    describe_histogram!("rune_batch_size", "Batch authorization request size");

    // Gauges
    describe_gauge!("rune_loaded_rules_count", "Number of loaded Datalog rules");
    describe_gauge!("rune_loaded_policies_count", "Number of loaded Cedar policies");
    describe_gauge!("rune_cache_size_bytes", "Cache size in bytes");
    describe_gauge!("rune_fact_store_entries", "Number of entries in the fact store");
    describe_gauge!("rune_active_connections", "Number of active HTTP connections");
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
static PROMETHEUS_HANDLE: std::sync::OnceLock<metrics_exporter_prometheus::PrometheusHandle> = std::sync::OnceLock::new();

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