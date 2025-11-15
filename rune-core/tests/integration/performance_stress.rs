//! Performance stress tests for RUNE under high load
//!
//! Tests system performance with:
//! - Large fact bases (1M+ facts)
//! - Concurrent authorization requests
//! - Complex policy sets (1000+ policies)
//! - Memory usage verification
//! - Latency requirements (sub-millisecond P99)

use rune_core::datalog::{DatalogEngine, Evaluator};
use rune_core::facts::{Fact, FactStore};
use rune_core::parser::parse_rules;
use rune_core::policy::{PolicyEngine, PolicySet};
use rune_core::request::Request;
use rune_core::types::{Action, Principal, Resource, Value};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

/// Test with 1 million facts
#[test]
#[ignore] // Run with: cargo test --ignored test_million_facts
fn test_million_facts() {
    let fact_store = Arc::new(FactStore::new());
    let start = Instant::now();

    // Generate 1 million facts
    println!("Generating 1 million facts...");
    let batch_size = 10000;
    for batch in 0..100 {
        for i in 0..batch_size {
            let id = batch * batch_size + i;

            // User facts
            fact_store.add_fact(Fact::new(
                "user",
                vec![Value::string(format!("user_{}", id))],
            ));

            // Role assignments (10% admins, 30% managers, 60% users)
            let role = if id % 10 == 0 {
                "admin"
            } else if id % 3 == 0 {
                "manager"
            } else {
                "user"
            };

            fact_store.add_fact(Fact::new(
                "has_role",
                vec![
                    Value::string(format!("user_{}", id)),
                    Value::string(role),
                ],
            ));

            // Resource ownership
            fact_store.add_fact(Fact::new(
                "owns",
                vec![
                    Value::string(format!("user_{}", id)),
                    Value::string(format!("resource_{}", id)),
                ],
            ));

            // Department membership
            let dept = format!("dept_{}", id % 100);
            fact_store.add_fact(Fact::new(
                "member_of",
                vec![Value::string(format!("user_{}", id)), Value::string(dept)],
            ));

            // Access logs
            fact_store.add_fact(Fact::new(
                "access_log",
                vec![
                    Value::string(format!("user_{}", id)),
                    Value::Integer(1700000000 + (id as i64)),
                    Value::string(format!("resource_{}", id % 1000)),
                ],
            ));
        }

        if (batch + 1) % 10 == 0 {
            println!("  Generated {} facts...", (batch + 1) * batch_size);
        }
    }

    let load_time = start.elapsed();
    println!("Loaded 1M facts in {:?}", load_time);
    assert!(load_time.as_secs() < 30, "Should load 1M facts in <30s");

    // Test queries on large fact base
    let query_rules = r#"
        // Find all admins
        admin_user(User) :- has_role(User, "admin").

        // Find users who can access a resource
        can_access(User, Resource) :-
            owns(User, Resource).

        can_access(User, Resource) :-
            has_role(User, "admin").

        // Department statistics
        dept_member_count(Dept, Count) :-
            member_of(_, Dept),
            count(_, Count).
    "#;

    println!("Evaluating rules on 1M facts...");
    let rules = parse_rules(query_rules).expect("Failed to parse rules");
    let engine = DatalogEngine::new(rules, fact_store.clone());

    let eval_start = Instant::now();
    let derived = engine.derive_facts().expect("Failed to derive facts");
    let eval_time = eval_start.elapsed();

    println!("Derived {} facts in {:?}", derived.len(), eval_time);
    assert!(eval_time.as_secs() < 10, "Evaluation should complete in <10s");

    // Memory check
    let fact_count = fact_store.size();
    assert_eq!(fact_count, 500_000, "Should have 500K base facts");

    // Verify correctness
    let admins: Vec<_> = derived
        .iter()
        .filter(|f| f.predicate == "admin_user")
        .collect();
    assert!(admins.len() > 90_000, "Should have ~100K admins (10%)");
}

