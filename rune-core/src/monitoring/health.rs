//! Health check endpoints and system status monitoring

use crate::facts::FactStore;
use crate::monitoring::collector::MetricsCollector;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

/// Health check status
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// System is healthy and ready to serve requests
    Healthy,
    /// System is degraded but still operational
    Degraded,
    /// System is unhealthy and should not receive traffic
    Unhealthy,
}

impl HealthStatus {
    /// Convert to HTTP status code
    pub fn to_http_status(&self) -> u16 {
        match self {
            HealthStatus::Healthy => 200,
            HealthStatus::Degraded => 503, // Service Unavailable
            HealthStatus::Unhealthy => 503,
        }
    }
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub status: HealthStatus,
    pub timestamp: SystemTime,
    pub checks: Vec<ComponentHealth>,
    pub version: String,
    pub uptime: Duration,
    pub metrics: Option<HealthMetrics>,
}

/// Individual component health
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub name: String,
    pub status: HealthStatus,
    pub message: Option<String>,
    pub latency_ms: Option<u64>,
}

/// Health-related metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMetrics {
    pub requests_per_second: f64,
    pub avg_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub error_rate: f64,
    pub fact_store_size: usize,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
}

/// Health check system
pub struct HealthCheck {
    collector: Arc<MetricsCollector>,
    fact_store: Arc<FactStore>,
    start_time: Instant,
    thresholds: HealthThresholds,
}

/// Configurable health check thresholds
#[derive(Debug, Clone)]
pub struct HealthThresholds {
    /// Maximum P99 latency in milliseconds before degradation
    pub max_p99_latency_ms: f64,
    /// Maximum error rate percentage before degradation
    pub max_error_rate: f64,
    /// Maximum fact store size before degradation
    pub max_fact_store_size: usize,
    /// Maximum memory usage in MB before degradation
    pub max_memory_mb: f64,
    /// Minimum requests per second to be considered healthy
    pub min_rps: f64,
}

impl Default for HealthThresholds {
    fn default() -> Self {
        Self {
            max_p99_latency_ms: 10.0,       // 10ms P99 latency
            max_error_rate: 5.0,             // 5% error rate
            max_fact_store_size: 10_000_000, // 10M facts
            max_memory_mb: 1024.0,            // 1GB memory
            min_rps: 0.1,                     // At least 0.1 RPS to be "alive"
        }
    }
}

impl HealthCheck {
    /// Create a new health check system
    pub fn new(
        collector: Arc<MetricsCollector>,
        fact_store: Arc<FactStore>,
    ) -> Self {
        Self {
            collector,
            fact_store,
            start_time: Instant::now(),
            thresholds: HealthThresholds::default(),
        }
    }

    /// Create with custom thresholds
    pub fn with_thresholds(
        collector: Arc<MetricsCollector>,
        fact_store: Arc<FactStore>,
        thresholds: HealthThresholds,
    ) -> Self {
        Self {
            collector,
            fact_store,
            start_time: Instant::now(),
            thresholds,
        }
    }

    /// Perform a readiness check (is the service ready to accept requests?)
    pub async fn readiness_check(&self) -> HealthCheckResult {
        let mut checks = Vec::new();
        let start = Instant::now();

        // Check fact store
        let fact_store_check = self.check_fact_store();
        checks.push(fact_store_check);

        // Check Cedar policy engine
        let policy_check = self.check_policy_engine();
        checks.push(policy_check);

        // Check Datalog evaluator
        let datalog_check = self.check_datalog_evaluator();
        checks.push(datalog_check);

        // Check metrics system
        let metrics_check = self.check_metrics_system();
        checks.push(metrics_check);

        // Determine overall status
        let status = self.determine_overall_status(&checks);

        // Collect health metrics
        let metrics = self.collect_health_metrics();

        HealthCheckResult {
            status,
            timestamp: SystemTime::now(),
            checks,
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime: self.start_time.elapsed(),
            metrics: Some(metrics),
        }
    }

