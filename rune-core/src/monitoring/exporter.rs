//! Prometheus metrics exporter

use crate::monitoring::collector::{MetricsCollector, MetricsSnapshot};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use std::fmt::Write;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Prometheus registry wrapper
pub struct PrometheusRegistry {
    handle: PrometheusHandle,
    collector: Arc<MetricsCollector>,
}

impl PrometheusRegistry {
    /// Create a new registry with the given collector
    pub fn new(collector: Arc<MetricsCollector>) -> Self {
        let builder = PrometheusBuilder::new();
        let handle = builder
            .set_buckets_for_metric(
                Matcher::Full("rune_authorization_latency".to_string()),
                &[0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0],
            )
            .unwrap()
            .set_buckets_for_metric(
                Matcher::Full("rune_datalog_evaluation_latency".to_string()),
                &[0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0],
            )
            .unwrap()
            .set_buckets_for_metric(
                Matcher::Full("rune_policy_evaluation_latency".to_string()),
                &[0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1],
            )
            .unwrap()
            .install_recorder()
            .unwrap();

        Self { handle, collector }
    }

    /// Render metrics in Prometheus format
    pub fn render(&self) -> String {
        self.handle.render()
    }

    /// Get the collector
    pub fn collector(&self) -> Arc<MetricsCollector> {
        Arc::clone(&self.collector)
    }
}

/// Prometheus exporter implementation
pub struct PrometheusExporter {
    collector: Arc<MetricsCollector>,
}

impl PrometheusExporter {
    /// Create a new exporter
    pub fn new(collector: Arc<MetricsCollector>) -> Self {
        Self { collector }
    }

    /// Export metrics in Prometheus text format
    pub fn export(&self) -> String {
        let snapshot = self.collector.snapshot();
        let mut output = String::new();

        // Write header
        writeln!(
            &mut output,
            "# HELP rune_up Uptime of the RUNE engine in seconds"
        )
        .unwrap();
        writeln!(&mut output, "# TYPE rune_up gauge").unwrap();
        writeln!(
            &mut output,
            "rune_up {}",
            self.collector.uptime().as_secs()
        )
        .unwrap();

        // Write timestamp
        let timestamp = snapshot
            .timestamp
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        // Export counters
        for (name, value) in &snapshot.counters {
            let metric_name = format!("rune_{}", sanitize_metric_name(name));
            writeln!(
                &mut output,
                "# HELP {} Counter metric for {}",
                metric_name, name
            )
            .unwrap();
            writeln!(&mut output, "# TYPE {} counter", metric_name).unwrap();
            writeln!(&mut output, "{} {} {}", metric_name, value, timestamp).unwrap();
        }

        // Export gauges
        for (name, value) in &snapshot.gauges {
            let metric_name = format!("rune_{}", sanitize_metric_name(name));
            writeln!(
                &mut output,
                "# HELP {} Gauge metric for {}",
                metric_name, name
            )
            .unwrap();
            writeln!(&mut output, "# TYPE {} gauge", metric_name).unwrap();
            writeln!(&mut output, "{} {} {}", metric_name, value, timestamp).unwrap();
        }

        // Export histograms
        for (name, hist) in &snapshot.histograms {
            let metric_name = format!("rune_{}", sanitize_metric_name(name));

            writeln!(
                &mut output,
                "# HELP {} Histogram metric for {}",
                metric_name, name
            )
            .unwrap();
            writeln!(&mut output, "# TYPE {} histogram", metric_name).unwrap();

            // Buckets (simplified - would need actual bucket boundaries in production)
            writeln!(
                &mut output,
                "{}_bucket{{le=\"0.001\"}} {} {}",
                metric_name,
                hist.count as f64 * 0.1,
                timestamp
            )
            .unwrap();
            writeln!(
                &mut output,
                "{}_bucket{{le=\"0.01\"}} {} {}",
                metric_name,
                hist.count as f64 * 0.5,
                timestamp
            )
            .unwrap();
            writeln!(
                &mut output,
                "{}_bucket{{le=\"0.1\"}} {} {}",
                metric_name,
                hist.count as f64 * 0.95,
                timestamp
            )
            .unwrap();
            writeln!(
                &mut output,
                "{}_bucket{{le=\"+Inf\"}} {} {}",
                metric_name, hist.count, timestamp
            )
            .unwrap();

            // Sum and count
            writeln!(
                &mut output,
                "{}_sum {} {}",
                metric_name, hist.sum, timestamp
            )
            .unwrap();
            writeln!(
                &mut output,
                "{}_count {} {}",
                metric_name, hist.count, timestamp
            )
            .unwrap();

            // Quantiles
            writeln!(
                &mut output,
                "# HELP {}_quantile Quantiles for {}",
                metric_name, name
            )
            .unwrap();
            writeln!(&mut output, "# TYPE {}_quantile gauge", metric_name).unwrap();
            writeln!(
                &mut output,
                "{}_quantile{{quantile=\"0.5\"}} {} {}",
                metric_name, hist.p50, timestamp
            )
            .unwrap();
            writeln!(
                &mut output,
                "{}_quantile{{quantile=\"0.95\"}} {} {}",
                metric_name, hist.p95, timestamp
            )
            .unwrap();
            writeln!(
                &mut output,
                "{}_quantile{{quantile=\"0.99\"}} {} {}",
                metric_name, hist.p99, timestamp
            )
            .unwrap();
            writeln!(
                &mut output,
                "{}_quantile{{quantile=\"0.999\"}} {} {}",
                metric_name, hist.p999, timestamp
            )
            .unwrap();
        }

        // Add custom RUNE-specific metrics
        self.export_custom_metrics(&mut output, &snapshot, timestamp);

        output
    }

