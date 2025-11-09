//! Core RUNE engine with high-performance authorization

use crate::datalog::DatalogEngine;
use crate::error::Result;
use crate::facts::FactStore;
use crate::policy::PolicySet;
use crate::request::Request;
use crate::types::Value;
use dashmap::DashMap;
use parking_lot::RwLock;
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
    /// Datalog evaluation engine
    datalog: Arc<RwLock<DatalogEngine>>,
    /// Cedar policy set
    policies: Arc<RwLock<PolicySet>>,
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
        RUNEEngine {
            datalog: Arc::new(RwLock::new(DatalogEngine::new())),
            policies: Arc::new(RwLock::new(PolicySet::new())),
            facts: Arc::new(FactStore::new()),
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
                let engine = datalog.read();
                engine.evaluate(&req_clone, &facts)
            },
            || -> Result<AuthorizationResult> {
                let policy_set = policies.read();
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
            let engine = self.datalog.read();
            engine.evaluate(request, &self.facts)?
        };

        let cedar_result = {
            let policy_set = self.policies.read();
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
    use crate::types::{Action, Principal, Resource};

    #[test]
    fn test_engine_creation() {
        let engine = RUNEEngine::new();
        assert_eq!(engine.cache_stats().size, 0);
    }

    #[test]
    fn test_decision_combination() {
        assert_eq!(Decision::Forbid.combine(Decision::Permit), Decision::Forbid);
        assert_eq!(Decision::Deny.combine(Decision::Permit), Decision::Deny);
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
}
