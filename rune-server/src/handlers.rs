//! HTTP request handlers

use crate::api::{
    AuthorizeRequest, AuthorizeResponse, BatchAuthorizeRequest, BatchAuthorizeResponse, Decision,
    Diagnostics, HealthResponse, HealthStatus,
};
use crate::error::{ApiError, ApiResult};
use crate::metrics;
use crate::state::AppState;
use axum::{
    extract::{Query, State},
    Json,
};
use rune_core::{Action, Principal, RequestBuilder, Resource};
use serde::Deserialize;
use std::time::Instant;
use tracing::{debug, error, info, warn};

/// Parse a principal string (format: "type:id" or just "id")
fn parse_principal(s: &str) -> Principal {
    if let Some((typ, id)) = s.split_once(':') {
        Principal::new(typ, id)
    } else {
        Principal::new("User", s)
    }
}

/// Parse a resource string (format: "type:id" or "type:path/to/resource")
fn parse_resource(s: &str) -> Resource {
    if let Some((typ, id)) = s.split_once(':') {
        Resource::new(typ, id)
    } else {
        Resource::new("Resource", s)
    }
}

/// Query parameters for debug mode
#[derive(Debug, Deserialize)]
pub struct DebugParams {
    #[serde(default)]
    debug: bool,
}

/// Handle authorization request
#[tracing::instrument(
    name = "authorize",
    skip(state, params),
    fields(
        principal = %req.principal,
        action = %req.action,
        resource = %req.resource,
        decision = tracing::field::Empty,
        latency_ms = tracing::field::Empty,
    )
)]
pub async fn authorize(
    State(state): State<AppState>,
    Query(params): Query<DebugParams>,
    Json(req): Json<AuthorizeRequest>,
) -> ApiResult<Json<AuthorizeResponse>> {
    let start = Instant::now();

    debug!("Authorization request: {:?}", req);

    // Build the request with tracing
    let request = crate::tracing::trace_parse_request(|| {
        RequestBuilder::new()
            .principal(parse_principal(&req.principal))
            .action(Action::new(&req.action))
            .resource(parse_resource(&req.resource))
            .build()
            .map_err(|e| ApiError::BadRequest(format!("Invalid request: {}", e)))
    })?;

    // Evaluate authorization with tracing
    let result = crate::tracing::trace_datalog_evaluation(0, || {
        state
            .engine
            .authorize(&request)
            .map_err(|e| ApiError::Internal(format!("Authorization failed: {}", e)))
    })?;

    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

    // Convert decision
    let decision = result.decision.into();

    // Record metrics and tracing
    let decision_str = match decision {
        Decision::Permit => "permit",
        Decision::Deny => "deny",
        Decision::Forbid => "forbid",
    };
    metrics::record_authorization(decision_str, elapsed_ms / 1000.0, result.cached);
    metrics::record_rule_evaluations(result.evaluated_rules.len());

    // Record decision in trace
    crate::tracing::record_decision(decision_str, elapsed_ms);

    // Build response with tracing
    let mut response = crate::tracing::trace_format_response(|| AuthorizeResponse {
        decision,
        reasons: vec![result.explanation],
        diagnostics: None,
    });

    // Add diagnostics if in debug mode
    if state.debug || params.debug {
        response.diagnostics = Some(Diagnostics {
            evaluation_time_ms: elapsed_ms,
            cache_hit: result.cached,
            rules_evaluated: result.evaluated_rules.len(),
            policies_evaluated: 0, // TODO: Track Cedar policies
            matched_rules: result.evaluated_rules,
            matched_policies: Vec::new(), // TODO: Track matched policies
        });
    }

    info!(
        "Authorization: {} {} {} -> {:?} ({:.2}ms)",
        req.principal, req.action, req.resource, decision, elapsed_ms
    );

    Ok(Json(response))
}

