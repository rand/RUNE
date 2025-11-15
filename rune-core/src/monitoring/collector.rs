//! Metrics collection and aggregation

use dashmap::DashMap;
use metrics::{Key, KeyName, Recorder, SharedString, Unit};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Type of metric being recorded
#[derive(Debug, Clone, PartialEq)]
pub enum MetricValue {
    Counter(u64),
    Gauge(f64),
    Histogram(Vec<f64>),
}

/// A snapshot of metrics at a point in time
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub timestamp: SystemTime,
    pub counters: HashMap<String, u64>,
    pub gauges: HashMap<String, f64>,
    pub histograms: HashMap<String, HistogramSnapshot>,
}

/// Histogram statistics snapshot
#[derive(Debug, Clone)]
pub struct HistogramSnapshot {
    pub count: usize,
    pub sum: f64,
    pub min: f64,
    pub max: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub p999: f64,
}

/// Metrics collector implementation
pub struct MetricsCollector {
    counters: Arc<DashMap<String, AtomicU64>>,
    gauges: Arc<DashMap<String, Arc<RwLock<f64>>>>,
    histograms: Arc<DashMap<String, Arc<RwLock<Vec<f64>>>>>,
    start_time: Instant,

    // Performance tracking
    total_requests: AtomicUsize,
    total_latency_us: AtomicU64,

    // Business metrics
    allows: AtomicUsize,
    denies: AtomicUsize,

    // Cache metrics
    cache_hits: AtomicUsize,
    cache_misses: AtomicUsize,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            counters: Arc::new(DashMap::new()),
            gauges: Arc::new(DashMap::new()),
            histograms: Arc::new(DashMap::new()),
            start_time: Instant::now(),
            total_requests: AtomicUsize::new(0),
            total_latency_us: AtomicU64::new(0),
            allows: AtomicUsize::new(0),
            denies: AtomicUsize::new(0),
            cache_hits: AtomicUsize::new(0),
            cache_misses: AtomicUsize::new(0),
        }
    }

    /// Record an authorization request
    pub fn record_request(&self, latency: Duration, allowed: bool) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.total_latency_us
            .fetch_add(latency.as_micros() as u64, Ordering::Relaxed);

        if allowed {
            self.allows.fetch_add(1, Ordering::Relaxed);
        } else {
            self.denies.fetch_add(1, Ordering::Relaxed);
        }

        // Update histogram
        let key = "authorization_latency".to_string();
        let histogram = self
            .histograms
            .entry(key)
            .or_insert_with(|| Arc::new(RwLock::new(Vec::new())));
        histogram.write().push(latency.as_secs_f64());
    }

    /// Record a cache access
    pub fn record_cache_access(&self, hit: bool) {
        if hit {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.cache_misses.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Get current metrics snapshot
    pub fn snapshot(&self) -> MetricsSnapshot {
        let mut counters = HashMap::new();
        let mut gauges = HashMap::new();
        let mut histograms = HashMap::new();

        // Collect counters
        for entry in self.counters.iter() {
            let (key, value) = entry.pair();
            counters.insert(key.clone(), value.load(Ordering::Relaxed));
        }

        // Add built-in counters
        counters.insert(
            "total_requests".to_string(),
            self.total_requests.load(Ordering::Relaxed) as u64,
        );
        counters.insert(
            "allows".to_string(),
            self.allows.load(Ordering::Relaxed) as u64,
        );
        counters.insert(
            "denies".to_string(),
            self.denies.load(Ordering::Relaxed) as u64,
        );
        counters.insert(
            "cache_hits".to_string(),
            self.cache_hits.load(Ordering::Relaxed) as u64,
        );
        counters.insert(
            "cache_misses".to_string(),
            self.cache_misses.load(Ordering::Relaxed) as u64,
        );

        // Collect gauges
        for entry in self.gauges.iter() {
            let (key, value) = entry.pair();
            gauges.insert(key.clone(), *value.read());
        }

        // Calculate derived gauges
        let total_cache = self.cache_hits.load(Ordering::Relaxed)
            + self.cache_misses.load(Ordering::Relaxed);
        if total_cache > 0 {
            let hit_rate = self.cache_hits.load(Ordering::Relaxed) as f64
                / total_cache as f64
                * 100.0;
            gauges.insert("cache_hit_rate".to_string(), hit_rate);
        }

        // Calculate average latency
        let total_reqs = self.total_requests.load(Ordering::Relaxed);
        if total_reqs > 0 {
            let avg_latency = self.total_latency_us.load(Ordering::Relaxed) as f64
                / total_reqs as f64;
            gauges.insert("avg_latency_us".to_string(), avg_latency);
        }

        // Collect histograms
        for entry in self.histograms.iter() {
            let (key, values) = entry.pair();
            let data = values.read();
            if !data.is_empty() {
                let snapshot = calculate_histogram_stats(&data);
                histograms.insert(key.clone(), snapshot);
            }
        }

        MetricsSnapshot {
            timestamp: SystemTime::now(),
            counters,
            gauges,
            histograms,
        }
    }

    /// Get uptime duration
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.counters.clear();
        self.gauges.clear();
        self.histograms.clear();
        self.total_requests.store(0, Ordering::Relaxed);
        self.total_latency_us.store(0, Ordering::Relaxed);
        self.allows.store(0, Ordering::Relaxed);
        self.denies.store(0, Ordering::Relaxed);
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
    }

    /// Increment a counter
    pub fn increment_counter(&self, name: &str, value: u64) {
        self.counters
            .entry(name.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(value, Ordering::Relaxed);
    }

    /// Update a gauge
    pub fn update_gauge(&self, name: &str, value: f64) {
        self.gauges
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(RwLock::new(0.0)))
            .write()
            .clone_from(&value);
    }

    /// Record a histogram value
    pub fn record_histogram(&self, name: &str, value: f64) {
        self.histograms
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(RwLock::new(Vec::new())))
            .write()
            .push(value);
    }
}

