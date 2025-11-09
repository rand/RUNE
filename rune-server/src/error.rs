//! Error types for the HTTP API

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::fmt;

/// API error type
#[derive(Debug)]
pub enum ApiError {
    /// Bad request (400)
    BadRequest(String),

    /// Unauthorized (401)
    Unauthorized(String),

    /// Forbidden (403)
    Forbidden(String),

    /// Not found (404)
    NotFound(String),

    /// Internal server error (500)
    Internal(String),

    /// Service unavailable (503)
    ServiceUnavailable(String),

    /// RUNE core error
    RuneError(rune_core::RUNEError),

    /// Serialization error
    SerializationError(serde_json::Error),
}

/// API result type
pub type ApiResult<T> = Result<T, ApiError>;

/// Error response body
#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            ApiError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            ApiError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            ApiError::NotFound(msg) => write!(f, "Not found: {}", msg),
            ApiError::Internal(msg) => write!(f, "Internal error: {}", msg),
            ApiError::ServiceUnavailable(msg) => write!(f, "Service unavailable: {}", msg),
            ApiError::RuneError(e) => write!(f, "RUNE error: {}", e),
            ApiError::SerializationError(e) => write!(f, "Serialization error: {}", e),
        }
    }
}

impl std::error::Error for ApiError {}

impl From<rune_core::RUNEError> for ApiError {
    fn from(err: rune_core::RUNEError) -> Self {
        ApiError::RuneError(err)
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        ApiError::SerializationError(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_type, message, details) = match self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "bad_request", msg, None),
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "unauthorized", msg, None),
            ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, "forbidden", msg, None),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, "not_found", msg, None),
            ApiError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                msg,
                None,
            ),
            ApiError::ServiceUnavailable(msg) => (
                StatusCode::SERVICE_UNAVAILABLE,
                "service_unavailable",
                msg,
                None,
            ),
            ApiError::RuneError(e) => {
                let msg = format!("Authorization engine error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "engine_error", msg, None)
            }
            ApiError::SerializationError(e) => {
                let msg = format!("Invalid JSON: {}", e);
                (StatusCode::BAD_REQUEST, "invalid_json", msg, None)
            }
        };

        let body = Json(ErrorResponse {
            error: error_type.to_string(),
            message,
            details,
        });

        (status, body).into_response()
    }
}
