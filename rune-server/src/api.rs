//! API request and response types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Authorization request
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizeRequest {
    /// Principal making the request (e.g., "user:alice", "role:admin")
    pub principal: String,

    /// Action being performed (e.g., "read", "write", "delete")
    pub action: String,

    /// Resource being accessed (e.g., "file:/tmp/data.txt", "api:/users/123")
    pub resource: String,

    /// Additional context for the request
    #[serde(default)]
    pub context: HashMap<String, serde_json::Value>,
}

/// Authorization response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizeResponse {
    /// Authorization decision
    pub decision: Decision,

    /// Reasons for the decision
    #[serde(default)]
    pub reasons: Vec<String>,

    /// Diagnostic information (only in debug mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<Diagnostics>,
}

/// Authorization decision
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Decision {
    /// Request is permitted
    Permit,
    /// Request is denied
    Deny,
    /// Request is explicitly forbidden
    Forbid,
}

/// Diagnostic information for debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    /// Time taken to evaluate (milliseconds)
    pub evaluation_time_ms: f64,

    /// Cache hit or miss
    pub cache_hit: bool,

    /// Number of rules evaluated
    pub rules_evaluated: usize,

    /// Number of policies evaluated
    pub policies_evaluated: usize,

    /// Matched rules
    #[serde(default)]
    pub matched_rules: Vec<String>,

    /// Matched policies
    #[serde(default)]
    pub matched_policies: Vec<String>,
}

/// Batch authorization request
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchAuthorizeRequest {
    /// Multiple authorization requests
    pub requests: Vec<AuthorizeRequest>,
}

/// Batch authorization response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchAuthorizeResponse {
    /// Results for each request
    pub results: Vec<AuthorizeResponse>,
}

/// Health check response
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    /// Service status
    pub status: HealthStatus,

    /// Service version
    pub version: String,

    /// Uptime in seconds
    pub uptime_seconds: u64,

    /// Number of loaded rules
    pub loaded_rules: usize,

    /// Number of loaded policies
    pub loaded_policies: usize,
}

/// Health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// Service is healthy
    Healthy,
    /// Service is degraded but functional
    Degraded,
    /// Service is unhealthy
    Unhealthy,
}

impl From<rune_core::Decision> for Decision {
    fn from(decision: rune_core::Decision) -> Self {
        match decision {
            rune_core::Decision::Permit => Decision::Permit,
            rune_core::Decision::Deny => Decision::Deny,
            rune_core::Decision::Forbid => Decision::Forbid,
        }
    }
}