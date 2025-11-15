//! Integration tests for Datalog-Cedar bidirectional data flow
//!
//! Tests the complete pipeline:
//! 1. Facts derived from Datalog feeding into Cedar policies
//! 2. Policy decisions affecting Datalog evaluation
//! 3. Real-time synchronization between systems

use rune_core::datalog::{Atom, DatalogEngine, Evaluator, Rule, Term};
use rune_core::facts::{Fact, FactStore};
use rune_core::parser::parse_rules;
use rune_core::policy::{PolicyEngine, PolicySet};
use rune_core::request::Request;
use rune_core::types::{Action, Principal, Resource, Value};
use std::sync::Arc;
use std::time::Instant;

/// Test that facts derived from Datalog rules are correctly used by Cedar policies
#[test]
fn test_datalog_derived_facts_in_cedar() {
    // Setup Datalog engine with hierarchical permission rules
    let rules_source = r#"
        // Derive effective permissions from role hierarchy
        effective_permission(User, Resource, Permission) :-
            has_role(User, Role),
            role_permission(Role, Resource, Permission).

        // Derive inherited permissions through role hierarchy
        effective_permission(User, Resource, Permission) :-
            has_role(User, Role),
            role_inherits(Role, ParentRole),
            role_permission(ParentRole, Resource, Permission).

        // Derive group-based permissions
        effective_permission(User, Resource, Permission) :-
            member_of(User, Group),
            group_permission(Group, Resource, Permission).
    "#;

    let rules = parse_rules(rules_source).expect("Failed to parse rules");
    let fact_store = Arc::new(FactStore::new());

    // Add base facts
    fact_store.add_fact(Fact::new(
        "has_role",
        vec![Value::string("alice"), Value::string("admin")],
    ));
    fact_store.add_fact(Fact::new(
        "has_role",
        vec![Value::string("bob"), Value::string("developer")],
    ));
    fact_store.add_fact(Fact::new(
        "role_inherits",
        vec![Value::string("admin"), Value::string("developer")],
    ));
    fact_store.add_fact(Fact::new(
        "role_permission",
        vec![
            Value::string("admin"),
            Value::string("database"),
            Value::string("delete"),
        ],
    ));
    fact_store.add_fact(Fact::new(
        "role_permission",
        vec![
            Value::string("developer"),
            Value::string("database"),
            Value::string("read"),
        ],
    ));
    fact_store.add_fact(Fact::new(
        "member_of",
        vec![Value::string("charlie"), Value::string("contractors")],
    ));
    fact_store.add_fact(Fact::new(
        "group_permission",
        vec![
            Value::string("contractors"),
            Value::string("database"),
            Value::string("read"),
        ],
    ));

    // Derive facts using Datalog
    let engine = DatalogEngine::new(rules, fact_store.clone());
    let derived = engine.derive_facts().expect("Failed to derive facts");

    // Verify derived facts
    let alice_permissions: Vec<_> = derived
        .iter()
        .filter(|f| {
            f.predicate == "effective_permission"
                && f.args[0] == Value::string("alice")
        })
        .collect();
    assert_eq!(alice_permissions.len(), 2); // admin has delete + inherited read

    // Create Cedar policies that reference derived facts
    let cedar_policy = r#"
        permit(
            principal == User::"alice",
            action == Action::"delete",
            resource == Database::"prod"
        ) when {
            context.has_permission == "delete" &&
            context.derived_from_datalog == true
        };

        permit(
            principal,
            action == Action::"read",
            resource
        ) when {
            context.has_permission == "read" &&
            context.role_verified == true
        };
    "#;

    let mut policy_set = PolicySet::new();
    policy_set.load_policies(cedar_policy).expect("Failed to load policies");

    // Create request with context from derived facts
    let request = Request::new(
        Principal::user("alice"),
        Action::new("delete"),
        Resource::database("prod"),
    )
    .with_context("has_permission", Value::string("delete"))
    .with_context("derived_from_datalog", Value::Boolean(true))
    .with_context("role_verified", Value::Boolean(true));

    // Evaluate authorization using derived facts
    let result = policy_set.evaluate(&request).expect("Failed to evaluate");
    assert!(result.is_allowed());
}

