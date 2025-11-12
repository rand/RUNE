//! Example demonstrating rich error diagnostics in RUNE
//!
//! This example shows how RUNE's diagnostic system provides helpful,
//! actionable error messages with source context and suggestions.
//!
//! Run with: cargo run --example error_diagnostics

use rune_core::datalog::diagnostics::{DatalogDiagnostics, DiagnosticBag, Span};
use rune_core::error::RUNEError;

fn main() {
    println!("=== RUNE Error Diagnostics Examples ===\n");

    // Example 1: Undefined variable error
    println!("--- Example 1: Undefined Variable ---");
    let source1 = "authorized(user) :- hasPermission(X).";
    let error1 = RUNEError::from_diagnostic(DatalogDiagnostics::undefined_variable(
        "X",
        Span::new(36, 37, 1, 37),
    ));
    println!("{}", error1.format_with_source(Some(source1)));

    // Example 2: Unsafe negation error
    println!("\n--- Example 2: Unsafe Negation ---");
    let source2 = "canAccess(user, resource) :- !isBlocked(user).";
    let error2 = RUNEError::from_diagnostic(DatalogDiagnostics::unsafe_negation(
        "user",
        Span::new(26, 30, 1, 27),
    ));
    println!("{}", error2.format_with_source(Some(source2)));

    // Example 3: Type mismatch error
    println!("\n--- Example 3: Type Mismatch ---");
    let source3 = "count_users(total) :- total = sum(User).";
    let error3 = RUNEError::from_diagnostic(DatalogDiagnostics::type_mismatch(
        "Integer",
        "Entity",
        Span::new(34, 38, 1, 35),
    ));
    println!("{}", error3.format_with_source(Some(source3)));

    // Example 4: Multiple errors (DiagnosticBag)
    println!("\n--- Example 4: Multiple Errors ---");
    let mut bag = DiagnosticBag::new();
    let source4 = "path(X, Y) :- edge(X, Z), edge(Z, Y).\npath(A) :- path(A, B).";

    bag.add(DatalogDiagnostics::undefined_variable(
        "X",
        Span::new(6, 7, 1, 7),
    ));
    bag.add(DatalogDiagnostics::undefined_variable(
        "Y",
        Span::new(9, 10, 1, 10),
    ));
    bag.add(DatalogDiagnostics::empty_rule_body(
        "path",
        Span::new(38, 45, 2, 1),
    ));

    let error4 = RUNEError::from_diagnostics(bag);
    println!("{}", error4.format_with_source(Some(source4)));

    // Example 5: Singleton variable warning
    println!("\n--- Example 5: Singleton Variable Warning ---");
    let source5 = "authorized(user) :- role(user, admin), department(Dept).";
    let error5 = RUNEError::from_diagnostic(DatalogDiagnostics::singleton_variable(
        "Dept",
        Span::new(51, 55, 1, 52),
    ));
    println!("{}", error5.format_with_source(Some(source5)));

    // Example 6: Stratification violation
    println!("\n--- Example 6: Stratification Violation ---");
    let error6 = RUNEError::from_diagnostic(DatalogDiagnostics::stratification_violation(
        "ancestor",
    ));
    println!("{}", error6);

    // Example 7: Unification failure
    println!("\n--- Example 7: Unification Failure ---");
    let source7 = "rule(X) :- atom1(X), atom2(5).";
    let error7 = RUNEError::from_diagnostic(DatalogDiagnostics::unification_failure(
        "Variable(X)",
        "Constant(5)",
        Span::new(20, 21, 1, 21),
    ));
    println!("{}", error7.format_with_source(Some(source7)));

    // Example 8: Aggregate without grouping (warning)
    println!("\n--- Example 8: Aggregate Without Grouping ---");
    let source8 = "total_count(count) :- count = count(*).";
    let error8 = RUNEError::from_diagnostic(DatalogDiagnostics::aggregate_without_grouping(
        "count",
        Span::new(30, 37, 1, 31),
    ));
    println!("{}", error8.format_with_source(Some(source8)));

    println!("\n=== End of Examples ===");
}
