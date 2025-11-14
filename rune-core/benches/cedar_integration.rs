//! Benchmarks for Cedar Policy integration
//!
//! Tests the performance of RUNE's Cedar integration including:
//! - Policy parsing and loading
//! - Request authorization
//! - Entity creation
//! - Request conversion
//! - Multi-policy evaluation
//!
//! Performance targets:
//! - P99 latency: <1ms per authorization
//! - Throughput: 10K+ authz/sec
//! - Policy loading: <10ms for 100 policies

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rune_core::policy::PolicySet;
use rune_core::request::Request;
use rune_core::types::{Action, Principal, Resource};

/// Generate a simple allow policy
fn generate_simple_policy(id: usize) -> String {
    format!(
        r#"permit(
            principal == User::"user{}",
            action == Action::"read",
            resource == File::"file{}"
        );"#,
        id, id
    )
}

/// Generate a policy with conditions
fn generate_conditional_policy(id: usize) -> String {
    format!(
        r#"permit(
            principal == User::"user{}",
            action == Action::"read",
            resource == File::"file{}"
        ) when {{
            context.timestamp > 1000 &&
            context.source_ip == "192.168.1.{}"
        }};"#,
        id, id, id % 256
    )
}

/// Generate a policy with resource hierarchy
fn generate_hierarchical_policy(id: usize) -> String {
    format!(
        r#"permit(
            principal == User::"user{}",
            action == Action::"read",
            resource in Folder::"folder{}"
        );"#,
        id, id / 10
    )
}

/// Create a test request
fn create_test_request(user_id: usize, action: &str, file_id: usize) -> Request {
    Request::new(
        Principal::user(format!("user{}", user_id)),
        Action::new(action),
        Resource::file(format!("file{}", file_id)),
    )
}

/// Benchmark policy parsing and loading
fn bench_policy_loading(c: &mut Criterion) {
    let mut group = c.benchmark_group("cedar/policy_loading");

    for num_policies in [1, 10, 50, 100].iter() {
        group.throughput(Throughput::Elements(*num_policies as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(num_policies),
            num_policies,
            |b, &num_policies| {
                let policies: Vec<String> = (0..num_policies)
                    .map(|i| generate_simple_policy(i))
                    .collect();
                let policy_str = policies.join("\n\n");

                b.iter(|| {
                    let mut policy_set = PolicySet::new();
                    policy_set.load_policies(&policy_str).unwrap();
                    black_box(policy_set)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark single authorization request
fn bench_single_authorization(c: &mut Criterion) {
    let mut group = c.benchmark_group("cedar/single_authorization");

    // Test different policy types
    let policy_types = vec![
        ("simple", generate_simple_policy(0)),
        ("conditional", generate_conditional_policy(0)),
        ("hierarchical", generate_hierarchical_policy(0)),
    ];

    for (name, policy) in policy_types {
        group.bench_with_input(BenchmarkId::new("policy", name), &policy, |b, policy| {
            let mut policy_set = PolicySet::new();
            policy_set.load_policies(policy).unwrap();
            let request = create_test_request(0, "read", 0);

            b.iter(|| {
                let result = policy_set.evaluate(&request);
                black_box(result)
            });
        });
    }

    group.finish();
}

/// Benchmark authorization with multiple policies
fn bench_multi_policy_authorization(c: &mut Criterion) {
    let mut group = c.benchmark_group("cedar/multi_policy");

    for num_policies in [1, 10, 50, 100].iter() {
        group.throughput(Throughput::Elements(*num_policies as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(num_policies),
            num_policies,
            |b, &num_policies| {
                let policies: Vec<String> = (0..num_policies)
                    .map(|i| generate_simple_policy(i))
                    .collect();
                let policy_str = policies.join("\n\n");

                let mut policy_set = PolicySet::new();
                policy_set.load_policies(&policy_str).unwrap();
                let request = create_test_request(0, "read", 0);

                b.iter(|| {
                    let result = policy_set.evaluate(&request);
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark batch authorization (multiple requests)
fn bench_batch_authorization(c: &mut Criterion) {
    let mut group = c.benchmark_group("cedar/batch_authorization");

    for batch_size in [10, 50, 100, 500].iter() {
        group.throughput(Throughput::Elements(*batch_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            batch_size,
            |b, &batch_size| {
                // Create policy set
                let policies: Vec<String> = (0..100).map(|i| generate_simple_policy(i)).collect();
                let policy_str = policies.join("\n\n");

                let mut policy_set = PolicySet::new();
                policy_set.load_policies(&policy_str).unwrap();

                // Create batch of requests
                let requests: Vec<Request> = (0..batch_size)
                    .map(|i| create_test_request(i % 100, "read", i % 100))
                    .collect();

                b.iter(|| {
                    for request in &requests {
                        let _ = policy_set.evaluate(request);
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark request cache key generation
fn bench_request_cache_key(c: &mut Criterion) {
    let mut group = c.benchmark_group("cedar/cache_key");

    let requests = vec![
        ("simple", create_test_request(0, "read", 0)),
        (
            "complex",
            create_test_request(123, "write", 456)
                .with_context("timestamp", rune_core::types::Value::Integer(1234567890))
                .with_context("ip", rune_core::types::Value::string("192.168.1.1")),
        ),
    ];

    for (name, request) in requests {
        group.bench_with_input(BenchmarkId::new("request", name), &request, |b, request| {
            b.iter(|| {
                let key = request.cache_key();
                black_box(key)
            });
        });
    }

    group.finish();
}

/// Benchmark authorization with deny policies
fn bench_deny_policies(c: &mut Criterion) {
    let mut group = c.benchmark_group("cedar/deny_policies");

    // Mix of permit and forbid policies
    let policies = vec![
        r#"permit(principal, action == Action::"read", resource);"#,
        r#"forbid(principal == User::"blocked", action, resource);"#,
        r#"forbid(principal, action == Action::"delete", resource in Folder::"protected");"#,
    ]
    .join("\n\n");

    let mut policy_set = PolicySet::new();
    policy_set.load_policies(&policies).unwrap();

    // Different request scenarios
    let scenarios = vec![
        ("allowed", create_test_request(1, "read", 1)),
        ("blocked_user", create_test_request(0, "read", 1)), // Assumes "blocked" user
        ("protected_delete", create_test_request(1, "delete", 1)),
    ];

    for (name, request) in scenarios {
        group.bench_with_input(BenchmarkId::new("scenario", name), &request, |b, request| {
            b.iter(|| {
                let result = policy_set.evaluate(request);
                black_box(result)
            });
        });
    }

    group.finish();
}

/// Benchmark incremental policy additions
fn bench_incremental_policy_addition(c: &mut Criterion) {
    let mut group = c.benchmark_group("cedar/incremental_addition");

    for batch_size in [1, 5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            batch_size,
            |b, &batch_size| {
                b.iter(|| {
                    let mut policy_set = PolicySet::new();

                    // Add policies one by one
                    for i in 0..batch_size {
                        let policy = generate_simple_policy(i);
                        policy_set.add_policy(&format!("policy{}", i), &policy).unwrap();
                    }

                    black_box(policy_set)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_policy_loading,
    bench_single_authorization,
    bench_multi_policy_authorization,
    bench_batch_authorization,
    bench_request_cache_key,
    bench_deny_policies,
    bench_incremental_policy_addition
);
criterion_main!(benches);