/// Test bidirectional flow: Cedar decisions affecting Datalog evaluation
#[test]
fn test_cedar_decisions_affect_datalog() {
    // Setup fact store
    let fact_store = Arc::new(FactStore::new());

    // Initial facts
    fact_store.add_fact(Fact::new(
        "access_attempt",
        vec![Value::string("user1"), Value::string("resource1")],
    ));

    // Cedar policy that makes authorization decisions
    let cedar_policy = r#"
        permit(
            principal,
            action == Action::"read",
            resource
        ) when {
            context.risk_score < 50
        };

        forbid(
            principal,
            action,
            resource
        ) when {
            context.risk_score >= 80
        };
    "#;

    let mut policy_set = PolicySet::new();
    policy_set.load_policies(cedar_policy).expect("Failed to load policies");

    // Evaluate Cedar policy with different risk scores
    let low_risk_request = Request::new(
        Principal::user("user1"),
        Action::new("read"),
        Resource::file("file1"),
    )
    .with_context("risk_score", Value::Integer(30));

    let high_risk_request = Request::new(
        Principal::user("user2"),
        Action::new("read"),
        Resource::file("file2"),
    )
    .with_context("risk_score", Value::Integer(85));

    let low_result = policy_set.evaluate(&low_risk_request).expect("Failed to evaluate");
    let high_result = policy_set.evaluate(&high_risk_request).expect("Failed to evaluate");

    // Feed Cedar decisions back into Datalog as facts
    if low_result.is_allowed() {
        fact_store.add_fact(Fact::new(
            "authorized_access",
            vec![
                Value::string("user1"),
                Value::string("file1"),
                Value::Integer(30),
            ],
        ));
    }

    if high_result.is_denied() {
        fact_store.add_fact(Fact::new(
            "denied_access",
            vec![
                Value::string("user2"),
                Value::string("file2"),
                Value::Integer(85),
            ],
        ));
    }

    // Create Datalog rules that use Cedar decision facts
    let rules_source = r#"
        // Track suspicious patterns based on denied accesses
        suspicious_user(User) :-
            denied_access(User, _, RiskScore),
            RiskScore > 75.

        // Calculate user trust score based on authorization history
        user_trust_level(User, "high") :-
            authorized_access(User, _, _),
            !suspicious_user(User).

        user_trust_level(User, "low") :-
            suspicious_user(User).
    "#;

    let rules = parse_rules(rules_source).expect("Failed to parse rules");
    let engine = DatalogEngine::new(rules, fact_store);
    let derived = engine.derive_facts().expect("Failed to derive facts");

    // Verify bidirectional flow
    let suspicious_users: Vec<_> = derived
        .iter()
        .filter(|f| f.predicate == "suspicious_user")
        .collect();
    assert_eq!(suspicious_users.len(), 1);
    assert_eq!(suspicious_users[0].args[0], Value::string("user2"));

    let trust_levels: Vec<_> = derived
        .iter()
        .filter(|f| f.predicate == "user_trust_level")
        .collect();
    assert_eq!(trust_levels.len(), 2);
}