/// Handle batch authorization request
#[tracing::instrument(
    name = "batch_authorize",
    skip(state, params),
    fields(
        batch_size = req.requests.len(),
        latency_ms = tracing::field::Empty,
    )
)]
pub async fn batch_authorize(
    State(state): State<AppState>,
    Query(params): Query<DebugParams>,
    Json(req): Json<BatchAuthorizeRequest>,
) -> ApiResult<Json<BatchAuthorizeResponse>> {
    let start = Instant::now();

    debug!(
        "Batch authorization request: {} requests",
        req.requests.len()
    );

    if req.requests.is_empty() {
        return Err(ApiError::BadRequest("No requests provided".to_string()));
    }

    if req.requests.len() > 100 {
        return Err(ApiError::BadRequest(
            "Too many requests (max 100)".to_string(),
        ));
    }

    let mut results = Vec::with_capacity(req.requests.len());

    // Process each request
    for auth_req in req.requests {
        let request = match RequestBuilder::new()
            .principal(parse_principal(&auth_req.principal))
            .action(Action::new(&auth_req.action))
            .resource(parse_resource(&auth_req.resource))
            .build()
        {
            Ok(r) => r,
            Err(e) => {
                // Add error response for this request
                results.push(AuthorizeResponse {
                    decision: Decision::Forbid,
                    reasons: vec![format!("Invalid request: {}", e)],
                    diagnostics: None,
                });
                continue;
            }
        };

        // Evaluate authorization
        match state.engine.authorize(&request) {
            Ok(result) => {
                let mut response = AuthorizeResponse {
                    decision: result.decision.into(),
                    reasons: vec![result.explanation],
                    diagnostics: None,
                };

                // Add diagnostics if in debug mode
                if state.debug || params.debug {
                    response.diagnostics = Some(Diagnostics {
                        evaluation_time_ms: 0.0, // Not tracked per-request in batch
                        cache_hit: result.cached,
                        rules_evaluated: result.evaluated_rules.len(),
                        policies_evaluated: 0, // TODO: Track Cedar policies
                        matched_rules: result.evaluated_rules,
                        matched_policies: Vec::new(),
                    });
                }

                results.push(response);
            }
            Err(e) => {
                error!("Batch authorization error: {}", e);
                results.push(AuthorizeResponse {
                    decision: Decision::Forbid,
                    reasons: vec![format!("Authorization error: {}", e)],
                    diagnostics: None,
                });
            }
        }
    }

    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

    // Record batch metrics and tracing
    metrics::record_batch_authorization(results.len(), elapsed_ms / 1000.0);
    tracing::Span::current().record("latency_ms", elapsed_ms);

    info!(
        "Batch authorization: {} requests processed in {:.2}ms",
        results.len(),
        elapsed_ms
    );

    Ok(Json(BatchAuthorizeResponse { results }))
}

/// Health check - liveness probe
pub async fn health_live(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: HealthStatus::Healthy,
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: state.uptime_seconds(),
        loaded_rules: 0,    // TODO: Get from engine
        loaded_policies: 0, // TODO: Get from engine
    })
}

/// Health check - readiness probe
pub async fn health_ready(State(state): State<AppState>) -> ApiResult<Json<HealthResponse>> {
    // Check if engine is ready by doing a simple authorization
    let test_request = RequestBuilder::new()
        .principal(Principal::new("health", "check"))
        .action(Action::new("health:check"))
        .resource(Resource::new("health", "check"))
        .build()
        .map_err(|e| ApiError::Internal(format!("Health check failed: {}", e)))?;

    // Try to authorize
    match state.engine.authorize(&test_request) {
        Ok(_) => {
            Ok(Json(HealthResponse {
                status: HealthStatus::Healthy,
                version: env!("CARGO_PKG_VERSION").to_string(),
                uptime_seconds: state.uptime_seconds(),
                loaded_rules: 0,    // TODO: Get from engine
                loaded_policies: 0, // TODO: Get from engine
            }))
        }
        Err(e) => {
            warn!("Readiness check failed: {}", e);
            Err(ApiError::ServiceUnavailable("Engine not ready".to_string()))
        }
    }
}

