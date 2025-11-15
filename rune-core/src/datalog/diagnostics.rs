//! Error diagnostics for Datalog evaluation
//!
//! Provides rich, structured error messages with:
//! - Source location tracking (spans)
//! - Multiple error reporting
//! - Helpful suggestions for common mistakes
//! - Pretty formatting with code snippets
//!
//! Design principles:
//! - Collect multiple errors before failing (don't stop at first error)
//! - Provide actionable suggestions (not just "syntax error")
//! - Show relevant code context
//! - Use consistent formatting across all error types

use std::fmt;

/// Source location in input text
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    /// Starting byte offset
    pub start: usize,
    /// Ending byte offset (exclusive)
    pub end: usize,
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
}

impl Span {
    /// Create a new span
    pub fn new(start: usize, end: usize, line: usize, column: usize) -> Self {
        Span {
            start,
            end,
            line,
            column,
        }
    }

    /// Create a span for a single character
    pub fn single(offset: usize, line: usize, column: usize) -> Self {
        Span::new(offset, offset + 1, line, column)
    }

    /// Combine two spans into one that covers both
    pub fn merge(&self, other: &Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            line: self.line.min(other.line),
            column: self.column.min(other.column),
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// Severity level for diagnostics
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Informational message
    Info,
    /// Warning that doesn't prevent execution
    Warning,
    /// Error that prevents successful execution
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Info => write!(f, "info"),
            Severity::Warning => write!(f, "warning"),
            Severity::Error => write!(f, "error"),
        }
    }
}

/// A diagnostic message with context and suggestions
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Severity level
    pub severity: Severity,
    /// Primary error message
    pub message: String,
    /// Optional source location
    pub span: Option<Span>,
    /// Optional help text
    pub help: Option<String>,
    /// Suggested fixes
    pub suggestions: Vec<Suggestion>,
    /// Related diagnostics (e.g., "note: variable first used here")
    pub related: Vec<Diagnostic>,
}

impl Diagnostic {
    /// Create a new error diagnostic
    pub fn error(message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Error,
            message: message.into(),
            span: None,
            help: None,
            suggestions: Vec::new(),
            related: Vec::new(),
        }
    }

    /// Create a new warning diagnostic
    pub fn warning(message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Warning,
            message: message.into(),
            span: None,
            help: None,
            suggestions: Vec::new(),
            related: Vec::new(),
        }
    }

    /// Create a new info diagnostic
    pub fn info(message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Info,
            message: message.into(),
            span: None,
            help: None,
            suggestions: Vec::new(),
            related: Vec::new(),
        }
    }

    /// Add a source span
    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    /// Add help text
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Add a suggestion
    pub fn with_suggestion(mut self, suggestion: Suggestion) -> Self {
        self.suggestions.push(suggestion);
        self
    }

    /// Add a related diagnostic
    pub fn with_related(mut self, related: Diagnostic) -> Self {
        self.related.push(related);
        self
    }

    /// Format the diagnostic with optional source code
    pub fn format(&self, source: Option<&str>) -> String {
        let mut output = String::new();

        // Header: severity and message
        let severity_color = match self.severity {
            Severity::Error => "\x1b[1;31m",   // Bold red
            Severity::Warning => "\x1b[1;33m", // Bold yellow
            Severity::Info => "\x1b[1;36m",    // Bold cyan
        };
        let reset = "\x1b[0m";

        output.push_str(&format!(
            "{}{}{}: {}\n",
            severity_color, self.severity, reset, self.message
        ));

        // Location
        if let Some(ref span) = self.span {
            output.push_str(&format!("  --> {}\n", span));

            // Source code context
            if let Some(src) = source {
                if let Some(context) = extract_source_context(src, span) {
                    output.push_str(&format!("   |\n{}\n", context));
                }
            }
        }

        // Help text
        if let Some(ref help) = self.help {
            output.push_str(&format!(
                "\x1b[1m{} = help:\x1b[0m {}\n",
                severity_color, help
            ));
        }

        // Suggestions
        for suggestion in &self.suggestions {
            output.push_str(&format!(
                "\x1b[1;32msuggestion:\x1b[0m {}\n",
                suggestion.message
            ));
            if let Some(ref replacement) = suggestion.replacement {
                output.push_str(&format!("  replace with: {}\n", replacement));
            }
        }

        // Related diagnostics
        for related in &self.related {
            output.push_str(&format!("\n{}", related.format(source)));
        }

        output
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format(None))
    }
}

