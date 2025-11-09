//! Core RUNE engine with high-performance authorization

use crate::datalog::DatalogEngine;
use crate::error::Result;
use crate::facts::FactStore;
use crate::policy::PolicySet;
use crate::request::Request;
use crate::types::Value;
use arc_swap::ArcSwap;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{instrument, trace};

/// Authorization decision
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decision {
    /// Request is permitted
    Permit,
    /// Request is denied (no matching permit)
    Deny,
    /// Request is explicitly forbidden
    Forbid,
}

impl Decision {
    /// Check if decision allows the action
    pub fn is_permitted(&self) -> bool {
        matches!(self, Decision::Permit)
    }

    /// Combine decisions (forbid > deny > permit)
    pub fn combine(self, other: Decision) -> Decision {
        match (self, other) {
            (Decision::Forbid, _) | (_, Decision::Forbid) => Decision::Forbid,
            (Decision::Deny, _) | (_, Decision::Deny) => Decision::Deny,
            (Decision::Permit, Decision::Permit) => Decision::Permit,
        }
    }
}

/// Authorization result with details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationResult {
    /// The decision
    pub decision: Decision,
    /// Explanation for the decision
    pub explanation: String,
    /// Rules that were evaluated
    pub evaluated_rules: Vec<String>,
    /// Facts that were used
    pub facts_used: Vec<String>,
    /// Evaluation time in nanoseconds
    pub evaluation_time_ns: u64,
    /// Whether result was cached
    pub cached: bool,
}

/// Engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    /// Maximum cache size
    pub cache_size: usize,
    /// Cache TTL in seconds
    pub cache_ttl_secs: u64,
    /// Enable parallel evaluation
    pub parallel_eval: bool,
    /// Evaluation timeout in milliseconds
    pub timeout_ms: u64,
}

impl Default for EngineConfig {
    fn default() -> Self {
        EngineConfig {
            cache_size: 10_000,
            cache_ttl_secs: 60,
            parallel_eval: true,
            timeout_ms: 100,
        }
    }
}

/// Cache entry for authorization decisions
struct CacheEntry {
    result: AuthorizationResult,
    timestamp: Instant,
}

/// Main RUNE engine
pub struct RUNEEngine {
    /// Datalog evaluation engine (lock-free with ArcSwap for hot-reload)
    datalog: Arc<ArcSwap<DatalogEngine>>,
    /// Cedar policy set (lock-free with ArcSwap for hot-reload)
    policies: Arc<ArcSwap<PolicySet>>,
    /// Fact store
    facts: Arc<FactStore>,
    /// Decision cache
    cache: DashMap<u64, CacheEntry>,
    /// Engine configuration
    config: Arc<EngineConfig>,
    /// Metrics
    metrics: Arc<EngineMetrics>,
}

impl RUNEEngine {
    /// Create a new engine with default configuration
    pub fn new() -> Self {
        Self::with_config(EngineConfig::default())
    }

    /// Create a new engine with specified configuration
    pub fn with_config(config: EngineConfig) -> Self {
        let facts = Arc::new(FactStore::new());
        RUNEEngine {
            datalog: Arc::new(ArcSwap::new(Arc::new(DatalogEngine::empty(facts.clone())))),
            policies: Arc::new(ArcSwap::new(Arc::new(PolicySet::new()))),
            facts,
            cache: DashMap::new(),
            config: Arc::new(config),
            metrics: Arc::new(EngineMetrics::new()),
        }
    }