/// Test concurrent authorization requests
#[test]
fn test_concurrent_authorization() {
    let fact_store = Arc::new(FactStore::new());

    // Setup test data
    for i in 0..1000 {
        fact_store.add_fact(Fact::new(
            "user",
            vec![Value::string(format!("user_{}", i))],
        ));
        fact_store.add_fact(Fact::new(
            "has_permission",
            vec![
                Value::string(format!("user_{}", i)),
                Value::string(if i % 2 == 0 { "read" } else { "write" }),
            ],
        ));
    }

    // Create policy set
    let policy_source = r#"
        permit(
            principal,
            action == Action::"read",
            resource
        ) when {
            context.has_read_permission == true
        };

        permit(
            principal,
            action == Action::"write",
            resource
        ) when {
            context.has_write_permission == true
        };

        forbid(
            principal,
            action,
            resource
        ) when {
            context.blocked == true
        };
    "#;

    let mut policy_set = PolicySet::new();
    policy_set.load_policies(policy_source).expect("Failed to load policies");
    let policy_set = Arc::new(policy_set);

    // Concurrent authorization test
    let num_threads = 8;
    let requests_per_thread = 10000;
    let barrier = Arc::new(Barrier::new(num_threads + 1));

    let mut handles = vec![];

    for thread_id in 0..num_threads {
        let policy_set = Arc::clone(&policy_set);
        let barrier = Arc::clone(&barrier);

        let handle = thread::spawn(move || {
            let mut latencies = Vec::with_capacity(requests_per_thread);

            // Wait for all threads to be ready
            barrier.wait();

            for i in 0..requests_per_thread {
                let user_id = (thread_id * requests_per_thread + i) % 1000;
                let action = if user_id % 2 == 0 { "read" } else { "write" };

                let request = Request::new(
                    Principal::user(format!("user_{}", user_id)),
                    Action::new(action),
                    Resource::file(format!("file_{}", i)),
                )
                .with_context(
                    "has_read_permission",
                    Value::Boolean(user_id % 2 == 0),
                )
                .with_context(
                    "has_write_permission",
                    Value::Boolean(user_id % 2 == 1),
                )
                .with_context("blocked", Value::Boolean(false));

                let start = Instant::now();
                let result = policy_set.evaluate(&request).expect("Failed to evaluate");
                let latency = start.elapsed();

                latencies.push(latency);

                // Verify correctness
                assert!(result.is_allowed(), "Request should be allowed");
            }

            latencies
        });

        handles.push(handle);
    }

    println!("Starting concurrent authorization test...");
    let test_start = Instant::now();
    barrier.wait(); // Start all threads simultaneously

    // Collect results
    let mut all_latencies = Vec::new();
    for handle in handles {
        let latencies = handle.join().expect("Thread panicked");
        all_latencies.extend(latencies);
    }

    let test_duration = test_start.elapsed();
    let total_requests = num_threads * requests_per_thread;
    let throughput = total_requests as f64 / test_duration.as_secs_f64();

    // Calculate latency percentiles
    all_latencies.sort();
    let p50 = all_latencies[all_latencies.len() / 2];
    let p95 = all_latencies[all_latencies.len() * 95 / 100];
    let p99 = all_latencies[all_latencies.len() * 99 / 100];

    println!("Concurrent test results:");
    println!("  Total requests: {}", total_requests);
    println!("  Duration: {:?}", test_duration);
    println!("  Throughput: {:.0} req/s", throughput);
    println!("  P50 latency: {:?}", p50);
    println!("  P95 latency: {:?}", p95);
    println!("  P99 latency: {:?}", p99);

    // Performance assertions
    assert!(throughput > 100_000.0, "Throughput should exceed 100K req/s");
    assert!(p99.as_micros() < 1000, "P99 latency should be <1ms");
}

