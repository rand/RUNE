//! Error recovery and edge case integration tests
//!
//! Tests error handling for:
//! - Invalid policy handling
//! - Malformed fact insertion
//! - Cycle detection in rules
//! - Resource exhaustion
//! - Recovery mechanisms
//! - Graceful degradation

use rune_core::datalog::{DatalogEngine, DiagnosticBag, DiagnosticLevel};
use rune_core::facts::{Fact, FactStore};
use rune_core::parser::{parse_rules, ParseError};
use rune_core::policy::{PolicyError, PolicySet};
use rune_core::request::Request;
use rune_core::types::{Action, Principal, Resource, Value};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Test invalid Cedar policy handling
#[test]
fn test_invalid_cedar_policies() {
    let mut policy_set = PolicySet::new();

    // Test various invalid policies
    let invalid_policies = vec![
        (
            "missing_semicolon",
            r#"permit(principal, action, resource)"#,
            "Missing semicolon",
        ),
        (
            "invalid_entity_type",
            r#"permit(principal == InvalidType::"user", action, resource);"#,
            "Invalid entity type",
        ),
        (
            "malformed_condition",
            r#"permit(principal, action, resource) when { context.value == };"#,
            "Incomplete condition",
        ),
        (
            "circular_resource",
            r#"permit(principal, action, resource in resource);"#,
            "Circular reference",
        ),
        (
            "undefined_function",
            r#"permit(principal, action, resource) when { undefined_func() };"#,
            "Undefined function",
        ),
    ];

    for (name, policy, error_desc) in invalid_policies {
        let result = policy_set.load_policies(policy);
        assert!(
            result.is_err(),
            "{}: Should fail with '{}'",
            name,
            error_desc
        );

        // Verify error message is helpful
        if let Err(e) = result {
            let error_msg = format!("{}", e);
            assert!(
                !error_msg.is_empty(),
                "{}: Error message should not be empty",
                name
            );
            println!("  {}: {}", name, error_msg);
        }
    }

    // Verify policy set remains functional after errors
    let valid_policy = r#"
        permit(
            principal == User::"alice",
            action == Action::"read",
            resource == File::"test.txt"
        );
    "#;

    policy_set.load_policies(valid_policy)
        .expect("Should load valid policy after errors");

    let request = Request::new(
        Principal::user("alice"),
        Action::new("read"),
        Resource::file("test.txt"),
    );

    let result = policy_set.evaluate(&request).expect("Should evaluate");
    assert!(result.is_allowed(), "Valid policy should work after errors");
}

/// Test malformed Datalog rules
#[test]
fn test_malformed_datalog_rules() {
    // Test various syntax errors
    let malformed_rules = vec![
        (
            "missing_period",
            r#"fact(X, Y) :- other(X, Y)"#,
            "Missing period",
        ),
        (
            "invalid_variable",
            r#"fact(x, y) :- other(x, y)."#,
            "Lowercase variables",
        ),
        (
            "empty_body",
            r#"fact(X) :- ."#,
            "Empty rule body",
        ),
        (
            "missing_head",
            r#":- fact(X, Y)."#,
            "Missing rule head",
        ),
        (
            "unbound_variable",
            r#"fact(X, Z) :- other(X, Y)."#,
            "Unbound variable Z",
        ),
    ];

    for (name, rule_str, error_desc) in malformed_rules {
        let result = parse_rules(rule_str);
        assert!(
            result.is_err(),
            "{}: Should fail with '{}'",
            name,
            error_desc
        );

        // Check for helpful error messages
        if let Err(e) = result {
            let error_msg = format!("{}", e);
            println!("  {}: {}", name, error_msg);

            // Error should contain location info
            assert!(
                error_msg.contains("line") || error_msg.contains("column") ||
                error_msg.contains("position") || error_msg.contains("at"),
                "{}: Error should indicate location",
                name
            );
        }
    }
}