    /// Authorize a request
    #[instrument(skip(self), fields(request_id = %request.request_id))]
    pub fn authorize(&self, request: &Request) -> Result<AuthorizationResult> {
        let start = Instant::now();

        // Check cache first
        let cache_key = request.cache_key();
        if let Some(entry) = self.cache.get(&cache_key) {
            if start.duration_since(entry.timestamp).as_secs() < self.config.cache_ttl_secs {
                self.metrics.record_cache_hit();
                trace!("Cache hit for request");

                let mut result = entry.result.clone();
                result.cached = true;
                return Ok(result);
            } else {
                // Remove stale entry
                drop(entry);
                self.cache.remove(&cache_key);
            }
        }

        self.metrics.record_cache_miss();
        trace!("Cache miss, evaluating request");

        // Evaluate in parallel if configured
        let (datalog_result, cedar_result) = if self.config.parallel_eval {
            self.evaluate_parallel(request)?
        } else {
            self.evaluate_sequential(request)?
        };

        // Combine results
        let decision = datalog_result.decision.combine(cedar_result.decision);

        let explanation = match decision {
            Decision::Permit => format!(
                "Permitted by {} rules",
                datalog_result.evaluated_rules.len() + cedar_result.evaluated_rules.len()
            ),
            Decision::Deny => "No matching permit rules".to_string(),
            Decision::Forbid => {
                if cedar_result.decision == Decision::Forbid {
                    cedar_result.explanation
                } else {
                    datalog_result.explanation
                }
            }
        };

        let mut evaluated_rules = datalog_result.evaluated_rules;
        evaluated_rules.extend(cedar_result.evaluated_rules);

        let mut facts_used = datalog_result.facts_used;
        facts_used.extend(cedar_result.facts_used);

        let result = AuthorizationResult {
            decision,
            explanation,
            evaluated_rules,
            facts_used,
            evaluation_time_ns: start.elapsed().as_nanos() as u64,
            cached: false,
        };

        // Cache the result
        self.cache.insert(
            cache_key,
            CacheEntry {
                result: result.clone(),
                timestamp: start,
            },
        );

        // Record metrics
        self.metrics.record_authorization(decision, start.elapsed());

        Ok(result)
    }

    /// Evaluate in parallel using rayon
    fn evaluate_parallel(
        &self,
        request: &Request,
    ) -> Result<(AuthorizationResult, AuthorizationResult)> {
        let datalog = self.datalog.clone();
        let policies = self.policies.clone();
        let facts = self.facts.clone();
        let req_clone = request.clone();

        // Use rayon's parallel join for two tasks
        let (datalog_result, cedar_result) = rayon::join(
            || -> Result<AuthorizationResult> {
                let engine = datalog.load();
                engine.evaluate(&req_clone, &facts)
            },
            || -> Result<AuthorizationResult> {
                let policy_set = policies.load();
                policy_set.evaluate(&req_clone)
            },
        );

        Ok((datalog_result?, cedar_result?))
    }

    /// Evaluate sequentially
    fn evaluate_sequential(
        &self,
        request: &Request,
    ) -> Result<(AuthorizationResult, AuthorizationResult)> {
        let datalog_result = {
            let engine = self.datalog.load();
            engine.evaluate(request, &self.facts)?
        };

        let cedar_result = {
            let policy_set = self.policies.load();
            policy_set.evaluate(request)?
        };

        Ok((datalog_result, cedar_result))
    }

    /// Load configuration from a RUNE file
    pub fn load_configuration(&self, _config_path: &str) -> Result<()> {
        // This will be implemented with the parser
        todo!("Implement configuration loading")
    }

    /// Add a fact to the engine
    pub fn add_fact(&self, predicate: impl Into<String>, args: Vec<Value>) {
        self.facts
            .add_fact(crate::facts::Fact::new(predicate, args));
    }

    /// Clear the decision cache
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> CacheStats {
        CacheStats {
            size: self.cache.len(),
            hit_rate: self.metrics.cache_hit_rate(),
        }
    }

    /// Get engine metrics
    pub fn metrics(&self) -> Arc<EngineMetrics> {
        self.metrics.clone()
    }