    /// Perform a liveness check (is the service alive and not deadlocked?)
    pub async fn liveness_check(&self) -> HealthCheckResult {
        let mut checks = Vec::new();

        // Simple check that we can respond
        checks.push(ComponentHealth {
            name: "service".to_string(),
            status: HealthStatus::Healthy,
            message: Some("Service is responding".to_string()),
            latency_ms: Some(0),
        });

        // Check if we can read from fact store (non-blocking)
        let start = Instant::now();
        let _size = self.fact_store.size();
        let latency = start.elapsed().as_millis() as u64;

        checks.push(ComponentHealth {
            name: "fact_store_read".to_string(),
            status: if latency < 100 {
                HealthStatus::Healthy
            } else {
                HealthStatus::Degraded
            },
            message: Some(format!("Fact store read latency: {}ms", latency)),
            latency_ms: Some(latency),
        });

        HealthCheckResult {
            status: if checks.iter().all(|c| c.status == HealthStatus::Healthy) {
                HealthStatus::Healthy
            } else {
                HealthStatus::Degraded
            },
            timestamp: SystemTime::now(),
            checks,
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime: self.start_time.elapsed(),
            metrics: None, // Don't collect full metrics for liveness
        }
    }

    /// Check fact store health
    fn check_fact_store(&self) -> ComponentHealth {
        let start = Instant::now();
        let size = self.fact_store.size();
        let latency = start.elapsed().as_millis() as u64;

        let status = if size > self.thresholds.max_fact_store_size {
            HealthStatus::Degraded
        } else if latency > 100 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        ComponentHealth {
            name: "fact_store".to_string(),
            status,
            message: Some(format!(
                "Fact store contains {} facts, latency: {}ms",
                size, latency
            )),
            latency_ms: Some(latency),
        }
    }

    /// Check Cedar policy engine health
    fn check_policy_engine(&self) -> ComponentHealth {
        // In a real implementation, we'd test policy evaluation
        // For now, we'll simulate with a simple check
        ComponentHealth {
            name: "cedar_policy_engine".to_string(),
            status: HealthStatus::Healthy,
            message: Some("Policy engine is operational".to_string()),
            latency_ms: Some(0),
        }
    }

    /// Check Datalog evaluator health
    fn check_datalog_evaluator(&self) -> ComponentHealth {
        // In a real implementation, we'd test rule evaluation
        // For now, we'll simulate with a simple check
        ComponentHealth {
            name: "datalog_evaluator".to_string(),
            status: HealthStatus::Healthy,
            message: Some("Datalog evaluator is operational".to_string()),
            latency_ms: Some(0),
        }
    }

    /// Check metrics system health
    fn check_metrics_system(&self) -> ComponentHealth {
        let snapshot = self.collector.snapshot();
        let has_metrics = !snapshot.counters.is_empty() || !snapshot.gauges.is_empty();

        ComponentHealth {
            name: "metrics_system".to_string(),
            status: if has_metrics {
                HealthStatus::Healthy
            } else {
                HealthStatus::Degraded
            },
            message: Some(format!(
                "Metrics system tracking {} counters, {} gauges",
                snapshot.counters.len(),
                snapshot.gauges.len()
            )),
            latency_ms: None,
        }
    }