/// Test cycle detection in Datalog rules
#[test]
fn test_cycle_detection() {
    let fact_store = Arc::new(FactStore::new());

    // Direct cycle
    let direct_cycle = r#"
        a(X) :- b(X).
        b(X) :- a(X).
    "#;

    let rules = parse_rules(direct_cycle).expect("Should parse");
    let engine = DatalogEngine::new(rules, fact_store.clone());

    // Add initial fact to trigger cycle
    fact_store.add_fact(Fact::new("a", vec![Value::Integer(1)]));

    // Should detect cycle and handle gracefully
    let start = Instant::now();
    let result = engine.derive_facts();
    let duration = start.elapsed();

    // Should terminate quickly even with cycle
    assert!(
        duration.as_secs() < 1,
        "Should detect cycle and terminate quickly"
    );

    match result {
        Ok(facts) => {
            // Should reach fixed point
            assert!(facts.len() < 1000, "Should not infinite loop");
        }
        Err(e) => {
            // Or report cycle error
            println!("Cycle detected: {}", e);
        }
    }

    // Indirect cycle through negation (stratification violation)
    let negation_cycle = r#"
        p(X) :- q(X), !r(X).
        q(X) :- s(X).
        r(X) :- !p(X).
        s(1).
    "#;

    let result = parse_rules(negation_cycle);
    if let Ok(rules) = result {
        // Should detect stratification violation
        let engine = DatalogEngine::new(rules, Arc::new(FactStore::new()));
        let eval_result = engine.derive_facts();

        if let Err(e) = eval_result {
            println!("Stratification error: {}", e);
            assert!(
                format!("{}", e).contains("stratif") ||
                format!("{}", e).contains("negation"),
                "Should mention stratification issue"
            );
        }
    }
}

/// Test fact validation and type errors
#[test]
fn test_fact_validation() {
    let fact_store = Arc::new(FactStore::new());

    // Test type mismatches in facts
    fact_store.add_fact(Fact::new(
        "user_age",
        vec![Value::string("alice"), Value::Integer(25)],
    ));

    // Rules expecting different types
    let type_mismatch_rules = r#"
        adult(User) :- user_age(User, Age), Age > 18.
        name_length(User, Len) :- user_age(User, Name), string_length(Name, Len).
    "#;

    let rules = parse_rules(type_mismatch_rules).expect("Should parse");
    let engine = DatalogEngine::new(rules, fact_store.clone());

    let result = engine.derive_facts();

    // Should handle type errors gracefully
    match result {
        Ok(derived) => {
            // Adult rule should work (Age is integer)
            let adults: Vec<_> = derived
                .iter()
                .filter(|f| f.predicate == "adult")
                .collect();
            assert_eq!(adults.len(), 1, "Should derive adult fact");

            // name_length should fail or be empty (Age is not string)
            let lengths: Vec<_> = derived
                .iter()
                .filter(|f| f.predicate == "name_length")
                .collect();
            assert_eq!(lengths.len(), 0, "Should not derive with type mismatch");
        }
        Err(e) => {
            println!("Type error handled: {}", e);
        }
    }

    // Test invalid fact predicates
    let invalid_facts = vec![
        Fact::new("", vec![Value::Integer(1)]), // Empty predicate
        Fact::new(
            "very_long_predicate_name_that_exceeds_reasonable_limits_and_should_be_rejected",
            vec![Value::Integer(1)],
        ),
    ];

    for fact in invalid_facts {
        // Should handle invalid facts gracefully
        fact_store.add_fact(fact);
        // System should remain stable
    }

    assert!(fact_store.size() > 0, "Store should remain functional");
}

/// Test resource exhaustion scenarios
#[test]
fn test_resource_exhaustion() {
    // Test 1: Exponential rule expansion
    let exponential_rules = r#"
        // This could generate O(2^n) facts if not controlled
        pair(X, Y) :- num(X), num(Y).
        triple(X, Y, Z) :- num(X), num(Y), num(Z).
    "#;

    let fact_store = Arc::new(FactStore::new());

    // Add base facts that could cause explosion
    for i in 0..100 {
        fact_store.add_fact(Fact::new("num", vec![Value::Integer(i)]));
    }

    let rules = parse_rules(exponential_rules).expect("Should parse");
    let engine = DatalogEngine::new(rules, fact_store.clone());

    // Should handle resource limits
    let start = Instant::now();
    let result = engine.derive_facts();
    let duration = start.elapsed();

    match result {
        Ok(facts) => {
            println!("Generated {} facts in {:?}", facts.len(), duration);
            // Should have limits to prevent memory exhaustion
            assert!(
                facts.len() < 1_000_000,
                "Should limit fact generation"
            );
        }
        Err(e) => {
            println!("Resource limit hit: {}", e);
            assert!(
                format!("{}", e).contains("limit") ||
                format!("{}", e).contains("resource"),
                "Should mention resource limits"
            );
        }
    }

    // Test 2: Deep recursion
    let deep_recursion = r#"
        descendant(X, Y) :- parent(X, Y).
        descendant(X, Z) :- parent(X, Y), descendant(Y, Z).
    "#;

    let fact_store2 = Arc::new(FactStore::new());

    // Create deep parent chain
    for i in 0..10000 {
        fact_store2.add_fact(Fact::new(
            "parent",
            vec![Value::Integer(i), Value::Integer(i + 1)],
        ));
    }

    let rules = parse_rules(deep_recursion).expect("Should parse");
    let engine = DatalogEngine::new(rules, fact_store2);

    let start = Instant::now();
    let result = engine.derive_facts();
    let duration = start.elapsed();

    // Should complete without stack overflow
    assert!(
        duration.as_secs() < 30,
        "Should handle deep recursion efficiently"
    );

    if let Ok(facts) = result {
        println!("Deep recursion: {} facts in {:?}", facts.len(), duration);
    }
}