    /// Hot-reload Datalog rules (zero-downtime atomic swap)
    ///
    /// This method atomically replaces the DatalogEngine with a new one containing
    /// updated rules. Ongoing authorization requests continue using the old engine
    /// until they complete. The old engine is automatically cleaned up once all
    /// references are dropped.
    ///
    /// # Arguments
    /// * `rules` - New Datalog rules to evaluate
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err(_)` if the new engine cannot be created
    pub fn reload_datalog_rules(&self, rules: Vec<crate::datalog::types::Rule>) -> Result<()> {
        // Create new DatalogEngine with updated rules
        let new_engine = DatalogEngine::new(rules, self.facts.clone());

        // Atomically swap the engine (lock-free!)
        self.datalog.store(Arc::new(new_engine));

        // Clear cache since old decisions may be based on old rules
        self.clear_cache();

        trace!("Datalog rules reloaded successfully");
        Ok(())
    }

    /// Hot-reload Cedar policies (zero-downtime atomic swap)
    ///
    /// This method atomically replaces the PolicySet with a new one containing
    /// updated policies. Ongoing authorization requests continue using the old
    /// policy set until they complete.
    ///
    /// # Arguments
    /// * `policies` - New Cedar policies to evaluate
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err(_)` if the new policy set cannot be created
    pub fn reload_policies(&self, policies: PolicySet) -> Result<()> {
        // Atomically swap the policy set (lock-free!)
        self.policies.store(Arc::new(policies));

        // Clear cache since old decisions may be based on old policies
        self.clear_cache();

        trace!("Cedar policies reloaded successfully");
        Ok(())
    }

    /// Get current Datalog engine version (for testing/debugging)
    pub fn datalog_version(&self) -> Arc<DatalogEngine> {
        self.datalog.load_full()
    }

    /// Get current PolicySet version (for testing/debugging)
    pub fn policies_version(&self) -> Arc<PolicySet> {
        self.policies.load_full()
    }
}

impl Default for RUNEEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Current cache size
    pub size: usize,
    /// Cache hit rate (0.0 to 1.0)
    pub hit_rate: f64,
}

/// Engine metrics
#[derive(Debug, Clone)]
pub struct EngineMetrics {
    cache_hits: Arc<std::sync::atomic::AtomicU64>,
    cache_misses: Arc<std::sync::atomic::AtomicU64>,
    total_authorizations: Arc<std::sync::atomic::AtomicU64>,
    total_permits: Arc<std::sync::atomic::AtomicU64>,
    total_denies: Arc<std::sync::atomic::AtomicU64>,
    total_forbids: Arc<std::sync::atomic::AtomicU64>,
}

impl EngineMetrics {
    fn new() -> Self {
        use std::sync::atomic::AtomicU64;

        EngineMetrics {
            cache_hits: Arc::new(AtomicU64::new(0)),
            cache_misses: Arc::new(AtomicU64::new(0)),
            total_authorizations: Arc::new(AtomicU64::new(0)),
            total_permits: Arc::new(AtomicU64::new(0)),
            total_denies: Arc::new(AtomicU64::new(0)),
            total_forbids: Arc::new(AtomicU64::new(0)),
        }
    }

