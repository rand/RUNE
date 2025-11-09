//! Integration tests for RUNE Datalog engine
//!
//! Tests the full pipeline: parser → evaluation → Cedar bridge → authorization

use rune_core::datalog::{CedarDatalogBridge, DatalogEngine};
use rune_core::facts::{Fact, FactStore};
use rune_core::parser::parse_rules;
use rune_core::request::Request;
use rune_core::types::{Action, Principal, Resource, Value};
use std::sync::Arc;

#[test]
fn test_end_to_end_role_based_access() {
    // Parse Datalog rules
    let rules_source = r#"
        user_can(User, Permission) :- has_role(User, Role), role_permission(Role, Permission).
    "#;

    let rules = parse_rules(rules_source).expect("Failed to parse rules");
    assert_eq!(rules.len(), 1);

    // Create fact store with base facts
    let fact_store = Arc::new(FactStore::new());
    fact_store.add_fact(Fact::new(
        "has_role".to_string(),
        vec![Value::string("alice"), Value::string("admin")],
    ));
    fact_store.add_fact(Fact::new(
        "has_role".to_string(),
        vec![Value::string("bob"), Value::string("developer")],
    ));
    fact_store.add_fact(Fact::new(
        "role_permission".to_string(),
        vec![Value::string("admin"), Value::string("read")],
    ));
    fact_store.add_fact(Fact::new(
        "role_permission".to_string(),
        vec![Value::string("admin"), Value::string("write")],
    ));
    fact_store.add_fact(Fact::new(
        "role_permission".to_string(),
        vec![Value::string("developer"), Value::string("read")],
    ));

    // Create Datalog engine and derive facts
    let engine = DatalogEngine::new(rules, fact_store);
    let derived = engine.derive_facts().expect("Failed to derive facts");

    // Should derive user_can facts
    let user_can_facts: Vec<_> = derived
        .iter()
        .filter(|f| f.predicate.as_ref() == "user_can")
        .collect();

    // alice should have read and write permissions
    // bob should have read permission
    assert_eq!(user_can_facts.len(), 3);

    // Verify specific permissions
    assert!(user_can_facts.iter().any(|f| {
        matches!(&f.args[..], [Value::String(u), Value::String(p)]
            if u.as_ref() == "alice" && p.as_ref() == "read")
    }));
    assert!(user_can_facts.iter().any(|f| {
        matches!(&f.args[..], [Value::String(u), Value::String(p)]
            if u.as_ref() == "alice" && p.as_ref() == "write")
    }));
    assert!(user_can_facts.iter().any(|f| {
        matches!(&f.args[..], [Value::String(u), Value::String(p)]
            if u.as_ref() == "bob" && p.as_ref() == "read")
    }));
}

#[test]
fn test_end_to_end_transitive_permissions() {
    // Parse transitive closure rules
    let rules_source = r#"
        has_access(User, Resource) :- can_access(User, Resource).
        has_access(User, Child) :- has_access(User, Parent), parent_resource(Child, Parent).
    "#;

    let rules = parse_rules(rules_source).expect("Failed to parse rules");
    assert_eq!(rules.len(), 2);

    // Create fact store
    let fact_store = Arc::new(FactStore::new());

    // Direct access
    fact_store.add_fact(Fact::new(
        "can_access".to_string(),
        vec![Value::string("alice"), Value::string("/projects")],
    ));

    // Resource hierarchy
    fact_store.add_fact(Fact::new(
        "parent_resource".to_string(),
        vec![Value::string("/projects/alpha"), Value::string("/projects")],
    ));
    fact_store.add_fact(Fact::new(
        "parent_resource".to_string(),
        vec![
            Value::string("/projects/alpha/docs"),
            Value::string("/projects/alpha"),
        ],
    ));

    // Create engine and derive
    let engine = DatalogEngine::new(rules, fact_store);
    let derived = engine.derive_facts().expect("Failed to derive facts");

    // Should derive transitive access
    let access_facts: Vec<_> = derived
        .iter()
        .filter(|f| f.predicate.as_ref() == "has_access")
        .collect();

    // alice should have access to /projects, /projects/alpha, /projects/alpha/docs
    assert_eq!(access_facts.len(), 3);

    assert!(access_facts.iter().any(|f| {
        matches!(&f.args[..], [Value::String(u), Value::String(r)]
            if u.as_ref() == "alice" && r.as_ref() == "/projects")
    }));
    assert!(access_facts.iter().any(|f| {
        matches!(&f.args[..], [Value::String(u), Value::String(r)]
            if u.as_ref() == "alice" && r.as_ref() == "/projects/alpha")
    }));
    assert!(access_facts.iter().any(|f| {
        matches!(&f.args[..], [Value::String(u), Value::String(r)]
            if u.as_ref() == "alice" && r.as_ref() == "/projects/alpha/docs")
    }));
}

