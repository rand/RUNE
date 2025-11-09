//! HTTP request handlers

use crate::api::{
    AuthorizeRequest, AuthorizeResponse, BatchAuthorizeRequest, BatchAuthorizeResponse,
    Decision, Diagnostics, HealthResponse, HealthStatus,
};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use axum::{
    extract::{Query, State},
    Json,
};
use rune_core::{RequestBuilder, Principal, Action, Resource};
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
pub async fn authorize(
    State(state): State<AppState>,
    Query(params): Query<DebugParams>,
    Json(req): Json<AuthorizeRequest>,
) -> ApiResult<Json<AuthorizeResponse>> {
    let start = Instant::now();

    debug!("Authorization request: {:?}", req);

    // Build the request
    let request = RequestBuilder::new()
        .principal(parse_principal(&req.principal))
        .action(Action::new(&req.action))
        .resource(parse_resource(&req.resource))
        .build()
        .map_err(|e| ApiError::BadRequest(format!("Invalid request: {}", e)))?;

    // Evaluate authorization
    let result = state
        .engine
        .authorize(&request)
        .map_err(|e| ApiError::Internal(format!("Authorization failed: {}", e)))?;

    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

    // Convert decision
    let decision = result.decision.into();

    // Build response
    let mut response = AuthorizeResponse {
        decision,
        reasons: vec![result.explanation],
        diagnostics: None,
    };

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
pub async fn batch_authorize(
    State(state): State<AppState>,
    Query(params): Query<DebugParams>,
    Json(req): Json<BatchAuthorizeRequest>,
) -> ApiResult<Json<BatchAuthorizeResponse>> {
    let start = Instant::now();

    debug!("Batch authorization request: {} requests", req.requests.len());

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
        loaded_rules: 0, // TODO: Get from engine
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
                loaded_rules: 0, // TODO: Get from engine
                loaded_policies: 0, // TODO: Get from engine
            }))
        }
        Err(e) => {
            warn!("Readiness check failed: {}", e);
            Err(ApiError::ServiceUnavailable(
                "Engine not ready".to_string(),
            ))
        }
    }
}

/// Prometheus metrics endpoint
pub async fn metrics() -> String {
    // TODO: Properly collect and format metrics
    // For now, return a placeholder
    "# HELP rune_requests_total Total number of authorization requests\n\
     # TYPE rune_requests_total counter\n\
     rune_requests_total 0\n".to_string()
}