    fn record_cache_hit(&self) {
        use std::sync::atomic::Ordering;
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    fn record_cache_miss(&self) {
        use std::sync::atomic::Ordering;
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    fn record_authorization(&self, decision: Decision, _duration: Duration) {
        use std::sync::atomic::Ordering;

        self.total_authorizations.fetch_add(1, Ordering::Relaxed);

        match decision {
            Decision::Permit => self.total_permits.fetch_add(1, Ordering::Relaxed),
            Decision::Deny => self.total_denies.fetch_add(1, Ordering::Relaxed),
            Decision::Forbid => self.total_forbids.fetch_add(1, Ordering::Relaxed),
        };
    }

    fn cache_hit_rate(&self) -> f64 {
        use std::sync::atomic::Ordering;

        let hits = self.cache_hits.load(Ordering::Relaxed) as f64;
        let misses = self.cache_misses.load(Ordering::Relaxed) as f64;

        if hits + misses == 0.0 {
            0.0
        } else {
            hits / (hits + misses)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datalog::types::Rule;
    use crate::types::{Action, Principal, Resource};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_engine_creation() {
        let engine = RUNEEngine::new();
        assert_eq!(engine.cache_stats().size, 0);
    }

    #[test]
    fn test_engine_with_custom_config() {
        let config = EngineConfig {
            cache_size: 5000,
            cache_ttl_secs: 30,
            parallel_eval: false,
            timeout_ms: 200,
        };
        let engine = RUNEEngine::with_config(config.clone());
        assert_eq!(engine.config.cache_size, 5000);
        assert_eq!(engine.config.cache_ttl_secs, 30);
        assert!(!engine.config.parallel_eval);
        assert_eq!(engine.config.timeout_ms, 200);
    }

    #[test]
    fn test_engine_default() {
        let engine = RUNEEngine::default();
        assert_eq!(engine.cache_stats().size, 0);
    }

    #[test]
    fn test_config_default() {
        let config = EngineConfig::default();
        assert_eq!(config.cache_size, 10_000);
        assert_eq!(config.cache_ttl_secs, 60);
        assert!(config.parallel_eval);
        assert_eq!(config.timeout_ms, 100);
    }

    #[test]
    fn test_decision_is_permitted() {
        assert!(Decision::Permit.is_permitted());
        assert!(!Decision::Deny.is_permitted());
        assert!(!Decision::Forbid.is_permitted());
    }

    #[test]
    fn test_decision_combination() {
        // Forbid takes priority over everything
        assert_eq!(Decision::Forbid.combine(Decision::Permit), Decision::Forbid);
        assert_eq!(Decision::Forbid.combine(Decision::Deny), Decision::Forbid);
        assert_eq!(Decision::Forbid.combine(Decision::Forbid), Decision::Forbid);
        assert_eq!(Decision::Permit.combine(Decision::Forbid), Decision::Forbid);
        assert_eq!(Decision::Deny.combine(Decision::Forbid), Decision::Forbid);

        // Deny takes priority over Permit
        assert_eq!(Decision::Deny.combine(Decision::Permit), Decision::Deny);
        assert_eq!(Decision::Permit.combine(Decision::Deny), Decision::Deny);
        assert_eq!(Decision::Deny.combine(Decision::Deny), Decision::Deny);

        // Both Permit results in Permit
        assert_eq!(Decision::Permit.combine(Decision::Permit), Decision::Permit);
    }

    #[test]
    fn test_cache_key_generation() {
        let request1 = Request::new(
            Principal::agent("agent1"),
            Action::new("read"),
            Resource::file("/tmp/test.txt"),
        );

        let request2 = Request::new(
            Principal::agent("agent1"),
            Action::new("read"),
            Resource::file("/tmp/test.txt"),
        );

        assert_eq!(request1.cache_key(), request2.cache_key());

        let request3 = Request::new(
            Principal::agent("agent2"),
            Action::new("read"),
            Resource::file("/tmp/test.txt"),
        );

        assert_ne!(request1.cache_key(), request3.cache_key());
    }

    #[test]
    fn test_basic_authorization() {
        let engine = RUNEEngine::new();
        let request = Request::new(
            Principal::agent("alice"),
            Action::new("read"),
            Resource::file("/data/public.txt"),
        );

        let result = engine.authorize(&request).expect("Authorization failed");
        assert!(!result.cached);
        assert!(result.evaluation_time_ns > 0);
    }

    #[test]
    fn test_authorization_with_context() {
        let engine = RUNEEngine::new();
        let request = Request::new(
            Principal::agent("alice"),
            Action::new("read"),
            Resource::file("/data/public.txt"),
        )
        .with_context("ip_address", Value::String("192.168.1.1".to_string()))
        .with_context("time", Value::String("2024-01-01T12:00:00Z".to_string()));

        let result = engine.authorize(&request).expect("Authorization failed");
        assert!(!result.cached);
    }

    #[test]
    fn test_cache_hit() {
        let engine = RUNEEngine::new();
        let request = Request::new(
            Principal::agent("bob"),
            Action::new("write"),
            Resource::file("/data/private.txt"),
        );

        // First authorization - should miss cache
        let result1 = engine.authorize(&request).expect("Authorization failed");
        assert!(!result1.cached);

        // Second authorization - should hit cache
        let result2 = engine.authorize(&request).expect("Authorization failed");
        assert!(result2.cached);

        // Cache stats should reflect this
        let stats = engine.cache_stats();
        assert_eq!(stats.size, 1);
        assert_eq!(stats.hit_rate, 0.5); // 1 hit out of 2 requests
    }

    #[test]
    fn test_cache_ttl_expiry() {
        let config = EngineConfig {
            cache_size: 100,
            cache_ttl_secs: 1, // Very short TTL
            parallel_eval: true,
            timeout_ms: 100,
        };
        let engine = RUNEEngine::with_config(config);

        let request = Request::new(
            Principal::agent("charlie"),
            Action::new("execute"),
            Resource::file("/bin/script.sh"),
        );

        // First authorization
        let result1 = engine.authorize(&request).expect("Authorization failed");
        assert!(!result1.cached);

        // Wait for TTL to expire
        thread::sleep(Duration::from_secs(2));

        // Second authorization - cache should be expired
        let result2 = engine.authorize(&request).expect("Authorization failed");
        assert!(!result2.cached);
    }

    #[test]
    fn test_cache_clear() {
        let engine = RUNEEngine::new();
        let request = Request::new(
            Principal::agent("dave"),
            Action::new("delete"),
            Resource::file("/data/temp.txt"),
        );

        // Populate cache
        engine.authorize(&request).expect("Authorization failed");
        assert_eq!(engine.cache_stats().size, 1);

        // Clear cache
        engine.clear_cache();
        assert_eq!(engine.cache_stats().size, 0);

        // Next request should miss cache
        let result = engine.authorize(&request).expect("Authorization failed");
        assert!(!result.cached);
    }

    #[test]
    fn test_metrics_tracking() {
        let engine = RUNEEngine::new();
        let metrics = engine.metrics();

        let request1 = Request::new(
            Principal::agent("eve"),
            Action::new("read"),
            Resource::file("/data/file1.txt"),
        );
        let request2 = Request::new(
            Principal::agent("frank"),
            Action::new("read"),
            Resource::file("/data/file2.txt"),
        );

        // Perform some authorizations
        engine.authorize(&request1).expect("Authorization failed");
        engine.authorize(&request1).expect("Authorization failed"); // Cache hit
        engine.authorize(&request2).expect("Authorization failed");

        // Check metrics
        use std::sync::atomic::Ordering;
        assert_eq!(metrics.cache_hits.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.cache_misses.load(Ordering::Relaxed), 2);
        assert_eq!(metrics.total_authorizations.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn test_cache_hit_rate_zero_requests() {
        let metrics = EngineMetrics::new();
        assert_eq!(metrics.cache_hit_rate(), 0.0);
    }

    #[test]
    fn test_cache_hit_rate_calculation() {
        let metrics = EngineMetrics::new();

        // 3 misses, 2 hits = 0.4 hit rate
        metrics.record_cache_miss();
        metrics.record_cache_miss();
        metrics.record_cache_miss();
        metrics.record_cache_hit();
        metrics.record_cache_hit();

        assert_eq!(metrics.cache_hit_rate(), 0.4);
    }

    #[test]
    fn test_add_fact() {
        let engine = RUNEEngine::new();
        engine.add_fact("user", vec![Value::String("alice".to_string())]);
        engine.add_fact(
            "role",
            vec![
                Value::String("alice".to_string()),
                Value::String("admin".to_string()),
            ],
        );

        // Facts should be in the store (we can't easily verify without exposing the fact store)
        // but at least ensure it doesn't panic
    }

    #[test]
    fn test_sequential_evaluation() {
        let config = EngineConfig {
            cache_size: 100,
            cache_ttl_secs: 60,
            parallel_eval: false, // Force sequential
            timeout_ms: 100,
        };
        let engine = RUNEEngine::with_config(config);

        let request = Request::new(
            Principal::agent("grace"),
            Action::new("read"),
            Resource::file("/data/sequential.txt"),
        );

        let result = engine.authorize(&request).expect("Authorization failed");
        assert!(!result.cached);
        assert!(result.evaluation_time_ns > 0);
    }

    #[test]
    fn test_parallel_evaluation() {
        let config = EngineConfig {
            cache_size: 100,
            cache_ttl_secs: 60,
            parallel_eval: true, // Force parallel
            timeout_ms: 100,
        };
        let engine = RUNEEngine::with_config(config);

        let request = Request::new(
            Principal::agent("heidi"),
            Action::new("read"),
            Resource::file("/data/parallel.txt"),
        );

        let result = engine.authorize(&request).expect("Authorization failed");
        assert!(!result.cached);
        assert!(result.evaluation_time_ns > 0);
    }

    #[test]
    fn test_reload_datalog_rules() {
        let engine = RUNEEngine::new();

        // Add some facts
        engine.add_fact("user", vec![Value::String("alice".to_string())]);

        // Create new rules (empty for this test)
        let new_rules: Vec<Rule> = vec![];

        // Reload rules
        engine
            .reload_datalog_rules(new_rules)
            .expect("Failed to reload rules");

        // Cache should be cleared
        assert_eq!(engine.cache_stats().size, 0);
    }

    #[test]
    fn test_reload_policies() {
        let engine = RUNEEngine::new();

        // Populate cache first
        let request = Request::new(
            Principal::agent("iris"),
            Action::new("read"),
            Resource::file("/data/test.txt"),
        );
        engine.authorize(&request).expect("Authorization failed");
        assert_eq!(engine.cache_stats().size, 1);

        // Reload policies with new set
        let new_policies = PolicySet::new();
        engine
            .reload_policies(new_policies)
            .expect("Failed to reload policies");

        // Cache should be cleared
        assert_eq!(engine.cache_stats().size, 0);
    }

    #[test]
    fn test_datalog_version() {
        let engine = RUNEEngine::new();
        let version = engine.datalog_version();
        assert_eq!(version.rules().len(), 0); // Empty engine
    }

    #[test]
    fn test_policies_version() {
        let engine = RUNEEngine::new();
        let version = engine.policies_version();
        // Just ensure we can get the version
        let _ = version;
    }

    #[test]
    fn test_concurrent_authorizations() {
        use std::sync::Arc;

        let engine = Arc::new(RUNEEngine::new());
        let mut handles = vec![];

        // Spawn multiple threads performing authorizations
        for i in 0..10 {
            let engine_clone = engine.clone();
            let handle = thread::spawn(move || {
                let request = Request::new(
                    Principal::agent(&format!("user_{}", i)),
                    Action::new("read"),
                    Resource::file(&format!("/data/file_{}.txt", i)),
                );
                engine_clone
                    .authorize(&request)
                    .expect("Authorization failed")
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().expect("Thread panicked");
        }

        // All authorizations should have completed
        let metrics = engine.metrics();
        use std::sync::atomic::Ordering;
        assert_eq!(metrics.total_authorizations.load(Ordering::Relaxed), 10);
    }

    #[test]
    fn test_authorization_result_fields() {
        let engine = RUNEEngine::new();
        let request = Request::new(
            Principal::agent("judy"),
            Action::new("read"),
            Resource::file("/data/test.txt"),
        );

        let result = engine.authorize(&request).expect("Authorization failed");

        // Verify all fields are populated
        assert!(!result.explanation.is_empty());
        assert!(result.evaluation_time_ns > 0);
        assert!(!result.cached);
        // evaluated_rules and facts_used may be empty but should exist
        let _ = result.evaluated_rules;
        let _ = result.facts_used;
    }

    #[test]
    fn test_decision_serialization() {
        // Ensure Decision can be serialized/deserialized
        use serde_json;

        let permit = Decision::Permit;
        let json = serde_json::to_string(&permit).expect("Failed to serialize");
        let deserialized: Decision =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(permit, deserialized);
    }

    #[test]
    fn test_config_serialization() {
        use serde_json;

        let config = EngineConfig::default();
        let json = serde_json::to_string(&config).expect("Failed to serialize");
        let deserialized: EngineConfig =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(config.cache_size, deserialized.cache_size);
    }

    #[test]
    fn test_cache_stats_serialization() {
        use serde_json;

        let stats = CacheStats {
            size: 100,
            hit_rate: 0.75,
        };
        let json = serde_json::to_string(&stats).expect("Failed to serialize");
        let deserialized: CacheStats =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(stats.size, deserialized.size);
        assert_eq!(stats.hit_rate, deserialized.hit_rate);
    }

    #[test]
    fn test_multiple_cache_entries() {
        let engine = RUNEEngine::new();

        // Create multiple different requests
        for i in 0..5 {
            let request = Request::new(
                Principal::agent(&format!("user_{}", i)),
                Action::new("read"),
                Resource::file(&format!("/data/file_{}.txt", i)),
            );
            engine.authorize(&request).expect("Authorization failed");
        }

        // All should be cached
        assert_eq!(engine.cache_stats().size, 5);

        // Authorizing the same requests again should hit cache
        for i in 0..5 {
            let request = Request::new(
                Principal::agent(&format!("user_{}", i)),
                Action::new("read"),
                Resource::file(&format!("/data/file_{}.txt", i)),
            );
            let result = engine.authorize(&request).expect("Authorization failed");
            assert!(result.cached);
        }

        // Hit rate should be 0.5 (5 hits out of 10 total)
        assert_eq!(engine.cache_stats().hit_rate, 0.5);
    }

    #[test]
    fn test_metrics_decision_counts() {
        let engine = RUNEEngine::new();

        // Perform authorizations (they will all be Deny since we have no rules)
        for i in 0..3 {
            let request = Request::new(
                Principal::agent(&format!("user_{}", i)),
                Action::new("read"),
                Resource::file(&format!("/file_{}.txt", i)),
            );
            engine.authorize(&request).expect("Authorization failed");
        }

        let metrics = engine.metrics();
        use std::sync::atomic::Ordering;

        // All should be denies (no rules configured)
        assert_eq!(metrics.total_authorizations.load(Ordering::Relaxed), 3);
        assert_eq!(metrics.total_denies.load(Ordering::Relaxed), 3);
        assert_eq!(metrics.total_permits.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.total_forbids.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_engine_with_facts() {
        let engine = RUNEEngine::new();

        // Add some facts
        engine.add_fact(
            "has_role",
            vec![
                Value::String("alice".to_string()),
                Value::String("admin".to_string()),
            ],
        );
        engine.add_fact(
            "has_role",
            vec![
                Value::String("bob".to_string()),
                Value::String("user".to_string()),
            ],
        );

        // Authorize a request
        let request = Request::new(
            Principal::agent("alice"),
            Action::new("read"),
            Resource::file("/admin/config.txt"),
        );

        let result = engine.authorize(&request).expect("Authorization failed");
        assert!(!result.cached);
    }

    #[test]
    fn test_authorization_result_explanation_permit() {
        let engine = RUNEEngine::new();

        // Add facts to trigger permit (Datalog will return non-empty facts)
        engine.add_fact("allow", vec![Value::String("test".to_string())]);

        let request = Request::new(
            Principal::agent("test"),
            Action::new("read"),
            Resource::file("/test.txt"),
        );

        let result = engine.authorize(&request).expect("Authorization failed");
        // The explanation should contain "Permitted by" for permit decisions
        // (though with empty rules, actual decision depends on evaluation)
        assert!(!result.explanation.is_empty());
    }
}