#[test]
fn test_end_to_end_negation() {
    // Parse rules with negation
    let rules_source = r#"
        allowed(User) :- user(User), not blocked(User).
    "#;

    let rules = parse_rules(rules_source).expect("Failed to parse rules");
    assert_eq!(rules.len(), 1);

    // Create fact store
    let fact_store = Arc::new(FactStore::new());

    // Users
    fact_store.add_fact(Fact::new("user".to_string(), vec![Value::string("alice")]));
    fact_store.add_fact(Fact::new("user".to_string(), vec![Value::string("bob")]));
    fact_store.add_fact(Fact::new(
        "user".to_string(),
        vec![Value::string("charlie")],
    ));

    // Blocked users
    fact_store.add_fact(Fact::new(
        "blocked".to_string(),
        vec![Value::string("charlie")],
    ));

    // Create engine and derive
    let engine = DatalogEngine::new(rules, fact_store);
    let derived = engine.derive_facts().expect("Failed to derive facts");

    // Should derive allowed users
    let allowed_facts: Vec<_> = derived
        .iter()
        .filter(|f| f.predicate.as_ref() == "allowed")
        .collect();

    // alice and bob should be allowed, charlie should not
    assert_eq!(allowed_facts.len(), 2);

    assert!(allowed_facts
        .iter()
        .any(|f| matches!(&f.args[..], [Value::String(u)] if u.as_ref() == "alice")));
    assert!(allowed_facts
        .iter()
        .any(|f| matches!(&f.args[..], [Value::String(u)] if u.as_ref() == "bob")));
    assert!(!allowed_facts
        .iter()
        .any(|f| matches!(&f.args[..], [Value::String(u)] if u.as_ref() == "charlie")));
}

#[test]
fn test_cedar_bridge_request_conversion() {
    // Create a request with hierarchical entities
    let parent_group = rune_core::types::Entity::new("Group", "admins")
        .with_attribute("level", Value::Integer(10));

    let principal = Principal::user("alice")
        .entity
        .with_parent(parent_group)
        .with_attribute("role", Value::string("admin"))
        .with_attribute("department", Value::string("engineering"));

    let principal = Principal { entity: principal };

    let resource = Resource::file("/tmp/secret.txt")
        .entity
        .with_attribute("owner", Value::string("alice"))
        .with_attribute("confidential", Value::Bool(true));

    let resource = Resource { entity: resource };

    let action = Action::new("read").with_parameter("mode", Value::string("readonly"));

    let request = Request::new(principal, action, resource)
        .with_context("ip_address", Value::string("192.168.1.100"));

    // Convert to facts
    let facts = CedarDatalogBridge::request_to_facts(&request);

    // Should have principal, action, resource, and context facts
    assert!(!facts.is_empty());

    // Verify principal facts
    assert!(facts.iter().any(|f| f.predicate.as_ref() == "principal"));
    assert!(facts
        .iter()
        .any(|f| f.predicate.as_ref() == "principal_attr"));
    assert!(facts
        .iter()
        .any(|f| f.predicate.as_ref() == "principal_parent"));

    // Verify resource facts
    assert!(facts.iter().any(|f| f.predicate.as_ref() == "resource"));
    assert!(facts
        .iter()
        .any(|f| f.predicate.as_ref() == "resource_attr"));

    // Verify action facts
    assert!(facts.iter().any(|f| f.predicate.as_ref() == "action"));
    assert!(facts.iter().any(|f| f.predicate.as_ref() == "action_param"));

    // Verify context facts
    assert!(facts.iter().any(|f| f.predicate.as_ref() == "context"));
}

#[test]
fn test_integration_request_authorization_with_rules() {
    // Scenario: Use Datalog rules to derive permissions, then use Cedar bridge
    // to convert request entities to facts

    // 1. Parse authorization rules
    let rules_source = r#"
        can_read(User, File) :- user_role(User, "admin").
        can_read(User, File) :- file_owner(File, User).
    "#;

    let rules = parse_rules(rules_source).expect("Failed to parse rules");

    // 2. Create fact store with authorization data
    let fact_store = Arc::new(FactStore::new());
    fact_store.add_fact(Fact::new(
        "user_role".to_string(),
        vec![Value::string("alice"), Value::string("admin")],
    ));
    fact_store.add_fact(Fact::new(
        "file_owner".to_string(),
        vec![Value::string("/data.txt"), Value::string("bob")],
    ));

    // 3. Create request
    let principal = Principal::user("alice");
    let action = Action::new("read");
    let resource = Resource::file("/data.txt");
    let request = Request::new(principal, action, resource);

    // 4. Convert request to facts
    let request_facts = CedarDatalogBridge::request_to_facts(&request);

    // Add request facts to store
    for fact in request_facts {
        fact_store.add_fact(fact);
    }

    // 5. Derive permissions
    let engine = DatalogEngine::new(rules, fact_store);
    let derived = engine.derive_facts().expect("Failed to derive facts");

    // 6. Check if alice can read /data.txt
    let can_read_facts: Vec<_> = derived
        .iter()
        .filter(|f| f.predicate.as_ref() == "can_read")
        .collect();

    // alice should be able to read (admin role)
    assert!(can_read_facts.iter().any(|f| {
        matches!(&f.args[..], [Value::String(u), Value::String(_f)]
            if u.as_ref() == "alice")
    }));
}