/// Test recovery from partial failures
#[test]
fn test_partial_failure_recovery() {
    let mut policy_set = PolicySet::new();

    // Load mix of valid and invalid policies
    let mixed_policies = r#"
        // Valid policy 1
        permit(
            principal == User::"alice",
            action == Action::"read",
            resource == File::"doc1.txt"
        );

        // Invalid policy (syntax error)
        permit(
            principal == User::"bob"
            action == Action::"write",  // Missing comma
            resource
        );

        // Valid policy 2
        permit(
            principal == User::"charlie",
            action == Action::"delete",
            resource == File::"doc2.txt"
        );
    "#;

    // Should load valid policies and skip invalid ones
    let result = policy_set.load_policies(mixed_policies);

    // Check if partial loading is supported
    if result.is_ok() {
        // Test that valid policies work
        let alice_request = Request::new(
            Principal::user("alice"),
            Action::new("read"),
            Resource::file("doc1.txt"),
        );

        let result = policy_set.evaluate(&alice_request).unwrap();
        assert!(result.is_allowed(), "Valid policy should work");

        let charlie_request = Request::new(
            Principal::user("charlie"),
            Action::new("delete"),
            Resource::file("doc2.txt"),
        );

        let result = policy_set.evaluate(&charlie_request).unwrap();
        assert!(result.is_allowed(), "Second valid policy should work");
    }
}

/// Test graceful degradation under errors
#[test]
fn test_graceful_degradation() {
    let fact_store = Arc::new(FactStore::new());

    // Setup initial valid state
    fact_store.add_fact(Fact::new(
        "user_role",
        vec![Value::string("alice"), Value::string("admin")],
    ));

    let rules = r#"
        can_access(User, "everything") :- user_role(User, "admin").
        can_access(User, Resource) :- owns(User, Resource).
    "#;

    let parsed_rules = parse_rules(rules).expect("Should parse");
    let engine = DatalogEngine::new(parsed_rules, fact_store.clone());

    // Initial evaluation should work
    let result1 = engine.derive_facts().expect("Should evaluate");
    let initial_count = result1.len();

    // Simulate corruption: add conflicting facts
    fact_store.add_fact(Fact::new(
        "user_role",
        vec![Value::string("alice"), Value::string("user")], // Conflicting role
    ));

    // System should still function
    let engine2 = DatalogEngine::new(
        parse_rules(rules).expect("Should parse"),
        fact_store.clone(),
    );
    let result2 = engine2.derive_facts().expect("Should still evaluate");

    println!(
        "Facts after conflict: {} (was {})",
        result2.len(),
        initial_count
    );

    // Should handle conflicts somehow (either both or latest)
    assert!(
        result2.len() > 0,
        "Should still produce results with conflicts"
    );
}

/// Test error diagnostics quality
#[test]
fn test_error_diagnostics() {
    let mut diagnostics = DiagnosticBag::new();

    // Test various error scenarios with diagnostics
    let bad_rule = r#"
        // Line 1
        fact(X, Y) :-
            pred1(X, Z),
            pred2(Z),  // Missing Y binding
            pred3(Y, W).  // W is unbound
    "#;

    // Parse and collect diagnostics
    match parse_rules(bad_rule) {
        Ok(_) => {
            // If parsing succeeds, validation should catch errors
        }
        Err(e) => {
            // Convert parse error to diagnostic
            diagnostics.add_diagnostic(
                DiagnosticLevel::Error,
                format!("Parse error: {}", e),
                Some((0, bad_rule.len())), // Span covering entire rule
                Some("Check rule syntax and variable bindings"),
            );
        }
    }

    // Check unsafe variables
    if bad_rule.contains("W") && !bad_rule.contains("W)") {
        diagnostics.add_diagnostic(
            DiagnosticLevel::Error,
            "Unsafe variable 'W' in rule head".to_string(),
            None,
            Some("Variable 'W' appears in head but is not bound in body"),
        );
    }

    // Verify diagnostics quality
    assert!(diagnostics.has_errors(), "Should detect errors");

    for diag in diagnostics.iter() {
        // Should have helpful messages
        assert!(!diag.message.is_empty(), "Error message should not be empty");

        // Should have suggestions when possible
        if diag.suggestion.is_some() {
            assert!(
                !diag.suggestion.as_ref().unwrap().is_empty(),
                "Suggestion should not be empty"
            );
        }

        println!("{}", diag);
    }
}

