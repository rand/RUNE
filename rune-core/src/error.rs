//! Error types for RUNE
//!
//! Provides both simple string-based errors (for backward compatibility)
//! and rich diagnostic errors (for detailed error reporting with source context).

use crate::datalog::diagnostics::{Diagnostic, DiagnosticBag};
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
    CedarError(#[from] Box<cedar_policy::PolicySetError>),

    /// Authorization denied
    #[error("Authorization denied: {reason}")]
    AuthorizationDenied {
        /// Reason for authorization denial
        reason: String,
    },

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

    /// Rich diagnostic error with multiple messages and suggestions
    #[error("{}", .0.format(None))]
    DiagnosticError(DiagnosticBag),
}

impl RUNEError {
    /// Create a diagnostic error from a single diagnostic
    pub fn from_diagnostic(diagnostic: Diagnostic) -> Self {
        let mut bag = DiagnosticBag::new();
        bag.add(diagnostic);
        RUNEError::DiagnosticError(bag)
    }

    /// Create a diagnostic error from multiple diagnostics
    pub fn from_diagnostics(diagnostics: DiagnosticBag) -> Self {
        RUNEError::DiagnosticError(diagnostics)
    }

    /// Get diagnostics if this is a diagnostic error
    pub fn diagnostics(&self) -> Option<&DiagnosticBag> {
        match self {
            RUNEError::DiagnosticError(bag) => Some(bag),
            _ => None,
        }
    }

    /// Check if this error has diagnostics
    pub fn has_diagnostics(&self) -> bool {
        matches!(self, RUNEError::DiagnosticError(_))
    }

    /// Format the error with optional source code context
    pub fn format_with_source(&self, source: Option<&str>) -> String {
        match self {
            RUNEError::DiagnosticError(bag) => bag.format(source),
            _ => self.to_string(),
        }
    }
}

/// Result type alias for RUNE operations
pub type Result<T> = std::result::Result<T, RUNEError>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datalog::diagnostics::{DatalogDiagnostics, Span};

    #[test]
    fn test_diagnostic_error_creation() {
        let diagnostic = DatalogDiagnostics::undefined_variable("X", Span::new(0, 1, 1, 1));
        let error = RUNEError::from_diagnostic(diagnostic);

        assert!(error.has_diagnostics());
        assert!(error.diagnostics().is_some());
    }

    #[test]
    fn test_diagnostic_error_multiple() {
        let mut bag = DiagnosticBag::new();
        bag.add(DatalogDiagnostics::undefined_variable(
            "X",
            Span::new(0, 1, 1, 1),
        ));
        bag.add(DatalogDiagnostics::undefined_variable(
            "Y",
            Span::new(5, 6, 1, 6),
        ));

        let error = RUNEError::from_diagnostics(bag);
        assert!(error.has_diagnostics());

        let diagnostics = error.diagnostics().unwrap();
        assert_eq!(diagnostics.error_count(), 2);
    }

    #[test]
    fn test_non_diagnostic_error() {
        let error = RUNEError::ConfigError("test error".to_string());
        assert!(!error.has_diagnostics());
        assert!(error.diagnostics().is_none());
    }

    #[test]
    fn test_error_formatting() {
        let diagnostic = DatalogDiagnostics::undefined_variable("X", Span::new(0, 1, 1, 1));
        let error = RUNEError::from_diagnostic(diagnostic);

        let formatted = error.to_string();
        assert!(formatted.contains("undefined variable"));
    }

    #[test]
    fn test_error_formatting_with_source() {
        let source = "rule(X) :- undefined(X).";
        let diagnostic = DatalogDiagnostics::undefined_variable("X", Span::new(5, 6, 1, 6));
        let error = RUNEError::from_diagnostic(diagnostic);

        let formatted = error.format_with_source(Some(source));
        assert!(formatted.contains("undefined variable"));
        assert!(formatted.contains("1:6"));
    }
}