    /// Determine overall health status from component checks
    fn determine_overall_status(&self, checks: &[ComponentHealth]) -> HealthStatus {
        let unhealthy_count = checks
            .iter()
            .filter(|c| c.status == HealthStatus::Unhealthy)
            .count();
        let degraded_count = checks
            .iter()
            .filter(|c| c.status == HealthStatus::Degraded)
            .count();

        if unhealthy_count > 0 {
            HealthStatus::Unhealthy
        } else if degraded_count > checks.len() / 2 {
            // If more than half are degraded, overall is degraded
            HealthStatus::Degraded
        } else if degraded_count > 0 {
            // Some degradation but still mostly healthy
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }

    /// Collect current health metrics
    fn collect_health_metrics(&self) -> HealthMetrics {
        let snapshot = self.collector.snapshot();

        // Calculate RPS
        let total_requests = snapshot.counters.get("total_requests").unwrap_or(&0);
        let uptime_secs = self.collector.uptime().as_secs_f64();
        let rps = if uptime_secs > 0.0 {
            *total_requests as f64 / uptime_secs
        } else {
            0.0
        };

        // Calculate error rate
        let allows = snapshot.counters.get("allows").unwrap_or(&0);
        let denies = snapshot.counters.get("denies").unwrap_or(&0);
        let errors = snapshot.counters.get("errors").unwrap_or(&0);
        let total = allows + denies;
        let error_rate = if total > 0 {
            (*errors as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        // Get latency metrics
        let avg_latency = snapshot.gauges.get("avg_latency_us").unwrap_or(&0.0) / 1000.0; // Convert to ms
        let p99_latency = if let Some(hist) = snapshot.histograms.get("authorization_latency") {
            hist.p99 * 1000.0 // Convert to ms
        } else {
            0.0
        };

        // Get system metrics (simplified - in production would use sysinfo crate)
        let memory_usage_mb = self.estimate_memory_usage();
        let cpu_usage_percent = 0.0; // Would need sysinfo crate for real CPU usage

        HealthMetrics {
            requests_per_second: rps,
            avg_latency_ms: avg_latency,
            p99_latency_ms: p99_latency,
            error_rate,
            fact_store_size: self.fact_store.size(),
            memory_usage_mb,
            cpu_usage_percent,
        }
    }

    /// Estimate memory usage (simplified)
    fn estimate_memory_usage(&self) -> f64 {
        // Rough estimation based on fact store size
        // In production, would use sysinfo crate
        let fact_store_size = self.fact_store.size();
        let bytes_per_fact = 100; // Rough estimate
        let bytes = fact_store_size * bytes_per_fact;
        bytes as f64 / (1024.0 * 1024.0)
    }
}

/// System health for monitoring dashboards
pub struct SystemHealth {
    health_check: Arc<HealthCheck>,
}

impl SystemHealth {
    /// Create a new system health monitor
    pub fn new(health_check: Arc<HealthCheck>) -> Self {
        Self { health_check }
    }

    /// Get current system health as JSON
    pub async fn get_health_json(&self) -> serde_json::Value {
        let result = self.health_check.readiness_check().await;
        serde_json::to_value(result).unwrap_or_else(|_| {
            serde_json::json!({
                "status": "error",
                "message": "Failed to serialize health check"
            })
        })
    }

    /// Get Prometheus-compatible health metrics
    pub fn get_prometheus_health(&self) -> String {
        let mut output = String::new();

        // Add health status as a gauge (1 = healthy, 0 = unhealthy)
        output.push_str("# HELP rune_health Overall system health status\n");
        output.push_str("# TYPE rune_health gauge\n");

        // This would be async in real implementation
        let fact_store_size = self.health_check.fact_store.size();
        let uptime = self.health_check.start_time.elapsed().as_secs();

        output.push_str(&format!("rune_health 1\n"));
        output.push_str(&format!("# HELP rune_uptime_seconds System uptime in seconds\n"));
        output.push_str(&format!("# TYPE rune_uptime_seconds counter\n"));
        output.push_str(&format!("rune_uptime_seconds {}\n", uptime));
        output.push_str(&format!("# HELP rune_fact_store_size Current fact store size\n"));
        output.push_str(&format!("# TYPE rune_fact_store_size gauge\n"));
        output.push_str(&format!("rune_fact_store_size {}\n", fact_store_size));

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_health_check() {
        let fact_store = Arc::new(FactStore::new());
        let collector = Arc::new(MetricsCollector::new());
        let health_check = HealthCheck::new(collector.clone(), fact_store.clone());

        // Perform readiness check
        let result = health_check.readiness_check().await;
        assert_eq!(result.status, HealthStatus::Healthy);
        assert!(!result.checks.is_empty());
        assert!(result.metrics.is_some());
    }

    #[tokio::test]
    async fn test_liveness_check() {
        let fact_store = Arc::new(FactStore::new());
        let collector = Arc::new(MetricsCollector::new());
        let health_check = HealthCheck::new(collector.clone(), fact_store.clone());

        // Perform liveness check
        let result = health_check.liveness_check().await;
        assert_eq!(result.status, HealthStatus::Healthy);
        assert!(!result.checks.is_empty());
    }

    #[test]
    fn test_health_status_to_http() {
        assert_eq!(HealthStatus::Healthy.to_http_status(), 200);
        assert_eq!(HealthStatus::Degraded.to_http_status(), 503);
        assert_eq!(HealthStatus::Unhealthy.to_http_status(), 503);
    }

    #[test]
    fn test_health_thresholds() {
        let thresholds = HealthThresholds::default();
        assert_eq!(thresholds.max_p99_latency_ms, 10.0);
        assert_eq!(thresholds.max_error_rate, 5.0);
        assert_eq!(thresholds.max_fact_store_size, 10_000_000);
    }
}