/// A suggested fix for a diagnostic
#[derive(Debug, Clone)]
pub struct Suggestion {
    /// Description of the suggestion
    pub message: String,
    /// Optional replacement text
    pub replacement: Option<String>,
    /// Span to replace (if different from diagnostic span)
    pub span: Option<Span>,
}

impl Suggestion {
    /// Create a new suggestion
    pub fn new(message: impl Into<String>) -> Self {
        Suggestion {
            message: message.into(),
            replacement: None,
            span: None,
        }
    }

    /// Add replacement text
    pub fn with_replacement(mut self, replacement: impl Into<String>) -> Self {
        self.replacement = Some(replacement.into());
        self
    }

    /// Add a span
    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }
}

/// Extract source code context around a span
fn extract_source_context(source: &str, span: &Span) -> Option<String> {
    let lines: Vec<&str> = source.lines().collect();
    if span.line == 0 || span.line > lines.len() {
        return None;
    }

    let line_idx = span.line - 1;
    let line = lines[line_idx];

    let mut output = String::new();

    // Line number and content
    output.push_str(&format!("{:>4} | {}\n", span.line, line));

    // Underline the error span
    let col_start = span.column.saturating_sub(1);
    let col_end = col_start + (span.end - span.start).min(line.len());
    let spaces = " ".repeat(col_start);
    let underline = "^".repeat((col_end - col_start).max(1));

    output.push_str(&format!("     | {}{}\n", spaces, underline));

    Some(output)
}

/// Collection of diagnostics
#[derive(Debug, Clone, Default)]
pub struct DiagnosticBag {
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticBag {
    /// Create a new empty diagnostic bag
    pub fn new() -> Self {
        DiagnosticBag {
            diagnostics: Vec::new(),
        }
    }

    /// Add a diagnostic
    pub fn add(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Add an error
    pub fn error(&mut self, message: impl Into<String>) {
        self.add(Diagnostic::error(message));
    }

    /// Add a warning
    pub fn warning(&mut self, message: impl Into<String>) {
        self.add(Diagnostic::warning(message));
    }

    /// Add an info message
    pub fn info(&mut self, message: impl Into<String>) {
        self.add(Diagnostic::info(message));
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }

    /// Check if there are any warnings
    pub fn has_warnings(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Warning)
    }

    /// Get all diagnostics
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Count errors
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count()
    }

    /// Count warnings
    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count()
    }

    /// Format all diagnostics with source code
    pub fn format(&self, source: Option<&str>) -> String {
        let mut output = String::new();

        for diagnostic in &self.diagnostics {
            output.push_str(&diagnostic.format(source));
            output.push('\n');
        }

        // Summary
        let errors = self.error_count();
        let warnings = self.warning_count();

        if errors > 0 || warnings > 0 {
            output.push_str(&format!(
                "\x1b[1mSummary:\x1b[0m {} error(s), {} warning(s)\n",
                errors, warnings
            ));
        }

        output
    }

    /// Clear all diagnostics
    pub fn clear(&mut self) {
        self.diagnostics.clear();
    }
}

impl fmt::Display for DiagnosticBag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format(None))
    }
}

/// Common diagnostic builders for Datalog errors
pub struct DatalogDiagnostics;

impl DatalogDiagnostics {
    /// Undefined variable error
    pub fn undefined_variable(var: &str, span: Span) -> Diagnostic {
        Diagnostic::error(format!("undefined variable '{}'", var))
            .with_span(span)
            .with_help("variables must appear in at least one positive (non-negated) body atom")
            .with_suggestion(Suggestion::new(
                "add a positive atom that binds this variable",
            ))
    }

