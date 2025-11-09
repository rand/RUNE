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
use rune_core::{RequestBuilder, RUNEEngine};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, error, info, warn};

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
        .principal(&req.principal)
        .action(&req.action)
        .resource(&req.resource)
        .build()
        .map_err(|e| ApiError::BadRequest(format!("Invalid request: {}", e)))?;

    // Evaluate authorization
    let result = state
        .engine
        .authorize(&request)
        .await
        .map_err(|e| ApiError::Internal(format!("Authorization failed: {}", e)))?;

    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

    // Convert decision
    let decision = result.decision.into();

    // Build response
    let mut response = AuthorizeResponse {
        decision,
        reasons: result.reasons,
        diagnostics: None,
    };

    // Add diagnostics if in debug mode
    if state.debug || params.debug {
        response.diagnostics = Some(Diagnostics {
            evaluation_time_ms: elapsed_ms,
            cache_hit: result.cache_hit,
            rules_evaluated: result.datalog_rules_evaluated,
            policies_evaluated: result.cedar_policies_evaluated,
            matched_rules: Vec::new(), // TODO: Track matched rules
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
            .principal(&auth_req.principal)
            .action(&auth_req.action)
            .resource(&auth_req.resource)
            .build()
        {
            Ok(r) => r,
            Err(e) => {
                // Add error response for this request
                results.push(AuthorizeResponse {
                    decision: Decision::Indeterminate,
                    reasons: vec![format!("Invalid request: {}", e)],
                    diagnostics: None,
                });
                continue;
            }
        };

        // Evaluate authorization
        match state.engine.authorize(&request).await {
            Ok(result) => {
                let mut response = AuthorizeResponse {
                    decision: result.decision.into(),
                    reasons: result.reasons,
                    diagnostics: None,
                };

                // Add diagnostics if in debug mode
                if state.debug || params.debug {
                    response.diagnostics = Some(Diagnostics {
                        evaluation_time_ms: 0.0, // Not tracked per-request in batch
                        cache_hit: result.cache_hit,
                        rules_evaluated: result.datalog_rules_evaluated,
                        policies_evaluated: result.cedar_policies_evaluated,
                        matched_rules: Vec::new(),
                        matched_policies: Vec::new(),
                    });
                }

                results.push(response);
            }
            Err(e) => {
                error!("Batch authorization error: {}", e);
                results.push(AuthorizeResponse {
                    decision: Decision::Indeterminate,
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
        .principal("health:check")
        .action("health:check")
        .resource("health:check")
        .build()
        .map_err(|e| ApiError::Internal(format!("Health check failed: {}", e)))?;

    // Try to authorize
    match state.engine.authorize(&test_request).await {
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
    // Use the metrics crate to export prometheus metrics
    let encoder = metrics_exporter_prometheus::Encoder::new();
    encoder.encode_to_string().unwrap_or_default()
}