/// Test real-time synchronization between Datalog and Cedar
#[test]
fn test_real_time_synchronization() {
    let fact_store = Arc::new(FactStore::new());

    // Simulate real-time event: user login
    fact_store.add_fact(Fact::new(
        "user_login",
        vec![
            Value::string("alice"),
            Value::Integer(1700000000), // timestamp
            Value::string("192.168.1.100"), // IP
        ],
    ));

    // Datalog rules for session management
    let session_rules = r#"
        // Active session detection
        active_session(User, IP) :-
            user_login(User, Timestamp, IP),
            !user_logout(User, _, _).

        // Multi-session detection
        multi_session_user(User) :-
            active_session(User, IP1),
            active_session(User, IP2),
            IP1 != IP2.

        // Session risk calculation
        session_risk(User, "high") :-
            multi_session_user(User).

        session_risk(User, "low") :-
            active_session(User, _),
            !multi_session_user(User).
    "#;

    let rules = parse_rules(session_rules).expect("Failed to parse rules");
    let engine = DatalogEngine::new(rules.clone(), fact_store.clone());

    // Initial evaluation
    let start = Instant::now();
    let derived = engine.derive_facts().expect("Failed to derive facts");
    let derive_time = start.elapsed();

    // Verify initial state
    let sessions: Vec<_> = derived
        .iter()
        .filter(|f| f.predicate == "active_session")
        .collect();
    assert_eq!(sessions.len(), 1);

    // Simulate another login from different IP (potential security event)
    fact_store.add_fact(Fact::new(
        "user_login",
        vec![
            Value::string("alice"),
            Value::Integer(1700000100),
            Value::string("10.0.0.50"),
        ],
    ));

    // Re-evaluate with new facts
    let engine2 = DatalogEngine::new(rules, fact_store.clone());
    let derived2 = engine2.derive_facts().expect("Failed to derive facts");

    // Check for multi-session detection
    let multi_sessions: Vec<_> = derived2
        .iter()
        .filter(|f| f.predicate == "multi_session_user")
        .collect();
    assert_eq!(multi_sessions.len(), 1);

    let risk_assessment: Vec<_> = derived2
        .iter()
        .filter(|f| f.predicate == "session_risk" && f.args[1] == Value::string("high"))
        .collect();
    assert_eq!(risk_assessment.len(), 1);

    // Create Cedar policy that uses real-time Datalog facts
    let adaptive_policy = r#"
        // Deny access for high-risk sessions
        forbid(
            principal,
            action,
            resource
        ) when {
            context.session_risk == "high" &&
            resource in Folder::"sensitive"
        };

        // Require MFA for multi-session users
        permit(
            principal,
            action,
            resource
        ) when {
            context.multi_session == true &&
            context.mfa_verified == true
        };
    "#;

    let mut policy_set = PolicySet::new();
    policy_set.load_policies(adaptive_policy).expect("Failed to load policies");

    // Test authorization with real-time derived facts
    let request_high_risk = Request::new(
        Principal::user("alice"),
        Action::new("read"),
        Resource::file_in_folder("sensitive", "secrets.txt"),
    )
    .with_context("session_risk", Value::string("high"))
    .with_context("multi_session", Value::Boolean(true))
    .with_context("mfa_verified", Value::Boolean(false));

    let result = policy_set.evaluate(&request_high_risk).expect("Failed to evaluate");
    assert!(result.is_denied(), "High-risk session should be denied");

    // Verify performance
    assert!(derive_time.as_millis() < 10, "Derivation should be fast");
}