    /// Export custom RUNE-specific metrics
    fn export_custom_metrics(
        &self,
        output: &mut String,
        snapshot: &MetricsSnapshot,
        timestamp: u128,
    ) {
        // Authorization success rate
        if let (Some(allows), Some(denies)) =
            (snapshot.counters.get("allows"), snapshot.counters.get("denies"))
        {
            let total = allows + denies;
            if total > 0 {
                let success_rate = *allows as f64 / total as f64 * 100.0;
                writeln!(
                    output,
                    "# HELP rune_authorization_success_rate Percentage of allowed requests"
                )
                .unwrap();
                writeln!(output, "# TYPE rune_authorization_success_rate gauge").unwrap();
                writeln!(
                    output,
                    "rune_authorization_success_rate {} {}",
                    success_rate, timestamp
                )
                .unwrap();
            }
        }

        // Requests per second (calculated from total requests and uptime)
        if let Some(total_requests) = snapshot.counters.get("total_requests") {
            let uptime_secs = self.collector.uptime().as_secs_f64();
            if uptime_secs > 0.0 {
                let rps = *total_requests as f64 / uptime_secs;
                writeln!(
                    output,
                    "# HELP rune_requests_per_second Current request rate"
                )
                .unwrap();
                writeln!(output, "# TYPE rune_requests_per_second gauge").unwrap();
                writeln!(output, "rune_requests_per_second {} {}", rps, timestamp).unwrap();
            }
        }

        // System load indicator (simplified)
        if let Some(avg_latency) = snapshot.gauges.get("avg_latency_us") {
            let load = if *avg_latency < 100.0 {
                "low"
            } else if *avg_latency < 1000.0 {
                "medium"
            } else {
                "high"
            };

            writeln!(
                output,
                "# HELP rune_system_load Current system load level"
            )
            .unwrap();
            writeln!(output, "# TYPE rune_system_load gauge").unwrap();
            writeln!(
                output,
                "rune_system_load{{level=\"{}\"}} 1 {}",
                load, timestamp
            )
            .unwrap();
        }
    }

    /// Export metrics in JSON format (for non-Prometheus consumers)
    pub fn export_json(&self) -> serde_json::Value {
        let snapshot = self.collector.snapshot();

        serde_json::json!({
            "timestamp": snapshot.timestamp.duration_since(UNIX_EPOCH)
                .unwrap().as_secs(),
            "uptime_seconds": self.collector.uptime().as_secs(),
            "counters": snapshot.counters,
            "gauges": snapshot.gauges,
            "histograms": snapshot.histograms.into_iter().map(|(k, v)| {
                (k, serde_json::json!({
                    "count": v.count,
                    "sum": v.sum,
                    "min": v.min,
                    "max": v.max,
                    "p50": v.p50,
                    "p95": v.p95,
                    "p99": v.p99,
                    "p999": v.p999,
                }))
            }).collect::<serde_json::Map<_, _>>(),
        })
    }
}

/// Sanitize metric names for Prometheus
fn sanitize_metric_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_prometheus_export() {
        let collector = Arc::new(MetricsCollector::new());

        // Record some metrics
        collector.record_request(Duration::from_millis(5), true);
        collector.record_request(Duration::from_millis(10), false);
        collector.increment_counter("test_counter", 42);
        collector.update_gauge("test_gauge", 3.14);

        let exporter = PrometheusExporter::new(collector);
        let output = exporter.export();

        // Check for expected content
        assert!(output.contains("rune_up"));
        assert!(output.contains("rune_total_requests"));
        assert!(output.contains("rune_allows"));
        assert!(output.contains("rune_denies"));
        assert!(output.contains("rune_test_counter"));
        assert!(output.contains("rune_test_gauge"));
        assert!(output.contains("# TYPE"));
        assert!(output.contains("# HELP"));
    }

    #[test]
    fn test_json_export() {
        let collector = Arc::new(MetricsCollector::new());

        collector.record_request(Duration::from_millis(5), true);
        collector.increment_counter("events", 10);
        collector.update_gauge("temperature", 23.5);

        let exporter = PrometheusExporter::new(collector);
        let json = exporter.export_json();

        assert!(json.get("timestamp").is_some());
        assert!(json.get("uptime_seconds").is_some());
        assert!(json.get("counters").is_some());
        assert!(json.get("gauges").is_some());
        assert!(json.get("histograms").is_some());
    }

    #[test]
    fn test_metric_name_sanitization() {
        assert_eq!(sanitize_metric_name("valid_name"), "valid_name");
        assert_eq!(sanitize_metric_name("invalid-name"), "invalid_name");
        assert_eq!(sanitize_metric_name("name.with.dots"), "name_with_dots");
        assert_eq!(sanitize_metric_name("name@with#symbols"), "name_with_symbols");
    }
}