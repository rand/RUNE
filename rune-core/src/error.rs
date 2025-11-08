//! Error types for RUNE

use thiserror::Error;

/// Main error type for RUNE operations
#[derive(Error, Debug)]
pub enum RUNEError {
    /// Parse error in configuration file
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Type checking failed
    #[error("Type error: {0}")]
    TypeError(String),

    /// Datalog evaluation error
    #[error("Datalog evaluation error: {0}")]
    DatalogError(String),

    /// Cedar policy error
    #[error("Cedar policy error: {0}")]
    CedarError(#[from] cedar_policy::PolicySetError),

    /// Authorization denied
    #[error("Authorization denied: {reason}")]
    AuthorizationDenied { reason: String },

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Invalid request
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Cache error
    #[error("Cache error: {0}")]
    CacheError(String),

    /// Timeout error
    #[error("Operation timed out after {0}ms")]
    Timeout(u64),
}

/// Result type alias for RUNE operations
pub type Result<T> = std::result::Result<T, RUNEError>;