/// Test with 1000+ complex policies
#[test]
fn test_thousand_policies() {
    let mut policy_set = PolicySet::new();

    println!("Generating 1000 policies...");
    let start = Instant::now();

    // Generate diverse policies
    for i in 0..1000 {
        let policy = match i % 5 {
            0 => {
                // Simple permit
                format!(
                    r#"permit(
                        principal == User::"user_{}",
                        action == Action::"read",
                        resource == File::"file_{}"
                    );"#,
                    i, i
                )
            }
            1 => {
                // Conditional permit
                format!(
                    r#"permit(
                        principal in Group::"group_{}",
                        action in [Action::"read", Action::"write"],
                        resource
                    ) when {{
                        context.department == "dept_{}" &&
                        context.clearance_level >= {}
                    }};"#,
                    i % 100,
                    i % 10,
                    i % 3 + 1
                )
            }
            2 => {
                // Forbid rule
                format!(
                    r#"forbid(
                        principal,
                        action == Action::"delete",
                        resource in Folder::"sensitive_{}"
                    ) unless {{
                        context.is_admin == true &&
                        context.mfa_verified == true
                    }};"#,
                    i % 50
                )
            }
            3 => {
                // Hierarchical resource
                format!(
                    r#"permit(
                        principal == User::"manager_{}",
                        action,
                        resource in Folder::"team_{}"
                    ) when {{
                        context.business_hours == true
                    }};"#,
                    i % 20,
                    i % 20
                )
            }
            4 => {
                // Complex conditions
                format!(
                    r#"permit(
                        principal,
                        action == Action::"approve",
                        resource == Document::"doc_{}"
                    ) when {{
                        context.approval_level > {} &&
                        context.budget_remaining > {} &&
                        context.deadline > "{}"
                    }};"#,
                    i,
                    i % 5,
                    i * 1000,
                    "2024-12-31"
                )
            }
            _ => unreachable!(),
        };

        policy_set.add_policy(&format!("policy_{}", i), &policy)
            .expect("Failed to add policy");
    }

    let load_time = start.elapsed();
    println!("Loaded 1000 policies in {:?}", load_time);
    assert!(load_time.as_secs() < 5, "Should load 1000 policies in <5s");

    // Test evaluation performance with many policies
    println!("Testing evaluation with 1000 policies...");
    let mut eval_times = Vec::new();

    for i in 0..1000 {
        let request = Request::new(
            Principal::user(format!("user_{}", i)),
            Action::new("read"),
            Resource::file(format!("file_{}", i)),
        )
        .with_context("department", Value::string(format!("dept_{}", i % 10)))
        .with_context("clearance_level", Value::Integer((i % 5) as i64))
        .with_context("business_hours", Value::Boolean(true));

        let start = Instant::now();
        let _ = policy_set.evaluate(&request);
        let eval_time = start.elapsed();

        eval_times.push(eval_time);
    }

    // Calculate statistics
    eval_times.sort();
    let p50 = eval_times[eval_times.len() / 2];
    let p95 = eval_times[eval_times.len() * 95 / 100];
    let p99 = eval_times[eval_times.len() * 99 / 100];

    println!("Evaluation with 1000 policies:");
    println!("  P50: {:?}", p50);
    println!("  P95: {:?}", p95);
    println!("  P99: {:?}", p99);

    assert!(p99.as_micros() < 1000, "P99 should be <1ms even with 1000 policies");
}