/// Test concurrent error scenarios
#[test]
fn test_concurrent_error_handling() {
    use std::sync::Mutex;
    use std::thread;

    let fact_store = Arc::new(FactStore::new());
    let errors = Arc::new(Mutex::new(Vec::new()));

    let mut handles = vec![];

    // Spawn threads that will cause various errors
    for thread_id in 0..4 {
        let fact_store = Arc::clone(&fact_store);
        let errors = Arc::clone(&errors);

        let handle = thread::spawn(move || {
            match thread_id {
                0 => {
                    // Thread 0: Add invalid facts
                    for i in 0..100 {
                        let fact = Fact::new(
                            "",  // Invalid empty predicate
                            vec![Value::Integer(i)],
                        );
                        fact_store.add_fact(fact);
                    }
                }
                1 => {
                    // Thread 1: Cause type errors
                    let rules = r#"
                        result(X) :- number(X), X > "string".
                    "#;

                    if let Ok(parsed) = parse_rules(rules) {
                        let engine = DatalogEngine::new(parsed, fact_store.clone());
                        if let Err(e) = engine.derive_facts() {
                            errors.lock().unwrap().push(format!("Thread 1: {}", e));
                        }
                    }
                }
                2 => {
                    // Thread 2: Resource exhaustion attempt
                    for i in 0..100000 {
                        fact_store.add_fact(Fact::new(
                            "spam",
                            vec![Value::Integer(i), Value::Integer(i * i)],
                        ));
                    }
                }
                3 => {
                    // Thread 3: Rapid add/remove causing races
                    for i in 0..1000 {
                        let fact = Fact::new("temp", vec![Value::Integer(i)]);
                        fact_store.add_fact(fact.clone());
                        fact_store.remove_fact(&fact);
                    }
                }
                _ => unreachable!(),
            }
        });

        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().expect("Thread should not panic");
    }

    // System should still be functional
    let test_fact = Fact::new("test", vec![Value::string("after_errors")]);
    fact_store.add_fact(test_fact.clone());

    let retrieved = fact_store.get_by_predicate("test");
    assert!(
        retrieved.iter().any(|f| f == &test_fact),
        "System should remain functional after concurrent errors"
    );

    // Check collected errors
    let collected_errors = errors.lock().unwrap();
    println!("Collected {} errors from threads", collected_errors.len());
}

/// Test policy conflict resolution
#[test]
fn test_policy_conflict_resolution() {
    let mut policy_set = PolicySet::new();

    // Load conflicting policies
    let conflicting_policies = r#"
        // Policy 1: Permit access
        @id("policy1")
        permit(
            principal == User::"alice",
            action == Action::"read",
            resource == File::"secret.txt"
        );

        // Policy 2: Forbid the same access
        @id("policy2")
        forbid(
            principal == User::"alice",
            action == Action::"read",
            resource == File::"secret.txt"
        );

        // Policy 3: Another permit with condition
        @id("policy3")
        permit(
            principal == User::"alice",
            action == Action::"read",
            resource == File::"secret.txt"
        ) when {
            context.time_of_day > 9 && context.time_of_day < 17
        };
    "#;

    policy_set.load_policies(conflicting_policies)
        .expect("Should load policies");

    // Test conflict resolution (forbid should win)
    let request = Request::new(
        Principal::user("alice"),
        Action::new("read"),
        Resource::file("secret.txt"),
    )
    .with_context("time_of_day", Value::Integer(12));

    let result = policy_set.evaluate(&request).expect("Should evaluate");

    // Cedar's deny-overrides semantics
    assert!(
        result.is_denied(),
        "Forbid should override permit in conflicts"
    );

    // Get decision reasons if available
    if let Some(reasons) = result.diagnostics() {
        println!("Decision reasons: {:?}", reasons);
        assert!(
            reasons.contains("forbid") || reasons.contains("deny"),
            "Should explain denial reason"
        );
    }
}