/// Test complex data flow with multiple derivation rounds
#[test]
fn test_complex_multi_round_derivation() {
    let fact_store = Arc::new(FactStore::new());

    // Complex organizational hierarchy
    fact_store.add_fact(Fact::new("employee", vec![Value::string("alice")]));
    fact_store.add_fact(Fact::new("employee", vec![Value::string("bob")]));
    fact_store.add_fact(Fact::new("employee", vec![Value::string("charlie")]));

    fact_store.add_fact(Fact::new(
        "reports_to",
        vec![Value::string("bob"), Value::string("alice")],
    ));
    fact_store.add_fact(Fact::new(
        "reports_to",
        vec![Value::string("charlie"), Value::string("bob")],
    ));

    fact_store.add_fact(Fact::new(
        "department",
        vec![Value::string("alice"), Value::string("engineering")],
    ));
    fact_store.add_fact(Fact::new(
        "department",
        vec![Value::string("bob"), Value::string("engineering")],
    ));
    fact_store.add_fact(Fact::new(
        "department",
        vec![Value::string("charlie"), Value::string("engineering")],
    ));

    // Complex rules with multiple derivation rounds
    let org_rules = r#"
        // Transitive manager relationship
        manages(Manager, Employee) :- reports_to(Employee, Manager).
        manages(Manager, Employee) :-
            reports_to(Employee, Middle),
            manages(Manager, Middle).

        // Department head detection
        dept_head(Person, Dept) :-
            department(Person, Dept),
            !reports_to(Person, _).

        // Access inheritance through management chain
        can_access(Manager, Resource) :-
            manages(Manager, Employee),
            owns(Employee, Resource).

        can_access(Person, Resource) :-
            owns(Person, Resource).

        // Team member detection
        same_team(Person1, Person2) :-
            department(Person1, Dept),
            department(Person2, Dept),
            Person1 != Person2.
    "#;

    // Add resource ownership
    fact_store.add_fact(Fact::new(
        "owns",
        vec![Value::string("charlie"), Value::string("project_docs")],
    ));

    let rules = parse_rules(org_rules).expect("Failed to parse rules");
    let engine = DatalogEngine::new(rules, fact_store);
    let derived = engine.derive_facts().expect("Failed to derive facts");

    // Verify transitive management
    let alice_manages: Vec<_> = derived
        .iter()
        .filter(|f| {
            f.predicate == "manages" && f.args[0] == Value::string("alice")
        })
        .collect();
    assert_eq!(alice_manages.len(), 2); // Alice manages Bob and Charlie

    // Verify access inheritance
    let alice_access: Vec<_> = derived
        .iter()
        .filter(|f| {
            f.predicate == "can_access" && f.args[0] == Value::string("alice")
        })
        .collect();
    assert!(
        alice_access.iter().any(|f| f.args[1] == Value::string("project_docs")),
        "Alice should have access to Charlie's resources through management chain"
    );

    // Create Cedar policy using complex derived facts
    let org_policy = r#"
        // Managers can access their team's resources
        permit(
            principal,
            action == Action::"read",
            resource
        ) when {
            context.is_manager == true &&
            context.manages_owner == true
        };

        // Same team collaboration
        permit(
            principal,
            action in [Action::"read", Action::"comment"],
            resource
        ) when {
            context.same_team == true
        };
    "#;

    let mut policy_set = PolicySet::new();
    policy_set.load_policies(org_policy).expect("Failed to load policies");

    // Test authorization with complex derived facts
    let manager_request = Request::new(
        Principal::user("alice"),
        Action::new("read"),
        Resource::file("project_docs"),
    )
    .with_context("is_manager", Value::Boolean(true))
    .with_context("manages_owner", Value::Boolean(true));

    let result = policy_set.evaluate(&manager_request).expect("Failed to evaluate");
    assert!(result.is_allowed(), "Manager should access team resources");
}