#[test]
fn test_complex_hierarchy_with_attributes() {
    // Test complex scenario with:
    // - Multiple levels of hierarchy
    // - Attribute-based rules
    // - Transitive relationships

    let rules_source = r#"
        has_permission(User, Resource, Permission) :-
            user_group(User, Group),
            group_permission(Group, Resource, Permission).

        user_group(User, Parent) :-
            user_group(User, Child),
            group_parent(Child, Parent).
    "#;

    let rules = parse_rules(rules_source).expect("Failed to parse rules");

    let fact_store = Arc::new(FactStore::new());

    // Direct group membership
    fact_store.add_fact(Fact::new(
        "user_group".to_string(),
        vec![Value::string("alice"), Value::string("developers")],
    ));

    // Group hierarchy
    fact_store.add_fact(Fact::new(
        "group_parent".to_string(),
        vec![Value::string("developers"), Value::string("engineering")],
    ));
    fact_store.add_fact(Fact::new(
        "group_parent".to_string(),
        vec![Value::string("engineering"), Value::string("employees")],
    ));

    // Permissions at different levels
    fact_store.add_fact(Fact::new(
        "group_permission".to_string(),
        vec![
            Value::string("developers"),
            Value::string("/code"),
            Value::string("write"),
        ],
    ));
    fact_store.add_fact(Fact::new(
        "group_permission".to_string(),
        vec![
            Value::string("engineering"),
            Value::string("/docs"),
            Value::string("read"),
        ],
    ));
    fact_store.add_fact(Fact::new(
        "group_permission".to_string(),
        vec![
            Value::string("employees"),
            Value::string("/wiki"),
            Value::string("read"),
        ],
    ));

    let engine = DatalogEngine::new(rules, fact_store);
    let derived = engine.derive_facts().expect("Failed to derive facts");

    // Check user_group transitive closure
    let user_groups: Vec<_> = derived
        .iter()
        .filter(|f| f.predicate.as_ref() == "user_group")
        .collect();

    // alice should be in: developers, engineering, employees
    assert!(user_groups.len() >= 3);

    // Check permissions
    let permissions: Vec<_> = derived
        .iter()
        .filter(|f| f.predicate.as_ref() == "has_permission")
        .collect();

    // alice should have permissions from all three groups
    assert!(permissions.len() >= 3);

    // Verify specific permissions
    assert!(permissions.iter().any(|f| {
        matches!(&f.args[..], [Value::String(u), Value::String(r), Value::String(p)]
            if u.as_ref() == "alice" && r.as_ref() == "/code" && p.as_ref() == "write")
    }));
    assert!(permissions.iter().any(|f| {
        matches!(&f.args[..], [Value::String(u), Value::String(r), Value::String(p)]
            if u.as_ref() == "alice" && r.as_ref() == "/docs" && p.as_ref() == "read")
    }));
    assert!(permissions.iter().any(|f| {
        matches!(&f.args[..], [Value::String(u), Value::String(r), Value::String(p)]
            if u.as_ref() == "alice" && r.as_ref() == "/wiki" && p.as_ref() == "read")
    }));
}

#[test]
fn test_performance_with_large_dataset() {
    // Test that the engine performs well with a larger dataset

    let rules_source = r#"
        connected(X, Y) :- edge(X, Y).
        connected(X, Z) :- connected(X, Y), edge(Y, Z).
    "#;

    let rules = parse_rules(rules_source).expect("Failed to parse rules");
    let fact_store = Arc::new(FactStore::new());

    // Create a chain of 100 nodes
    for i in 0..100 {
        fact_store.add_fact(Fact::new(
            "edge".to_string(),
            vec![Value::Integer(i), Value::Integer(i + 1)],
        ));
    }

    let engine = DatalogEngine::new(rules, fact_store);

    // Time the evaluation
    let start = std::time::Instant::now();
    let derived = engine.derive_facts().expect("Failed to derive facts");
    let duration = start.elapsed();

    // Should derive all transitive connections
    let connected_facts: Vec<_> = derived
        .iter()
        .filter(|f| f.predicate.as_ref() == "connected")
        .collect();

    // Should have 100 + 99 + 98 + ... + 1 = 5050 connections
    assert!(connected_facts.len() > 100);

    // Should complete in reasonable time (<100ms)
    assert!(
        duration.as_millis() < 100,
        "Evaluation took {}ms, expected <100ms",
        duration.as_millis()
    );
}