    /// Unsafe negation error
    pub fn unsafe_negation(var: &str, span: Span) -> Diagnostic {
        Diagnostic::error(format!(
            "variable '{}' appears only in negated atoms (unsafe)",
            var
        ))
        .with_span(span)
        .with_help("variables in negated atoms must also appear in positive atoms (stratification requirement)")
        .with_suggestion(Suggestion::new(
            "add a positive atom that binds this variable before the negated atom",
        ))
    }

    /// Infinite loop detected
    pub fn infinite_loop(rule_name: &str) -> Diagnostic {
        Diagnostic::error(format!(
            "infinite loop detected in rule evaluation: '{}'",
            rule_name
        ))
        .with_help("rule produces facts that trigger itself indefinitely")
        .with_suggestion(Suggestion::new(
            "add a termination condition or restructure the rule",
        ))
    }

    /// Type mismatch error
    pub fn type_mismatch(expected: &str, got: &str, span: Span) -> Diagnostic {
        Diagnostic::error(format!("type mismatch: expected {}, got {}", expected, got))
            .with_span(span)
            .with_help("all uses of a variable must have compatible types")
    }

    /// Stratification violation
    pub fn stratification_violation(predicate: &str) -> Diagnostic {
        Diagnostic::error(format!(
            "stratification violation: predicate '{}' depends on itself through negation",
            predicate
        ))
        .with_help("predicates cannot recursively depend on their own negation")
        .with_suggestion(Suggestion::new(
            "restructure rules to eliminate cycles through negation",
        ))
    }

    /// Parse error with context
    pub fn parse_error(message: impl Into<String>, span: Span) -> Diagnostic {
        Diagnostic::error(message).with_span(span)
    }

    /// Unification failure
    pub fn unification_failure(term1: &str, term2: &str, span: Span) -> Diagnostic {
        Diagnostic::error(format!("cannot unify '{}' with '{}'", term1, term2))
            .with_span(span)
            .with_help("terms must have compatible structure and types to unify")
    }

    /// Aggregate without grouping
    pub fn aggregate_without_grouping(op: &str, span: Span) -> Diagnostic {
        Diagnostic::warning(format!(
            "aggregate operation '{}' without explicit grouping",
            op
        ))
        .with_span(span)
        .with_help("aggregate will compute a single value over all matching facts")
        .with_suggestion(Suggestion::new(
            "add grouping variables if you want per-group aggregates",
        ))
    }

    /// Empty rule body
    pub fn empty_rule_body(head_predicate: &str, span: Span) -> Diagnostic {
        Diagnostic::error(format!("rule for '{}' has empty body", head_predicate))
            .with_span(span)
            .with_help("rules must have at least one body atom")
    }