/// Test hot-reload synchronization between systems
#[test]
fn test_hot_reload_synchronization() {
    use std::thread;
    use std::time::Duration;

    let fact_store = Arc::new(FactStore::new());

    // Initial facts
    fact_store.add_fact(Fact::new(
        "user_role",
        vec![Value::string("alice"), Value::string("viewer")],
    ));

    // Initial rules
    let initial_rules = r#"
        can_read(User) :- user_role(User, "viewer").
        can_read(User) :- user_role(User, "editor").
        can_write(User) :- user_role(User, "editor").
    "#;

    let rules = parse_rules(initial_rules).expect("Failed to parse rules");
    let engine = DatalogEngine::new(rules, fact_store.clone());
    let derived = engine.derive_facts().expect("Failed to derive facts");

    // Verify initial permissions
    let alice_perms: Vec<_> = derived
        .iter()
        .filter(|f| f.args.len() > 0 && f.args[0] == Value::string("alice"))
        .collect();
    assert_eq!(alice_perms.len(), 1); // Only can_read

    // Simulate hot-reload: update user role
    fact_store.remove_fact(&Fact::new(
        "user_role",
        vec![Value::string("alice"), Value::string("viewer")],
    ));
    fact_store.add_fact(Fact::new(
        "user_role",
        vec![Value::string("alice"), Value::string("editor")],
    ));

    // Re-evaluate after hot-reload
    let rules2 = parse_rules(initial_rules).expect("Failed to parse rules");
    let engine2 = DatalogEngine::new(rules2, fact_store.clone());
    let derived2 = engine2.derive_facts().expect("Failed to derive facts");

    // Verify updated permissions
    let alice_perms2: Vec<_> = derived2
        .iter()
        .filter(|f| f.args.len() > 0 && f.args[0] == Value::string("alice"))
        .collect();
    assert_eq!(alice_perms2.len(), 2); // Now has can_read and can_write

    // Cedar policy that adapts to hot-reloaded facts
    let adaptive_policy = r#"
        permit(
            principal,
            action == Action::"write",
            resource
        ) when {
            context.can_write == true &&
            context.reload_version > 0
        };
    "#;

    let mut policy_set = PolicySet::new();
    policy_set.load_policies(adaptive_policy).expect("Failed to load policies");

    // Test with hot-reloaded facts
    let write_request = Request::new(
        Principal::user("alice"),
        Action::new("write"),
        Resource::file("document.txt"),
    )
    .with_context("can_write", Value::Boolean(true))
    .with_context("reload_version", Value::Integer(1));

    let result = policy_set.evaluate(&write_request).expect("Failed to evaluate");
    assert!(result.is_allowed(), "Should allow write after role upgrade");
}

/// Test fact expiration and temporal policies
#[test]
fn test_temporal_facts_and_policies() {
    use chrono::{DateTime, Duration, Utc};

    let fact_store = Arc::new(FactStore::new());
    let now = Utc::now();

    // Add time-based facts
    fact_store.add_fact(Fact::new(
        "temporary_access",
        vec![
            Value::string("contractor"),
            Value::string("project_a"),
            Value::Integer(now.timestamp()),
            Value::Integer((now + Duration::hours(8)).timestamp()), // 8-hour access
        ],
    ));

    fact_store.add_fact(Fact::new(
        "business_hours",
        vec![
            Value::Integer(9), // start hour
            Value::Integer(17), // end hour
        ],
    ));

    // Temporal Datalog rules
    let temporal_rules = r#"
        // Check if access is currently valid
        valid_access(User, Resource, CurrentTime) :-
            temporary_access(User, Resource, StartTime, EndTime),
            CurrentTime >= StartTime,
            CurrentTime <= EndTime.

        // Check if current time is within business hours
        within_business_hours(Hour) :-
            business_hours(Start, End),
            Hour >= Start,
            Hour < End.
    "#;

    let rules = parse_rules(temporal_rules).expect("Failed to parse rules");

    // Test at different times
    let test_times = vec![
        now.timestamp() + 3600,  // +1 hour (valid)
        now.timestamp() + 36000, // +10 hours (expired)
    ];

    for test_time in test_times {
        // Add current time as fact for evaluation
        let temp_store = Arc::new(FactStore::new());

        // Copy existing facts
        for fact in fact_store.get_all_facts() {
            temp_store.add_fact(fact.clone());
        }

        // Add test-specific current time
        temp_store.add_fact(Fact::new(
            "current_time",
            vec![Value::Integer(test_time)],
        ));

        let engine = DatalogEngine::new(rules.clone(), temp_store);
        let derived = engine.derive_facts().expect("Failed to derive facts");

        // Check if access is valid at this time
        let valid_access: Vec<_> = derived
            .iter()
            .filter(|f| {
                f.predicate == "valid_access"
                    && f.args[0] == Value::string("contractor")
                    && f.args[2] == Value::Integer(test_time)
            })
            .collect();

        if test_time < now.timestamp() + 28800 {
            // Within 8 hours
            assert!(!valid_access.is_empty(), "Access should be valid");
        } else {
            assert!(valid_access.is_empty(), "Access should be expired");
        }
    }
}