/// Prometheus metrics endpoint
pub async fn metrics() -> String {
    metrics::get_prometheus_metrics()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_principal_with_type() {
        let principal = parse_principal("User:alice");
        assert_eq!(&*principal.entity.entity_type, "User");
        assert_eq!(&*principal.entity.id, "alice");
    }

    #[test]
    fn test_parse_principal_without_type() {
        let principal = parse_principal("bob");
        assert_eq!(&*principal.entity.entity_type, "User");
        assert_eq!(&*principal.entity.id, "bob");
    }

    #[test]
    fn test_parse_principal_with_colon_in_id() {
        let principal = parse_principal("Service:api:v2:production");
        assert_eq!(&*principal.entity.entity_type, "Service");
        assert_eq!(&*principal.entity.id, "api:v2:production");
    }

    #[test]
    fn test_parse_principal_empty() {
        let principal = parse_principal("");
        assert_eq!(&*principal.entity.entity_type, "User");
        assert_eq!(&*principal.entity.id, "");
    }

    #[test]
    fn test_parse_principal_various_types() {
        let test_cases = vec![
            ("Admin:root", "Admin", "root"),
            ("Service:database", "Service", "database"),
            ("Group:admins", "Group", "admins"),
            ("Role:viewer", "Role", "viewer"),
            ("anonymous", "User", "anonymous"),
        ];

        for (input, expected_type, expected_id) in test_cases {
            let principal = parse_principal(input);
            assert_eq!(&*principal.entity.entity_type, expected_type);
            assert_eq!(&*principal.entity.id, expected_id);
        }
    }

    #[test]
    fn test_parse_resource_with_type() {
        let resource = parse_resource("File:/tmp/data.txt");
        assert_eq!(&*resource.entity.entity_type, "File");
        assert_eq!(&*resource.entity.id, "/tmp/data.txt");
    }

    #[test]
    fn test_parse_resource_without_type() {
        let resource = parse_resource("/var/log/app.log");
        assert_eq!(&*resource.entity.entity_type, "Resource");
        assert_eq!(&*resource.entity.id, "/var/log/app.log");
    }

    #[test]
    fn test_parse_resource_with_colon_in_path() {
        let resource = parse_resource("Database:postgres://localhost:5432/mydb");
        assert_eq!(&*resource.entity.entity_type, "Database");
        assert_eq!(&*resource.entity.id, "postgres://localhost:5432/mydb");
    }

    #[test]
    fn test_parse_resource_empty() {
        let resource = parse_resource("");
        assert_eq!(&*resource.entity.entity_type, "Resource");
        assert_eq!(&*resource.entity.id, "");
    }

    #[test]
    fn test_parse_resource_various_types() {
        let test_cases = vec![
            ("API:/users/123", "API", "/users/123"),
            ("S3:bucket/key", "S3", "bucket/key"),
            ("Document:doc-123", "Document", "doc-123"),
            ("Queue:messages", "Queue", "messages"),
            ("simple-resource", "Resource", "simple-resource"),
        ];

        for (input, expected_type, expected_id) in test_cases {
            let resource = parse_resource(input);
            assert_eq!(&*resource.entity.entity_type, expected_type);
            assert_eq!(&*resource.entity.id, expected_id);
        }
    }

    #[test]
    fn test_debug_params_default() {
        let json = "{}";
        let params: DebugParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.debug, false);
    }

    #[test]
    fn test_debug_params_true() {
        let json = r#"{"debug": true}"#;
        let params: DebugParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.debug, true);
    }

    #[test]
    fn test_debug_params_false() {
        let json = r#"{"debug": false}"#;
        let params: DebugParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.debug, false);
    }

    #[test]
    fn test_debug_params_with_extra_fields() {
        // Should ignore extra fields
        let json = r#"{"debug": true, "extra": "field"}"#;
        let params: DebugParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.debug, true);
    }

    #[test]
    fn test_parse_principal_special_characters() {
        let test_cases = vec![
            ("User:alice@example.com", "User", "alice@example.com"),
            ("Service:api/v2", "Service", "api/v2"),
            ("Group:team-123", "Group", "team-123"),
            ("Role:read_only", "Role", "read_only"),
        ];

        for (input, expected_type, expected_id) in test_cases {
            let principal = parse_principal(input);
            assert_eq!(&*principal.entity.entity_type, expected_type);
            assert_eq!(&*principal.entity.id, expected_id);
        }
    }

    #[test]
    fn test_parse_resource_urls() {
        let test_cases = vec![
            ("HTTP:https://api.example.com/users", "HTTP", "https://api.example.com/users"),
            ("FTP:ftp://files.example.com/data", "FTP", "ftp://files.example.com/data"),
            ("File:file:///home/user/doc.txt", "File", "file:///home/user/doc.txt"),
        ];

        for (input, expected_type, expected_id) in test_cases {
            let resource = parse_resource(input);
            assert_eq!(&*resource.entity.entity_type, expected_type);
            assert_eq!(&*resource.entity.id, expected_id);
        }
    }

    #[test]
    fn test_parse_principal_with_only_colon() {
        let principal = parse_principal(":");
        assert_eq!(&*principal.entity.entity_type, "");
        assert_eq!(&*principal.entity.id, "");
    }

    #[test]
    fn test_parse_resource_with_only_colon() {
        let resource = parse_resource(":");
        assert_eq!(&*resource.entity.entity_type, "");
        assert_eq!(&*resource.entity.id, "");
    }

    #[test]
    fn test_parse_principal_with_multiple_colons() {
        let principal = parse_principal("Type:part1:part2:part3");
        assert_eq!(&*principal.entity.entity_type, "Type");
        assert_eq!(&*principal.entity.id, "part1:part2:part3");
    }

    #[test]
    fn test_parse_resource_with_windows_path() {
        let resource = parse_resource("File:C:\\Users\\Documents\\file.txt");
        assert_eq!(&*resource.entity.entity_type, "File");
        assert_eq!(&*resource.entity.id, "C:\\Users\\Documents\\file.txt");
    }
}
