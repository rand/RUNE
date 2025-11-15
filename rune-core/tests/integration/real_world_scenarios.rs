//! Real-world authorization scenarios integration tests
//!
//! Tests common production authorization patterns including:
//! - Multi-tenant isolation
//! - Geographic restrictions
//! - Time-based access control
//! - Rate limiting
//! - Hierarchical permissions
//! - Compliance and audit trails

use rune_core::datalog::{DatalogEngine, Rule};
use rune_core::facts::{Fact, FactStore};
use rune_core::parser::parse_rules;
use rune_core::policy::{PolicyEngine, PolicySet};
use rune_core::request::Request;
use rune_core::types::{Action, Principal, Resource, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// Test multi-tenant SaaS application with strict tenant isolation
#[test]
fn test_multi_tenant_saas_authorization() {
    let fact_store = Arc::new(FactStore::new());

    // Setup multi-tenant facts
    // Tenant 1: Acme Corp
    fact_store.add_fact(Fact::new(
        "tenant",
        vec![Value::string("acme"), Value::string("enterprise")],
    ));
    fact_store.add_fact(Fact::new(
        "user_tenant",
        vec![Value::string("alice@acme.com"), Value::string("acme")],
    ));
    fact_store.add_fact(Fact::new(
        "user_tenant",
        vec![Value::string("bob@acme.com"), Value::string("acme")],
    ));
    fact_store.add_fact(Fact::new(
        "resource_tenant",
        vec![Value::string("acme_database"), Value::string("acme")],
    ));
    fact_store.add_fact(Fact::new(
        "resource_tenant",
        vec![Value::string("acme_files"), Value::string("acme")],
    ));

    // Tenant 2: Widget Inc
    fact_store.add_fact(Fact::new(
        "tenant",
        vec![Value::string("widget"), Value::string("standard")],
    ));
    fact_store.add_fact(Fact::new(
        "user_tenant",
        vec![Value::string("charlie@widget.com"), Value::string("widget")],
    ));
    fact_store.add_fact(Fact::new(
        "resource_tenant",
        vec![Value::string("widget_database"), Value::string("widget")],
    ));

    // User roles within tenants
    fact_store.add_fact(Fact::new(
        "tenant_role",
        vec![
            Value::string("alice@acme.com"),
            Value::string("acme"),
            Value::string("admin"),
        ],
    ));
    fact_store.add_fact(Fact::new(
        "tenant_role",
        vec![
            Value::string("bob@acme.com"),
            Value::string("acme"),
            Value::string("user"),
        ],
    ));
    fact_store.add_fact(Fact::new(
        "tenant_role",
        vec![
            Value::string("charlie@widget.com"),
            Value::string("widget"),
            Value::string("admin"),
        ],
    ));

    // Datalog rules for tenant isolation
    let tenant_rules = r#"
        // Users can only access resources in their tenant
        can_access_resource(User, Resource) :-
            user_tenant(User, Tenant),
            resource_tenant(Resource, Tenant).

        // Admins have elevated permissions within their tenant
        tenant_admin(User, Tenant) :-
            tenant_role(User, Tenant, "admin").

        // Enterprise tenants get additional features
        has_feature(Tenant, "advanced_analytics") :-
            tenant(Tenant, "enterprise").

        // Cross-tenant access is explicitly forbidden (derived for audit)
        cross_tenant_violation(User, Resource) :-
            user_tenant(User, UserTenant),
            resource_tenant(Resource, ResourceTenant),
            UserTenant != ResourceTenant.
    "#;

    let rules = parse_rules(tenant_rules).expect("Failed to parse rules");
    let engine = DatalogEngine::new(rules, fact_store.clone());
    let derived = engine.derive_facts().expect("Failed to derive facts");

    // Verify tenant isolation
    let alice_access: Vec<_> = derived
        .iter()
        .filter(|f| {
            f.predicate == "can_access_resource"
                && f.args[0] == Value::string("alice@acme.com")
        })
        .collect();

    // Alice should only access Acme resources
    for fact in &alice_access {
        assert!(fact.args[1].to_string().contains("acme"));
    }

    // Check for cross-tenant violations
    let violations: Vec<_> = derived
        .iter()
        .filter(|f| f.predicate == "cross_tenant_violation")
        .collect();
    assert!(!violations.is_empty(), "Should detect cross-tenant attempts");

    // Cedar policies enforcing tenant isolation
    let tenant_policy = r#"
        // Strict tenant isolation
        forbid(
            principal,
            action,
            resource
        ) when {
            context.user_tenant != context.resource_tenant
        };

        // Allow access within same tenant
        permit(
            principal,
            action in [Action::"read", Action::"write"],
            resource
        ) when {
            context.user_tenant == context.resource_tenant &&
            context.tenant_verified == true
        };

        // Enterprise features
        permit(
            principal,
            action == Action::"use_advanced_analytics",
            resource
        ) when {
            context.tenant_plan == "enterprise"
        };
    "#;

    let mut policy_set = PolicySet::new();
    policy_set.load_policies(tenant_policy).expect("Failed to load policies");

    // Test: Alice accessing Acme resource (should succeed)
    let valid_request = Request::new(
        Principal::user("alice@acme.com"),
        Action::new("read"),
        Resource::database("acme_database"),
    )
    .with_context("user_tenant", Value::string("acme"))
    .with_context("resource_tenant", Value::string("acme"))
    .with_context("tenant_verified", Value::Boolean(true))
    .with_context("tenant_plan", Value::string("enterprise"));

    let result = policy_set.evaluate(&valid_request).expect("Failed to evaluate");
    assert!(result.is_allowed(), "Same-tenant access should be allowed");

    // Test: Charlie accessing Acme resource (should fail)
    let invalid_request = Request::new(
        Principal::user("charlie@widget.com"),
        Action::new("read"),
        Resource::database("acme_database"),
    )
    .with_context("user_tenant", Value::string("widget"))
    .with_context("resource_tenant", Value::string("acme"))
    .with_context("tenant_verified", Value::Boolean(true));

    let result = policy_set.evaluate(&invalid_request).expect("Failed to evaluate");
    assert!(result.is_denied(), "Cross-tenant access should be denied");
}

/// Test geographic restriction and data sovereignty
#[test]
fn test_geographic_restrictions() {
    let fact_store = Arc::new(FactStore::new());

    // User locations
    fact_store.add_fact(Fact::new(
        "user_location",
        vec![
            Value::string("eu_user"),
            Value::string("germany"),
            Value::string("eu"),
        ],
    ));
    fact_store.add_fact(Fact::new(
        "user_location",
        vec![
            Value::string("us_user"),
            Value::string("california"),
            Value::string("us"),
        ],
    ));
    fact_store.add_fact(Fact::new(
        "user_location",
        vec![
            Value::string("asia_user"),
            Value::string("japan"),
            Value::string("asia"),
        ],
    ));

    // Data residency requirements
    fact_store.add_fact(Fact::new(
        "data_residency",
        vec![Value::string("eu_customer_data"), Value::string("eu")],
    ));
    fact_store.add_fact(Fact::new(
        "data_residency",
        vec![Value::string("us_health_records"), Value::string("us")],
    ));

    // GDPR compliance flags
    fact_store.add_fact(Fact::new(
        "gdpr_region",
        vec![Value::string("eu")],
    ));
    fact_store.add_fact(Fact::new(
        "gdpr_consent",
        vec![Value::string("eu_user"), Value::Boolean(true)],
    ));

    // Geographic access rules
    let geo_rules = r#"
        // Users can access data in their region
        regional_access(User, Data) :-
            user_location(User, _, Region),
            data_residency(Data, Region).

        // GDPR compliance requirements
        requires_gdpr_compliance(User) :-
            user_location(User, _, Region),
            gdpr_region(Region).

        gdpr_compliant_access(User, Data) :-
            requires_gdpr_compliance(User),
            gdpr_consent(User, true),
            regional_access(User, Data).

        // Cross-region access for global admins
        global_admin_access(User, Data) :-
            global_admin(User),
            admin_region_approved(Data).

        // Data sovereignty violations
        sovereignty_violation(User, Data) :-
            user_location(User, _, UserRegion),
            data_residency(Data, DataRegion),
            UserRegion != DataRegion,
            !global_admin(User).
    "#;

    // Add a global admin
    fact_store.add_fact(Fact::new(
        "global_admin",
        vec![Value::string("admin@global.com")],
    ));

    let rules = parse_rules(geo_rules).expect("Failed to parse rules");
    let engine = DatalogEngine::new(rules, fact_store);
    let derived = engine.derive_facts().expect("Failed to derive facts");

    // Verify regional access
    let eu_access: Vec<_> = derived
        .iter()
        .filter(|f| {
            f.predicate == "regional_access" && f.args[0] == Value::string("eu_user")
        })
        .collect();
    assert_eq!(eu_access.len(), 1);
    assert_eq!(eu_access[0].args[1], Value::string("eu_customer_data"));

    // Check GDPR compliance
    let gdpr_required: Vec<_> = derived
        .iter()
        .filter(|f| f.predicate == "requires_gdpr_compliance")
        .collect();
    assert!(!gdpr_required.is_empty());

    // Cedar policy with geographic restrictions
    let geo_policy = r#"
        // Enforce data residency
        forbid(
            principal,
            action,
            resource
        ) when {
            context.user_region != context.data_region &&
            context.is_global_admin != true
        };

        // GDPR compliance
        forbid(
            principal,
            action in [Action::"read", Action::"process"],
            resource
        ) when {
            context.requires_gdpr == true &&
            context.has_consent != true
        };

        // Allow regional access
        permit(
            principal,
            action,
            resource
        ) when {
            context.user_region == context.data_region &&
            context.compliance_verified == true
        };

        // VPN detection and blocking
        forbid(
            principal,
            action,
            resource
        ) when {
            context.vpn_detected == true &&
            resource has "sensitive" &&
            context.vpn_authorized != true
        };
    "#;

    let mut policy_set = PolicySet::new();
    policy_set.load_policies(geo_policy).expect("Failed to load policies");

    // Test: EU user accessing EU data (should succeed)
    let valid_eu_request = Request::new(
        Principal::user("eu_user"),
        Action::new("read"),
        Resource::data("eu_customer_data"),
    )
    .with_context("user_region", Value::string("eu"))
    .with_context("data_region", Value::string("eu"))
    .with_context("requires_gdpr", Value::Boolean(true))
    .with_context("has_consent", Value::Boolean(true))
    .with_context("compliance_verified", Value::Boolean(true));

    let result = policy_set.evaluate(&valid_eu_request).expect("Failed to evaluate");
    assert!(result.is_allowed(), "Regional access should be allowed");

    // Test: US user accessing EU data (should fail)
    let cross_region_request = Request::new(
        Principal::user("us_user"),
        Action::new("read"),
        Resource::data("eu_customer_data"),
    )
    .with_context("user_region", Value::string("us"))
    .with_context("data_region", Value::string("eu"))
    .with_context("is_global_admin", Value::Boolean(false))
    .with_context("compliance_verified", Value::Boolean(true));

    let result = policy_set.evaluate(&cross_region_request).expect("Failed to evaluate");
    assert!(result.is_denied(), "Cross-region access should be denied");
}

/// Test rate limiting and abuse prevention
#[test]
fn test_rate_limiting_and_abuse_prevention() {
    let fact_store = Arc::new(FactStore::new());

    // Simulate API request history
    let users = vec!["normal_user", "suspicious_user", "bot_user"];
    let now = 1700000000i64; // Current timestamp

    // Normal user: steady rate
    for i in 0..10 {
        fact_store.add_fact(Fact::new(
            "api_request",
            vec![
                Value::string("normal_user"),
                Value::Integer(now - 60 + i * 6), // 10 requests per minute
                Value::string("/api/data"),
            ],
        ));
    }

    // Suspicious user: burst of requests
    for i in 0..50 {
        fact_store.add_fact(Fact::new(
            "api_request",
            vec![
                Value::string("suspicious_user"),
                Value::Integer(now - 10 + i), // 50 requests in 10 seconds
                Value::string("/api/data"),
            ],
        ));
    }

    // Bot user: automated pattern
    for i in 0..100 {
        fact_store.add_fact(Fact::new(
            "api_request",
            vec![
                Value::string("bot_user"),
                Value::Integer(now - 60 + i), // Exactly 1 request per second
                Value::string("/api/scrape"),
            ],
        ));
    }

    // User tiers and limits
    fact_store.add_fact(Fact::new(
        "user_tier",
        vec![Value::string("normal_user"), Value::string("free")],
    ));
    fact_store.add_fact(Fact::new(
        "user_tier",
        vec![Value::string("suspicious_user"), Value::string("free")],
    ));
    fact_store.add_fact(Fact::new(
        "user_tier",
        vec![Value::string("premium_user"), Value::string("premium")],
    ));

    fact_store.add_fact(Fact::new(
        "tier_limit",
        vec![
            Value::string("free"),
            Value::Integer(60), // requests per minute
        ],
    ));
    fact_store.add_fact(Fact::new(
        "tier_limit",
        vec![
            Value::string("premium"),
            Value::Integer(600),
        ],
    ));

    // Rate limiting rules
    let rate_rules = r#"
        // Count requests per user in time window
        request_count(User, Count, StartTime, EndTime) :-
            api_request(User, Time, _),
            Time >= StartTime,
            Time <= EndTime,
            count(Time, Count).

        // Detect rate limit violations
        rate_limit_exceeded(User, Count) :-
            request_count(User, Count, _, _),
            user_tier(User, Tier),
            tier_limit(Tier, Limit),
            Count > Limit.

        // Detect bot patterns (exact timing)
        bot_pattern_detected(User) :-
            api_request(User, Time1, _),
            api_request(User, Time2, _),
            api_request(User, Time3, _),
            Time2 - Time1 == 1,
            Time3 - Time2 == 1.

        // Detect burst patterns
        burst_detected(User) :-
            request_count(User, Count, StartTime, EndTime),
            EndTime - StartTime < 10,
            Count > 30.

        // Calculate risk score
        user_risk_score(User, "high") :-
            rate_limit_exceeded(User, _),
            burst_detected(User).

        user_risk_score(User, "medium") :-
            rate_limit_exceeded(User, _),
            !burst_detected(User).

        user_risk_score(User, "low") :-
            !rate_limit_exceeded(User, _).
    "#;

    let rules = parse_rules(rate_rules).expect("Failed to parse rules");
    let engine = DatalogEngine::new(rules, fact_store);
    let derived = engine.derive_facts().expect("Failed to derive facts");

    // Check rate limit violations
    let violations: Vec<_> = derived
        .iter()
        .filter(|f| f.predicate == "rate_limit_exceeded")
        .collect();
    assert!(violations.len() >= 2, "Should detect rate limit violations");

    // Check burst detection
    let bursts: Vec<_> = derived
        .iter()
        .filter(|f| f.predicate == "burst_detected")
        .collect();
    assert!(
        bursts.iter().any(|f| f.args[0] == Value::string("suspicious_user")),
        "Should detect burst pattern"
    );

    // Cedar policy for rate limiting
    let rate_policy = r#"
        // Enforce rate limits
        forbid(
            principal,
            action,
            resource
        ) when {
            context.rate_limit_exceeded == true &&
            context.user_tier != "enterprise"
        };

        // Block suspected bots
        forbid(
            principal,
            action,
            resource
        ) when {
            context.bot_pattern_detected == true &&
            context.captcha_verified != true
        };

        // Throttle high-risk users
        forbid(
            principal,
            action,
            resource
        ) when {
            context.user_risk_score == "high" &&
            context.admin_override != true
        };

        // Allow premium users higher limits
        permit(
            principal,
            action,
            resource
        ) when {
            context.user_tier in ["premium", "enterprise"] &&
            context.requests_per_minute < 600
        };
    "#;

    let mut policy_set = PolicySet::new();
    policy_set.load_policies(rate_policy).expect("Failed to load policies");

    // Test: Suspicious user request (should be throttled)
    let suspicious_request = Request::new(
        Principal::user("suspicious_user"),
        Action::new("api_call"),
        Resource::api("/api/data"),
    )
    .with_context("rate_limit_exceeded", Value::Boolean(true))
    .with_context("user_risk_score", Value::string("high"))
    .with_context("user_tier", Value::string("free"))
    .with_context("admin_override", Value::Boolean(false));

    let result = policy_set.evaluate(&suspicious_request).expect("Failed to evaluate");
    assert!(result.is_denied(), "High-risk user should be throttled");
}

/// Test hierarchical organization permissions
#[test]
fn test_hierarchical_organization_permissions() {
    let fact_store = Arc::new(FactStore::new());

    // Organization structure
    fact_store.add_fact(Fact::new(
        "organization",
        vec![Value::string("acme_corp")],
    ));

    // Departments
    fact_store.add_fact(Fact::new(
        "department",
        vec![Value::string("engineering"), Value::string("acme_corp")],
    ));
    fact_store.add_fact(Fact::new(
        "department",
        vec![Value::string("sales"), Value::string("acme_corp")],
    ));
    fact_store.add_fact(Fact::new(
        "department",
        vec![Value::string("hr"), Value::string("acme_corp")],
    ));

    // Teams within departments
    fact_store.add_fact(Fact::new(
        "team",
        vec![Value::string("backend"), Value::string("engineering")],
    ));
    fact_store.add_fact(Fact::new(
        "team",
        vec![Value::string("frontend"), Value::string("engineering")],
    ));
    fact_store.add_fact(Fact::new(
        "team",
        vec![Value::string("enterprise_sales"), Value::string("sales")],
    ));

    // User assignments
    fact_store.add_fact(Fact::new(
        "user_role",
        vec![
            Value::string("ceo@acme.com"),
            Value::string("acme_corp"),
            Value::string("ceo"),
        ],
    ));
    fact_store.add_fact(Fact::new(
        "user_role",
        vec![
            Value::string("cto@acme.com"),
            Value::string("engineering"),
            Value::string("head"),
        ],
    ));
    fact_store.add_fact(Fact::new(
        "user_role",
        vec![
            Value::string("dev@acme.com"),
            Value::string("backend"),
            Value::string("member"),
        ],
    ));
    fact_store.add_fact(Fact::new(
        "user_role",
        vec![
            Value::string("hr_manager@acme.com"),
            Value::string("hr"),
            Value::string("head"),
        ],
    ));

    // Resource ownership
    fact_store.add_fact(Fact::new(
        "resource_owner",
        vec![Value::string("backend_code"), Value::string("backend")],
    ));
    fact_store.add_fact(Fact::new(
        "resource_owner",
        vec![Value::string("employee_records"), Value::string("hr")],
    ));
    fact_store.add_fact(Fact::new(
        "resource_owner",
        vec![Value::string("financial_reports"), Value::string("acme_corp")],
    ));

    // Hierarchical permission rules
    let hierarchy_rules = r#"
        // Define organizational hierarchy
        parent_of(OrgUnit, Department) :-
            department(Department, OrgUnit).

        parent_of(Department, Team) :-
            team(Team, Department).

        // Transitive hierarchy
        ancestor_of(Ancestor, Descendant) :-
            parent_of(Ancestor, Descendant).

        ancestor_of(Ancestor, Descendant) :-
            parent_of(Ancestor, Middle),
            ancestor_of(Middle, Descendant).

        // Permission inheritance through hierarchy
        has_permission(User, Resource, "read") :-
            user_role(User, Unit, _),
            resource_owner(Resource, ResourceUnit),
            (Unit == ResourceUnit ; ancestor_of(Unit, ResourceUnit)).

        has_permission(User, Resource, "write") :-
            user_role(User, Unit, Role),
            resource_owner(Resource, Unit),
            (Role == "head" ; Role == "ceo").

        has_permission(User, Resource, "delete") :-
            user_role(User, _, "ceo").

        // Department heads can access their department and sub-units
        department_head_access(User, Resource) :-
            user_role(User, Dept, "head"),
            resource_owner(Resource, ResourceUnit),
            (ResourceUnit == Dept ; ancestor_of(Dept, ResourceUnit)).

        // Cross-department access restrictions
        cross_dept_restricted(User, Resource) :-
            user_role(User, UserUnit, Role),
            resource_owner(Resource, ResourceUnit),
            Role != "ceo",
            !ancestor_of(UserUnit, ResourceUnit),
            !ancestor_of(ResourceUnit, UserUnit),
            UserUnit != ResourceUnit.
    "#;

    let rules = parse_rules(hierarchy_rules).expect("Failed to parse rules");
    let engine = DatalogEngine::new(rules, fact_store);
    let derived = engine.derive_facts().expect("Failed to derive facts");

    // Verify hierarchical permissions
    // CEO should access everything
    let ceo_perms: Vec<_> = derived
        .iter()
        .filter(|f| {
            f.predicate == "has_permission"
                && f.args[0] == Value::string("ceo@acme.com")
        })
        .collect();
    assert!(ceo_perms.len() >= 3, "CEO should have broad access");

    // CTO should access engineering resources
    let cto_perms: Vec<_> = derived
        .iter()
        .filter(|f| {
            f.predicate == "has_permission"
                && f.args[0] == Value::string("cto@acme.com")
                && f.args[1] == Value::string("backend_code")
        })
        .collect();
    assert!(!cto_perms.is_empty(), "CTO should access engineering resources");

    // Dev should not access HR resources
    let cross_dept: Vec<_> = derived
        .iter()
        .filter(|f| {
            f.predicate == "cross_dept_restricted"
                && f.args[0] == Value::string("dev@acme.com")
                && f.args[1] == Value::string("employee_records")
        })
        .collect();
    assert!(!cross_dept.is_empty(), "Should detect cross-department restriction");

    // Cedar policy for hierarchical permissions
    let hierarchy_policy = r#"
        // CEO has full access
        permit(
            principal == User::"ceo@acme.com",
            action,
            resource
        ) when {
            context.org == "acme_corp"
        };

        // Department heads manage their departments
        permit(
            principal,
            action in [Action::"read", Action::"write", Action::"approve"],
            resource
        ) when {
            context.is_dept_head == true &&
            context.resource_in_dept == true
        };

        // Team members access team resources
        permit(
            principal,
            action in [Action::"read", Action::"write"],
            resource
        ) when {
            context.same_team == true &&
            context.resource_owner == context.user_team
        };

        // Block cross-department access
        forbid(
            principal,
            action,
            resource
        ) when {
            context.cross_dept_restricted == true &&
            context.special_permission != true
        };

        // HR special permissions for employee data
        permit(
            principal,
            action in [Action::"read", Action::"update"],
            resource has "employee_data"
        ) when {
            context.user_dept == "hr" &&
            context.privacy_training_completed == true
        };
    "#;

    let mut policy_set = PolicySet::new();
    policy_set.load_policies(hierarchy_policy).expect("Failed to load policies");

    // Test: CTO accessing backend code (should succeed)
    let cto_request = Request::new(
        Principal::user("cto@acme.com"),
        Action::new("write"),
        Resource::code("backend_code"),
    )
    .with_context("is_dept_head", Value::Boolean(true))
    .with_context("resource_in_dept", Value::Boolean(true))
    .with_context("user_dept", Value::string("engineering"));

    let result = policy_set.evaluate(&cto_request).expect("Failed to evaluate");
    assert!(result.is_allowed(), "CTO should access department resources");

    // Test: Dev accessing HR records (should fail)
    let cross_dept_request = Request::new(
        Principal::user("dev@acme.com"),
        Action::new("read"),
        Resource::data("employee_records"),
    )
    .with_context("cross_dept_restricted", Value::Boolean(true))
    .with_context("special_permission", Value::Boolean(false))
    .with_context("user_dept", Value::string("engineering"));

    let result = policy_set.evaluate(&cross_dept_request).expect("Failed to evaluate");
    assert!(result.is_denied(), "Cross-department access should be denied");
}

/// Test compliance and audit trail generation
#[test]
fn test_compliance_and_audit_trails() {
    let fact_store = Arc::new(FactStore::new());

    // Compliance requirements
    fact_store.add_fact(Fact::new(
        "compliance_requirement",
        vec![Value::string("hipaa"), Value::string("health_data")],
    ));
    fact_store.add_fact(Fact::new(
        "compliance_requirement",
        vec![Value::string("pci_dss"), Value::string("payment_data")],
    ));
    fact_store.add_fact(Fact::new(
        "compliance_requirement",
        vec![Value::string("sox"), Value::string("financial_data")],
    ));

    // User certifications
    fact_store.add_fact(Fact::new(
        "user_certification",
        vec![
            Value::string("doctor@hospital.com"),
            Value::string("hipaa"),
            Value::Boolean(true),
        ],
    ));
    fact_store.add_fact(Fact::new(
        "user_certification",
        vec![
            Value::string("analyst@bank.com"),
            Value::string("pci_dss"),
            Value::Boolean(true),
        ],
    ));

    // Data classification
    fact_store.add_fact(Fact::new(
        "data_classification",
        vec![
            Value::string("patient_records"),
            Value::string("health_data"),
            Value::string("confidential"),
        ],
    ));
    fact_store.add_fact(Fact::new(
        "data_classification",
        vec![
            Value::string("credit_cards"),
            Value::string("payment_data"),
            Value::string("restricted"),
        ],
    ));

    // Access events for audit
    let now = 1700000000i64;
    fact_store.add_fact(Fact::new(
        "access_event",
        vec![
            Value::string("doctor@hospital.com"),
            Value::string("patient_records"),
            Value::string("read"),
            Value::Integer(now),
            Value::string("success"),
        ],
    ));
    fact_store.add_fact(Fact::new(
        "access_event",
        vec![
            Value::string("nurse@hospital.com"),
            Value::string("patient_records"),
            Value::string("write"),
            Value::Integer(now + 100),
            Value::string("denied"),
        ],
    ));

    // Compliance and audit rules
    let compliance_rules = r#"
        // Compliance checks
        compliant_access(User, Data) :-
            data_classification(Data, DataType, _),
            compliance_requirement(Compliance, DataType),
            user_certification(User, Compliance, true).

        non_compliant_access(User, Data) :-
            data_classification(Data, DataType, _),
            compliance_requirement(Compliance, DataType),
            !user_certification(User, Compliance, true).

        // Audit trail requirements
        requires_audit(Data) :-
            data_classification(Data, _, Classification),
            (Classification == "confidential" ; Classification == "restricted").

        audit_entry(User, Data, Action, Time, Result, "required") :-
            access_event(User, Data, Action, Time, Result),
            requires_audit(Data).

        // Suspicious patterns
        suspicious_activity(User, "multiple_denials") :-
            access_event(User, _, _, _, "denied"),
            access_event(User, _, _, _, "denied"),
            access_event(User, _, _, _, "denied").

        suspicious_activity(User, "after_hours") :-
            access_event(User, _, _, Time, _),
            hour_from_timestamp(Time, Hour),
            (Hour < 6 ; Hour > 22).

        // Retention requirements
        retention_period(DataType, Days) :-
            compliance_requirement("hipaa", DataType),
            Days = 2555. // 7 years for HIPAA

        retention_period(DataType, Days) :-
            compliance_requirement("pci_dss", DataType),
            Days = 365. // 1 year for PCI-DSS
    "#;

    let rules = parse_rules(compliance_rules).expect("Failed to parse rules");
    let engine = DatalogEngine::new(rules, fact_store);
    let derived = engine.derive_facts().expect("Failed to derive facts");

    // Verify compliance checks
    let compliant: Vec<_> = derived
        .iter()
        .filter(|f| f.predicate == "compliant_access")
        .collect();
    assert!(!compliant.is_empty(), "Should have compliant accesses");

    // Check audit requirements
    let audit_required: Vec<_> = derived
        .iter()
        .filter(|f| f.predicate == "audit_entry")
        .collect();
    assert!(!audit_required.is_empty(), "Should generate audit entries");

    // Cedar policy for compliance enforcement
    let compliance_policy = r#"
        // Enforce compliance certifications
        forbid(
            principal,
            action,
            resource has "health_data"
        ) when {
            context.hipaa_certified != true
        };

        forbid(
            principal,
            action,
            resource has "payment_data"
        ) when {
            context.pci_certified != true
        };

        // Require audit logging for sensitive data
        permit(
            principal,
            action,
            resource
        ) when {
            context.compliant_access == true &&
            context.audit_logged == true &&
            context.retention_configured == true
        };

        // Block suspicious activities
        forbid(
            principal,
            action,
            resource
        ) when {
            context.suspicious_activity == true &&
            context.security_override != true
        };

        // Data retention enforcement
        permit(
            principal,
            action == Action::"delete",
            resource
        ) when {
            context.retention_expired == true &&
            context.deletion_approved == true &&
            context.audit_logged == true
        };
    "#;

    let mut policy_set = PolicySet::new();
    policy_set.load_policies(compliance_policy).expect("Failed to load policies");

    // Test: Compliant access to health data
    let compliant_request = Request::new(
        Principal::user("doctor@hospital.com"),
        Action::new("read"),
        Resource::health_data("patient_records"),
    )
    .with_context("hipaa_certified", Value::Boolean(true))
    .with_context("compliant_access", Value::Boolean(true))
    .with_context("audit_logged", Value::Boolean(true))
    .with_context("retention_configured", Value::Boolean(true));

    let result = policy_set.evaluate(&compliant_request).expect("Failed to evaluate");
    assert!(result.is_allowed(), "Compliant access should be allowed");

    // Test: Non-compliant access attempt
    let non_compliant_request = Request::new(
        Principal::user("unauthorized@user.com"),
        Action::new("read"),
        Resource::health_data("patient_records"),
    )
    .with_context("hipaa_certified", Value::Boolean(false));

    let result = policy_set.evaluate(&non_compliant_request).expect("Failed to evaluate");
    assert!(result.is_denied(), "Non-compliant access should be denied");
}