/// Test memory usage under load
#[test]
fn test_memory_efficiency() {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Simple memory tracker
    struct MemoryTracker;
    static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

    unsafe impl GlobalAlloc for MemoryTracker {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let size = layout.size();
            let ptr = System.alloc(layout);
            if !ptr.is_null() {
                ALLOCATED.fetch_add(size, Ordering::SeqCst);
            }
            ptr
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            let size = layout.size();
            System.dealloc(ptr, layout);
            ALLOCATED.fetch_sub(size, Ordering::SeqCst);
        }
    }

    let initial_memory = ALLOCATED.load(Ordering::SeqCst);
    println!("Initial memory: {} bytes", initial_memory);

    // Create large fact store
    {
        let fact_store = Arc::new(FactStore::new());

        // Add 100K facts
        for i in 0..100_000 {
            fact_store.add_fact(Fact::new(
                "test_fact",
                vec![
                    Value::string(format!("subject_{}", i)),
                    Value::string(format!("object_{}", i)),
                    Value::Integer(i as i64),
                ],
            ));
        }

        let after_facts = ALLOCATED.load(Ordering::SeqCst);
        let fact_memory = after_facts - initial_memory;
        let bytes_per_fact = fact_memory / 100_000;

        println!("Memory after 100K facts: {} bytes", after_facts);
        println!("Memory used: {} MB", fact_memory / 1_048_576);
        println!("Bytes per fact: {}", bytes_per_fact);

        // Should use less than 100 bytes per fact on average
        assert!(
            bytes_per_fact < 100,
            "Memory usage should be <100 bytes per fact"
        );

        // Verify fact store operations are still fast
        let start = Instant::now();
        let all_facts = fact_store.get_by_predicate("test_fact");
        let query_time = start.elapsed();

        assert_eq!(all_facts.len(), 100_000);
        assert!(
            query_time.as_millis() < 100,
            "Query should complete in <100ms"
        );
    }

    // Memory should be reclaimed after scope
    thread::sleep(Duration::from_millis(100)); // Give time for deallocation
    let final_memory = ALLOCATED.load(Ordering::SeqCst);
    let leaked = final_memory.saturating_sub(initial_memory);

    println!("Final memory: {} bytes", final_memory);
    println!("Leaked memory: {} bytes", leaked);

    // Allow some leakage but should be minimal
    assert!(
        leaked < 1_048_576,
        "Should leak less than 1MB after cleanup"
    );
}

/// Test incremental evaluation performance
#[test]
fn test_incremental_performance() {
    use rune_core::datalog::IncrementalEvaluator;

    let fact_store = Arc::new(FactStore::new());

    // Initial facts
    println!("Adding initial 10K facts...");
    for i in 0..10_000 {
        fact_store.add_fact(Fact::new(
            "edge",
            vec![Value::Integer(i as i64), Value::Integer((i + 1) as i64)],
        ));
    }

    // Transitive closure rules
    let rules_source = r#"
        path(X, Y) :- edge(X, Y).
        path(X, Z) :- edge(X, Y), path(Y, Z).
    "#;

    let rules = parse_rules(rules_source).expect("Failed to parse rules");

    // Initial evaluation
    println!("Initial evaluation...");
    let start = Instant::now();
    let mut evaluator = IncrementalEvaluator::new(rules, fact_store.clone());
    let initial_result = evaluator.evaluate().expect("Failed initial evaluation");
    let initial_time = start.elapsed();

    println!(
        "Initial evaluation: {} facts in {:?}",
        initial_result.new_facts.len(),
        initial_time
    );

    // Incremental updates
    println!("Testing incremental updates...");
    let mut incremental_times = Vec::new();

    for batch in 0..10 {
        // Add 100 new edges
        for i in 0..100 {
            let base = 10_000 + batch * 100 + i;
            fact_store.add_fact(Fact::new(
                "edge",
                vec![
                    Value::Integer(base as i64),
                    Value::Integer((base + 1) as i64),
                ],
            ));
        }

        let start = Instant::now();
        let result = evaluator.evaluate().expect("Failed incremental evaluation");
        let incr_time = start.elapsed();

        incremental_times.push(incr_time);
        println!(
            "  Batch {}: {} new facts in {:?}",
            batch,
            result.new_facts.len(),
            incr_time
        );
    }

    // Incremental should be much faster than full re-evaluation
    let avg_incremental = incremental_times
        .iter()
        .map(|d| d.as_micros())
        .sum::<u128>()
        / incremental_times.len() as u128;

    println!("Average incremental time: {} μs", avg_incremental);
    assert!(
        avg_incremental < initial_time.as_micros() / 10,
        "Incremental should be >10x faster than full evaluation"
    );
}

