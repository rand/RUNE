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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[test]
    fn test_api_error_display() {
        let err = ApiError::BadRequest("Invalid input".to_string());
        assert_eq!(format!("{}", err), "Bad request: Invalid input");

        let err = ApiError::Unauthorized("Token expired".to_string());
        assert_eq!(format!("{}", err), "Unauthorized: Token expired");

        let err = ApiError::Forbidden("Access denied".to_string());
        assert_eq!(format!("{}", err), "Forbidden: Access denied");

        let err = ApiError::NotFound("Resource not found".to_string());
        assert_eq!(format!("{}", err), "Not found: Resource not found");

        let err = ApiError::Internal("Database error".to_string());
        assert_eq!(format!("{}", err), "Internal error: Database error");

        let err = ApiError::ServiceUnavailable("Service down".to_string());
        assert_eq!(format!("{}", err), "Service unavailable: Service down");
    }

    #[test]
    fn test_api_error_from_rune_error() {
        let rune_err = rune_core::RUNEError::ParseError("Invalid syntax".to_string());
        let api_err: ApiError = rune_err.into();
        assert!(matches!(api_err, ApiError::RuneError(_)));
        assert!(format!("{}", api_err).contains("RUNE error"));
    }

    #[test]
    fn test_api_error_from_serde_error() {
        // Create a serde error by trying to parse invalid JSON
        let serde_err = serde_json::from_str::<String>("not json").unwrap_err();
        let api_err: ApiError = serde_err.into();
        assert!(matches!(api_err, ApiError::SerializationError(_)));
        assert!(format!("{}", api_err).contains("Serialization error"));
    }

    #[test]
    fn test_api_error_display_with_rune_error() {
        let rune_err = rune_core::RUNEError::TypeError("Type mismatch".to_string());
        let api_err = ApiError::RuneError(rune_err);
        let display = format!("{}", api_err);
        assert!(display.contains("RUNE error"));
        assert!(display.contains("Type error"));
    }

    #[test]
    fn test_api_error_display_with_serialization_error() {
        let serde_err = serde_json::from_str::<String>("{invalid}").unwrap_err();
        let api_err = ApiError::SerializationError(serde_err);
        let display = format!("{}", api_err);
        assert!(display.contains("Serialization error"));
    }

    #[test]
    fn test_error_trait_implementation() {
        let err = ApiError::BadRequest("test".to_string());
        // Test that it implements std::error::Error
        let _error: &dyn std::error::Error = &err;
    }

    #[tokio::test]
    async fn test_api_error_into_response_bad_request() {
        let err = ApiError::BadRequest("Invalid parameter".to_string());
        let response = err.into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = response.into_body();
        let bytes = to_bytes(body, usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(json["error"], "bad_request");
        assert_eq!(json["message"], "Invalid parameter");
    }

    #[tokio::test]
    async fn test_api_error_into_response_unauthorized() {
        let err = ApiError::Unauthorized("Invalid token".to_string());
        let response = err.into_response();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let body = response.into_body();
        let bytes = to_bytes(body, usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(json["error"], "unauthorized");
        assert_eq!(json["message"], "Invalid token");
    }

    #[tokio::test]
    async fn test_api_error_into_response_forbidden() {
        let err = ApiError::Forbidden("Insufficient permissions".to_string());
        let response = err.into_response();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let body = response.into_body();
        let bytes = to_bytes(body, usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(json["error"], "forbidden");
        assert_eq!(json["message"], "Insufficient permissions");
    }

    #[tokio::test]
    async fn test_api_error_into_response_not_found() {
        let err = ApiError::NotFound("User not found".to_string());
        let response = err.into_response();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body = response.into_body();
        let bytes = to_bytes(body, usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(json["error"], "not_found");
        assert_eq!(json["message"], "User not found");
    }

    #[tokio::test]
    async fn test_api_error_into_response_internal() {
        let err = ApiError::Internal("Database connection failed".to_string());
        let response = err.into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = response.into_body();
        let bytes = to_bytes(body, usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(json["error"], "internal_error");
        assert_eq!(json["message"], "Database connection failed");
    }

    #[tokio::test]
    async fn test_api_error_into_response_service_unavailable() {
        let err = ApiError::ServiceUnavailable("Service maintenance".to_string());
        let response = err.into_response();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

        let body = response.into_body();
        let bytes = to_bytes(body, usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(json["error"], "service_unavailable");
        assert_eq!(json["message"], "Service maintenance");
    }

    #[tokio::test]
    async fn test_api_error_into_response_rune_error() {
        let rune_err = rune_core::RUNEError::ParseError("Syntax error".to_string());
        let err = ApiError::RuneError(rune_err);
        let response = err.into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = response.into_body();
        let bytes = to_bytes(body, usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(json["error"], "engine_error");
        assert!(json["message"].as_str().unwrap().contains("Authorization engine error"));
    }

    #[tokio::test]
    async fn test_api_error_into_response_serialization_error() {
        let serde_err = serde_json::from_str::<String>("{bad json}").unwrap_err();
        let err = ApiError::SerializationError(serde_err);
        let response = err.into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = response.into_body();
        let bytes = to_bytes(body, usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(json["error"], "invalid_json");
        assert!(json["message"].as_str().unwrap().contains("Invalid JSON"));
    }

    #[test]
    fn test_error_response_serialization() {
        let response = ErrorResponse {
            error: "test_error".to_string(),
            message: "Test message".to_string(),
            details: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test_error"));
        assert!(json.contains("Test message"));
        assert!(!json.contains("details")); // Should skip serializing None

        let response_with_details = ErrorResponse {
            error: "test_error".to_string(),
            message: "Test message".to_string(),
            details: Some("Additional details".to_string()),
        };

        let json = serde_json::to_string(&response_with_details).unwrap();
        assert!(json.contains("details"));
        assert!(json.contains("Additional details"));
    }

    #[test]
    fn test_api_result_type() {
        // Test that ApiResult type alias works correctly
        let success: ApiResult<String> = Ok("Success".to_string());
        assert!(success.is_ok());

        let error: ApiResult<String> = Err(ApiError::BadRequest("Failed".to_string()));
        assert!(error.is_err());
    }
}