/// Calculate histogram statistics
fn calculate_histogram_stats(values: &[f64]) -> HistogramSnapshot {
    if values.is_empty() {
        return HistogramSnapshot {
            count: 0,
            sum: 0.0,
            min: 0.0,
            max: 0.0,
            p50: 0.0,
            p95: 0.0,
            p99: 0.0,
            p999: 0.0,
        };
    }

    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let count = sorted.len();
    let sum: f64 = sorted.iter().sum();
    let min = sorted[0];
    let max = sorted[count - 1];

    let percentile = |p: f64| -> f64 {
        let index = ((count as f64 - 1.0) * p / 100.0) as usize;
        sorted[index]
    };

    HistogramSnapshot {
        count,
        sum,
        min,
        max,
        p50: percentile(50.0),
        p95: percentile(95.0),
        p99: percentile(99.0),
        p999: percentile(99.9),
    }
}

/// Implement the metrics Recorder trait for integration
impl Recorder for MetricsCollector {
    fn describe_counter(&self, _key: KeyName, _unit: Option<Unit>, _description: SharedString) {
        // Descriptions are handled at registration time
    }

    fn describe_gauge(&self, _key: KeyName, _unit: Option<Unit>, _description: SharedString) {
        // Descriptions are handled at registration time
    }

    fn describe_histogram(&self, _key: KeyName, _unit: Option<Unit>, _description: SharedString) {
        // Descriptions are handled at registration time
    }

    fn register_counter(&self, _key: &Key) -> metrics::Counter {
        // Use the default implementation
        metrics::Counter::noop()
    }

    fn register_gauge(&self, _key: &Key) -> metrics::Gauge {
        // Use the default implementation
        metrics::Gauge::noop()
    }

    fn register_histogram(&self, _key: &Key) -> metrics::Histogram {
        // Use the default implementation
        metrics::Histogram::noop()
    }

    fn increment_counter(&self, key: &Key, value: u64) {
        let name = key.name().to_string();
        self.increment_counter(&name, value);
    }

    fn update_gauge(&self, key: &Key, value: metrics::GaugeValue) {
        let name = key.name().to_string();
        let float_value = match value {
            metrics::GaugeValue::Absolute(v) => v,
            metrics::GaugeValue::Increment(v) => {
                // Get current value and increment
                let current = self
                    .gauges
                    .entry(name.clone())
                    .or_insert_with(|| Arc::new(RwLock::new(0.0)))
                    .read()
                    .clone();
                current + v
            }
            metrics::GaugeValue::Decrement(v) => {
                // Get current value and decrement
                let current = self
                    .gauges
                    .entry(name.clone())
                    .or_insert_with(|| Arc::new(RwLock::new(0.0)))
                    .read()
                    .clone();
                current - v
            }
        };
        self.update_gauge(&name, float_value);
    }

    fn record_histogram(&self, key: &Key, value: f64) {
        let name = key.name().to_string();
        self.record_histogram(&name, value);
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collection() {
        let collector = MetricsCollector::new();

        // Record some metrics
        collector.record_request(Duration::from_millis(5), true);
        collector.record_request(Duration::from_millis(10), false);
        collector.record_cache_access(true);
        collector.record_cache_access(false);

        // Get snapshot
        let snapshot = collector.snapshot();

        // Verify counters
        assert_eq!(snapshot.counters.get("total_requests"), Some(&2));
        assert_eq!(snapshot.counters.get("allows"), Some(&1));
        assert_eq!(snapshot.counters.get("denies"), Some(&1));
        assert_eq!(snapshot.counters.get("cache_hits"), Some(&1));
        assert_eq!(snapshot.counters.get("cache_misses"), Some(&1));

        // Verify gauges
        assert!(snapshot.gauges.get("cache_hit_rate").unwrap() > 0.0);
        assert!(snapshot.gauges.get("avg_latency_us").unwrap() > 0.0);

        // Verify histogram
        assert!(snapshot.histograms.contains_key("authorization_latency"));
        let hist = &snapshot.histograms["authorization_latency"];
        assert_eq!(hist.count, 2);
    }

    #[test]
    fn test_histogram_calculation() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let stats = calculate_histogram_stats(&values);

        assert_eq!(stats.count, 10);
        assert_eq!(stats.sum, 55.0);
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 10.0);
        assert!(stats.p50 >= 5.0 && stats.p50 <= 6.0);
        assert!(stats.p95 >= 9.0);
    }

    #[test]
    fn test_reset_metrics() {
        let collector = MetricsCollector::new();

        collector.record_request(Duration::from_millis(5), true);
        collector.increment_counter("test_counter", 10);
        collector.update_gauge("test_gauge", 42.0);

        let snapshot1 = collector.snapshot();
        assert!(!snapshot1.counters.is_empty());

        collector.reset();

        let snapshot2 = collector.snapshot();
        assert_eq!(snapshot2.counters.get("total_requests"), Some(&0));
    }
}