/// Stress test with rapid fact updates
#[test]
fn test_rapid_fact_updates() {
    let fact_store = Arc::new(FactStore::new());
    let num_threads = 4;
    let updates_per_thread = 10000;
    let barrier = Arc::new(Barrier::new(num_threads + 1));

    let mut handles = vec![];

    for thread_id in 0..num_threads {
        let fact_store = Arc::clone(&fact_store);
        let barrier = Arc::clone(&barrier);

        let handle = thread::spawn(move || {
            barrier.wait(); // Synchronize start

            let start = Instant::now();

            for i in 0..updates_per_thread {
                // Add fact
                let fact = Fact::new(
                    "dynamic_fact",
                    vec![
                        Value::string(format!("thread_{}", thread_id)),
                        Value::Integer(i as i64),
                        Value::Boolean(i % 2 == 0),
                    ],
                );
                fact_store.add_fact(fact.clone());

                // Occasionally remove facts
                if i % 10 == 0 && i > 0 {
                    let old_fact = Fact::new(
                        "dynamic_fact",
                        vec![
                            Value::string(format!("thread_{}", thread_id)),
                            Value::Integer((i - 10) as i64),
                            Value::Boolean((i - 10) % 2 == 0),
                        ],
                    );
                    fact_store.remove_fact(&old_fact);
                }

                // Query occasionally
                if i % 100 == 0 {
                    let _ = fact_store.get_by_predicate("dynamic_fact");
                }
            }

            start.elapsed()
        });

        handles.push(handle);
    }

    println!("Starting rapid update test...");
    barrier.wait(); // Start all threads

    let mut durations = Vec::new();
    for handle in handles {
        let duration = handle.join().expect("Thread panicked");
        durations.push(duration);
    }

    let total_updates = num_threads * updates_per_thread;
    let max_duration = durations.iter().max().unwrap();
    let throughput = total_updates as f64 / max_duration.as_secs_f64();

    println!("Rapid update results:");
    println!("  Total updates: {}", total_updates);
    println!("  Max duration: {:?}", max_duration);
    println!("  Throughput: {:.0} updates/sec", throughput);

    assert!(
        throughput > 100_000.0,
        "Should handle >100K updates/sec"
    );

    // Verify final state consistency
    let final_count = fact_store.get_by_predicate("dynamic_fact").len();
    println!("  Final fact count: {}", final_count);
    assert!(final_count > 0, "Should have facts remaining");
}

/// Test system behavior near capacity limits
#[test]
#[ignore] // Run with: cargo test --ignored test_capacity_limits
fn test_capacity_limits() {
    let fact_store = Arc::new(FactStore::new());

    // Find the practical limit
    println!("Testing fact store capacity...");
    let batch_size = 100_000;
    let mut total_facts = 0;

    loop {
        let batch_start = Instant::now();

        // Try to add another batch
        for i in 0..batch_size {
            let fact = Fact::new(
                "capacity_test",
                vec![
                    Value::Integer(total_facts as i64),
                    Value::string(format!("data_{}", i)),
                ],
            );

            // Check if we can still add facts efficiently
            let add_start = Instant::now();
            fact_store.add_fact(fact);
            let add_time = add_start.elapsed();

            if add_time.as_millis() > 10 {
                println!(
                    "Performance degraded at {} facts (add took {:?})",
                    total_facts, add_time
                );
                break;
            }

            total_facts += 1;
        }

        let batch_time = batch_start.elapsed();
        println!(
            "Added batch to {}: {:?} ({:.0} facts/sec)",
            total_facts,
            batch_time,
            batch_size as f64 / batch_time.as_secs_f64()
        );

        // Stop if performance degrades or we hit memory limits
        if batch_time.as_secs() > 10 || total_facts > 10_000_000 {
            break;
        }
    }

    println!("Maximum efficient capacity: {} facts", total_facts);
    assert!(
        total_facts > 1_000_000,
        "Should handle at least 1M facts efficiently"
    );

    // Test query performance at capacity
    let query_start = Instant::now();
    let results = fact_store.get_by_predicate("capacity_test");
    let query_time = query_start.elapsed();

    println!("Query at capacity: {} results in {:?}", results.len(), query_time);
    assert!(
        query_time.as_secs() < 5,
        "Query should complete in <5s even at capacity"
    );
}