//! Custom metrics recorder implementation

use crate::monitoring::collector::MetricsCollector;
use metrics::{Counter, Gauge, Histogram, Key, KeyName, Recorder, SharedString, Unit};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Type of metric being recorded
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
}

/// Metadata for a metric
#[derive(Debug, Clone)]
pub struct MetricMetadata {
    pub name: String,
    pub metric_type: MetricType,
    pub unit: Option<Unit>,
    pub description: Option<String>,
}

/// RUNE-specific metrics recorder
pub struct MetricsRecorder {
    collector: Arc<MetricsCollector>,
    metadata: Arc<RwLock<HashMap<String, MetricMetadata>>>,
}

impl MetricsRecorder {
    /// Create a new metrics recorder
    pub fn new(collector: Arc<MetricsCollector>) -> Self {
        Self {
            collector,
            metadata: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Install this recorder as the global metrics recorder
    pub fn install(self) -> Result<(), metrics::SetRecorderError> {
        metrics::set_recorder(self)
    }

    /// Get metadata for all registered metrics
    pub fn get_metadata(&self) -> HashMap<String, MetricMetadata> {
        self.metadata.read().unwrap().clone()
    }

    /// Register a metric with metadata
    fn register_metric(&self, name: &str, metric_type: MetricType, unit: Option<Unit>, description: Option<String>) {
        let mut metadata = self.metadata.write().unwrap();
        metadata.insert(
            name.to_string(),
            MetricMetadata {
                name: name.to_string(),
                metric_type,
                unit,
                description,
            },
        );
    }
}

impl Recorder for MetricsRecorder {
    fn describe_counter(&self, key: KeyName, unit: Option<Unit>, description: SharedString) {
        self.register_metric(
            key.as_str(),
            MetricType::Counter,
            unit,
            Some(description.to_string()),
        );
    }

    fn describe_gauge(&self, key: KeyName, unit: Option<Unit>, description: SharedString) {
        self.register_metric(
            key.as_str(),
            MetricType::Gauge,
            unit,
            Some(description.to_string()),
        );
    }

    fn describe_histogram(&self, key: KeyName, unit: Option<Unit>, description: SharedString) {
        self.register_metric(
            key.as_str(),
            MetricType::Histogram,
            unit,
            Some(description.to_string()),
        );
    }

    fn register_counter(&self, _key: &Key) -> Counter {
        // We don't need to pre-register counters
        Counter::noop()
    }

    fn register_gauge(&self, _key: &Key) -> Gauge {
        // We don't need to pre-register gauges
        Gauge::noop()
    }

    fn register_histogram(&self, _key: &Key) -> Histogram {
        // We don't need to pre-register histograms
        Histogram::noop()
    }

    fn increment_counter(&self, key: &Key, value: u64) {
        let name = key.name().to_string();
        self.collector.increment_counter(&name, value);
    }

    fn update_gauge(&self, key: &Key, value: metrics::GaugeValue) {
        let name = key.name().to_string();
        let float_value = match value {
            metrics::GaugeValue::Absolute(v) => v,
            metrics::GaugeValue::Increment(v) => {
                // For increment, we need to get the current value
                // This is handled in the collector
                v
            }
            metrics::GaugeValue::Decrement(v) => {
                // For decrement, we need to get the current value
                // This is handled in the collector
                -v
            }
        };
        self.collector.update_gauge(&name, float_value);
    }

    fn record_histogram(&self, key: &Key, value: f64) {
        let name = key.name().to_string();
        self.collector.record_histogram(&name, value);
    }
}

/// Builder for creating and configuring the metrics system
pub struct MetricsBuilder {
    enable_prometheus: bool,
    enable_json: bool,
    prometheus_port: u16,
    json_port: u16,
}

impl Default for MetricsBuilder {
    fn default() -> Self {
        Self {
            enable_prometheus: true,
            enable_json: false,
            prometheus_port: 9090,
            json_port: 9091,
        }
    }
}

impl MetricsBuilder {
    /// Create a new metrics builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable Prometheus export
    pub fn with_prometheus(mut self, port: u16) -> Self {
        self.enable_prometheus = true;
        self.prometheus_port = port;
        self
    }

    /// Enable JSON export
    pub fn with_json(mut self, port: u16) -> Self {
        self.enable_json = true;
        self.json_port = port;
        self
    }

    /// Build and install the metrics system
    pub fn build(self) -> Result<Arc<MetricsCollector>, Box<dyn std::error::Error>> {
        // Create the collector
        let collector = Arc::new(MetricsCollector::new());

        // Create and install the recorder
        let recorder = MetricsRecorder::new(collector.clone());
        recorder.install()?;

        // Register all RUNE metrics
        register_rune_metrics();

        Ok(collector)
    }
}

/// Register all RUNE-specific metrics
fn register_rune_metrics() {
    use metrics::{describe_counter, describe_gauge, describe_histogram};

    // Performance metrics
    describe_histogram!(
        "rune_authorization_latency",
        Unit::Seconds,
        "Time taken to process authorization requests"
    );

    describe_histogram!(
        "rune_datalog_evaluation_latency",
        Unit::Seconds,
        "Time taken to evaluate Datalog rules"
    );

    describe_histogram!(
        "rune_policy_evaluation_latency",
        Unit::Seconds,
        "Time taken to evaluate Cedar policies"
    );

    describe_counter!(
        "rune_authorization_requests_total",
        Unit::Count,
        "Total number of authorization requests processed"
    );

    describe_gauge!(
        "rune_active_evaluations",
        Unit::Count,
        "Number of currently active evaluations"
    );

    // Business metrics
    describe_counter!(
        "rune_authorization_decisions",
        Unit::Count,
        "Number of authorization decisions made, labeled by result"
    );

    describe_counter!(
        "rune_policy_evaluations",
        Unit::Count,
        "Number of policy evaluations, labeled by policy ID"
    );

    describe_counter!(
        "rune_datalog_facts_derived",
        Unit::Count,
        "Number of facts derived from Datalog rules"
    );

    describe_histogram!(
        "rune_request_context_size",
        Unit::Bytes,
        "Size of authorization request context"
    );

    // System health metrics
    describe_gauge!(
        "rune_fact_store_size",
        Unit::Count,
        "Current number of facts in the fact store"
    );

    describe_gauge!(
        "rune_policy_count",
        Unit::Count,
        "Current number of loaded policies"
    );

    describe_gauge!(
        "rune_cache_size",
        Unit::Bytes,
        "Size of various internal caches"
    );

    describe_gauge!(
        "rune_cache_hit_rate",
        Unit::Percent,
        "Cache hit rate percentage"
    );

    describe_gauge!(
        "rune_memory_usage",
        Unit::Bytes,
        "Current memory usage of the RUNE engine"
    );

    // Error metrics
    describe_counter!(
        "rune_errors_total",
        Unit::Count,
        "Total number of errors, labeled by error type"
    );

    describe_counter!(
        "rune_policy_conflicts",
        Unit::Count,
        "Number of policy conflicts detected"
    );

    describe_counter!(
        "rune_datalog_cycles_detected",
        Unit::Count,
        "Number of cycles detected in Datalog evaluation"
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
        "Time taken to perform hot-reload"
    );
}

/// High-level metrics helpers
pub mod helpers {
    use std::time::Duration;

    /// Record an authorization request with all relevant metrics
    pub fn record_authorization(duration: Duration, allowed: bool, context_size: usize) {
        metrics::histogram!("rune_authorization_latency", duration.as_secs_f64());
        metrics::counter!("rune_authorization_requests_total", 1);

        let result = if allowed { "allow" } else { "deny" };
        metrics::counter!("rune_authorization_decisions", 1, "result" => result);

        metrics::histogram!("rune_request_context_size", context_size as f64);
    }

    /// Record a Datalog evaluation
    pub fn record_datalog_evaluation(duration: Duration, facts_derived: usize) {
        metrics::histogram!("rune_datalog_evaluation_latency", duration.as_secs_f64());
        metrics::counter!("rune_datalog_facts_derived", facts_derived as u64);
    }

    /// Record a Cedar policy evaluation
    pub fn record_policy_evaluation(policy_id: &str, duration: Duration) {
        metrics::histogram!("rune_policy_evaluation_latency", duration.as_secs_f64());
        metrics::counter!("rune_policy_evaluations", 1, "policy_id" => policy_id.to_string());
    }

    /// Record a cache access
    pub fn record_cache_access(cache_name: &str, hit: bool) {
        let result = if hit { "hit" } else { "miss" };
        metrics::counter!("rune_cache_accesses", 1, "cache" => cache_name.to_string(), "result" => result);
    }

    /// Record an error
    pub fn record_error(error_type: &str) {
        metrics::counter!("rune_errors_total", 1, "type" => error_type.to_string());
    }

    /// Record a hot-reload event
    pub fn record_hot_reload(duration: Duration, success: bool) {
        metrics::counter!("rune_hot_reloads", 1, "success" => success.to_string());
        metrics::histogram!("rune_hot_reload_duration", duration.as_secs_f64());
    }

    /// Update system metrics
    pub fn update_system_metrics(fact_store_size: usize, policy_count: usize, memory_mb: f64) {
        metrics::gauge!("rune_fact_store_size", fact_store_size as f64);
        metrics::gauge!("rune_policy_count", policy_count as f64);
        metrics::gauge!("rune_memory_usage", memory_mb * 1024.0 * 1024.0); // Convert MB to bytes
    }

    /// Update cache metrics
    pub fn update_cache_metrics(cache_name: &str, size: usize, hit_rate: f64) {
        metrics::gauge!("rune_cache_size", size as f64, "cache" => cache_name.to_string());
        metrics::gauge!("rune_cache_hit_rate", hit_rate, "cache" => cache_name.to_string());
    }

    /// Start tracking an active evaluation
    pub fn start_evaluation() {
        metrics::increment_gauge!("rune_active_evaluations", 1.0);
    }

    /// Stop tracking an active evaluation
    pub fn end_evaluation() {
        metrics::decrement_gauge!("rune_active_evaluations", 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_metadata() {
        let metadata = MetricMetadata {
            name: "test_metric".to_string(),
            metric_type: MetricType::Counter,
            unit: Some(Unit::Count),
            description: Some("Test metric".to_string()),
        };

        assert_eq!(metadata.name, "test_metric");
        assert_eq!(metadata.metric_type, MetricType::Counter);
        assert_eq!(metadata.unit, Some(Unit::Count));
    }

    #[test]
    fn test_metrics_builder() {
        let builder = MetricsBuilder::new()
            .with_prometheus(9090)
            .with_json(9091);

        assert!(builder.enable_prometheus);
        assert!(builder.enable_json);
        assert_eq!(builder.prometheus_port, 9090);
        assert_eq!(builder.json_port, 9091);
    }

    #[test]
    fn test_helpers() {
        use helpers::*;
        use std::time::Duration;

        // Test that helpers don't panic
        record_authorization(Duration::from_millis(5), true, 1024);
        record_datalog_evaluation(Duration::from_millis(10), 100);
        record_policy_evaluation("policy_123", Duration::from_millis(2));
        record_cache_access("authorization", true);
        record_error("parse_error");
        record_hot_reload(Duration::from_millis(50), true);
        update_system_metrics(10000, 50, 256.0);
        update_cache_metrics("authorization", 500, 0.85);
        start_evaluation();
        end_evaluation();
    }
}