    /// Singleton variable warning
    pub fn singleton_variable(var: &str, span: Span) -> Diagnostic {
        Diagnostic::warning(format!("variable '{}' appears only once", var))
            .with_span(span)
            .with_help("variables that appear only once might indicate a mistake")
            .with_suggestion(
                Suggestion::new("use '_' for intentionally unused variables")
                    .with_replacement(format!("_{}", var)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_creation() {
        let span = Span::new(0, 10, 1, 5);
        assert_eq!(span.start, 0);
        assert_eq!(span.end, 10);
        assert_eq!(span.line, 1);
        assert_eq!(span.column, 5);
        assert_eq!(span.to_string(), "1:5");
    }

    #[test]
    fn test_span_merge() {
        let span1 = Span::new(0, 5, 1, 1);
        let span2 = Span::new(10, 15, 1, 11);
        let merged = span1.merge(&span2);
        assert_eq!(merged.start, 0);
        assert_eq!(merged.end, 15);
    }

    #[test]
    fn test_diagnostic_creation() {
        let diag = Diagnostic::error("test error")
            .with_span(Span::new(0, 5, 1, 1))
            .with_help("this is help text");

        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.message, "test error");
        assert!(diag.span.is_some());
        assert_eq!(diag.help.unwrap(), "this is help text");
    }

    #[test]
    fn test_diagnostic_bag() {
        let mut bag = DiagnosticBag::new();
        bag.error("error 1");
        bag.warning("warning 1");
        bag.error("error 2");

        assert!(bag.has_errors());
        assert!(bag.has_warnings());
        assert_eq!(bag.error_count(), 2);
        assert_eq!(bag.warning_count(), 1);
    }

    #[test]
    fn test_undefined_variable_diagnostic() {
        let diag = DatalogDiagnostics::undefined_variable("X", Span::new(5, 6, 1, 6));

        assert_eq!(diag.severity, Severity::Error);
        assert!(diag.message.contains("undefined variable"));
        assert!(diag.help.is_some());
        assert!(!diag.suggestions.is_empty());
    }

    #[test]
    fn test_unsafe_negation_diagnostic() {
        let diag = DatalogDiagnostics::unsafe_negation("Y", Span::new(10, 11, 2, 5));

        assert_eq!(diag.severity, Severity::Error);
        assert!(diag.message.contains("unsafe"));
        assert!(diag.help.unwrap().contains("stratification"));
    }

    #[test]
    fn test_source_context_extraction() {
        let source = "path(X, Y) :- edge(X, Y).\npath(X, Z) :- edge(X, Y), path(Y, Z).";
        let span = Span::new(15, 20, 1, 16);

        let context = extract_source_context(source, &span);
        assert!(context.is_some());
        let ctx = context.unwrap();
        assert!(ctx.contains("path(X, Y)"));
        assert!(ctx.contains("^"));
    }

    #[test]
    fn test_diagnostic_formatting() {
        let source = "test(X) :- unknown(X).";
        let span = Span::new(7, 8, 1, 8);

        let diag = DatalogDiagnostics::undefined_variable("X", span);
        let formatted = diag.format(Some(source));

        assert!(formatted.contains("error"));
        assert!(formatted.contains("undefined variable"));
        assert!(formatted.contains("1:8"));
    }

    #[test]
    fn test_suggestion_with_replacement() {
        let suggestion =
            Suggestion::new("use underscore for unused variables").with_replacement("_var");

        assert_eq!(suggestion.message, "use underscore for unused variables");
        assert_eq!(suggestion.replacement.unwrap(), "_var");
    }

    #[test]
    fn test_multiple_diagnostics_formatting() {
        let mut bag = DiagnosticBag::new();

        bag.add(Diagnostic::error("first error").with_span(Span::new(0, 5, 1, 1)));

        bag.add(Diagnostic::warning("first warning").with_span(Span::new(10, 15, 2, 1)));

        let formatted = bag.format(None);
        assert!(formatted.contains("first error"));
        assert!(formatted.contains("first warning"));
        assert!(formatted.contains("Summary:"));
        assert!(formatted.contains("1 error(s)"));
        assert!(formatted.contains("1 warning(s)"));
    }

    #[test]
    fn test_stratification_violation() {
        let diag = DatalogDiagnostics::stratification_violation("ancestor");

        assert_eq!(diag.severity, Severity::Error);
        assert!(diag.message.contains("stratification violation"));
        assert!(diag.help.is_some());
    }

    #[test]
    fn test_singleton_variable_warning() {
        let diag = DatalogDiagnostics::singleton_variable("Z", Span::new(20, 21, 3, 10));

        assert_eq!(diag.severity, Severity::Warning);
        assert!(diag.message.contains("appears only once"));
        assert!(!diag.suggestions.is_empty());
        assert_eq!(diag.suggestions[0].replacement.as_ref().unwrap(), "_Z");
    }

    #[test]
    fn test_related_diagnostics() {
        let main_diag = Diagnostic::error("main error").with_related(
            Diagnostic::info("note: this is related information").with_span(Span::new(5, 10, 1, 6)),
        );

        assert_eq!(main_diag.related.len(), 1);
        assert_eq!(main_diag.related[0].severity, Severity::Info);
    }
}
