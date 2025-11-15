//! Benchmarks for Datalog evaluation performance
//!
//! Tests the performance of RUNE's Datalog engine including:
//! - Semi-naive evaluation
//! - Incremental evaluation
//! - Query planning
//! - Unification
//!
//! Performance targets:
//! - P99 latency: <1ms
//! - Throughput: 100K+ ops/sec

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rune_core::datalog::{Atom, Evaluator, IncrementalEvaluator, QueryPlanner, Rule, Term};
use rune_core::facts::{Fact, FactStore};
use rune_core::types::Value;
use std::sync::Arc;

/// Generate a chain of edge facts for transitive closure testing
fn generate_edge_facts(n: usize) -> Vec<Fact> {
    (0..n)
        .map(|i| {
            Fact::new(
                "edge",
                vec![Value::Integer(i as i64), Value::Integer((i + 1) as i64)],
            )
        })
        .collect()
}

/// Generate a complete graph for stress testing
fn generate_complete_graph(n: usize) -> Vec<Fact> {
    let mut facts = Vec::new();
    for i in 0..n {
        for j in 0..n {
            if i != j {
                facts.push(Fact::new(
                    "edge",
                    vec![Value::Integer(i as i64), Value::Integer(j as i64)],
                ));
            }
        }
    }
    facts
}

/// Generate hierarchical facts (parent-child relationships)
fn generate_hierarchy(depth: usize, fanout: usize) -> Vec<Fact> {
    let mut facts = Vec::new();
    let mut id = 0i64;

    fn add_level(
        facts: &mut Vec<Fact>,
        parent_id: i64,
        depth: usize,
        fanout: usize,
        next_id: &mut i64,
    ) {
        if depth == 0 {
            return;
        }

        for _ in 0..fanout {
            *next_id += 1;
            let child_id = *next_id;
            facts.push(Fact::new(
                "parent",
                vec![Value::Integer(parent_id), Value::Integer(child_id)],
            ));
            add_level(facts, child_id, depth - 1, fanout, next_id);
        }
    }

    add_level(&mut facts, id, depth, fanout, &mut id);
    facts
}

/// Create transitive closure rules
fn create_transitive_closure_rules() -> Vec<Rule> {
    vec![
        // Base case: path(X, Y) :- edge(X, Y).
        Rule {
            head: Atom {
                predicate: Arc::from("path"),
                terms: vec![Term::Variable("X".into()), Term::Variable("Y".into())],
                negated: false,
            },
            body: vec![Atom {
                predicate: Arc::from("edge"),
                terms: vec![Term::Variable("X".into()), Term::Variable("Y".into())],
                negated: false,
            }],
            stratum: 0,
        },
        // Recursive case: path(X, Z) :- edge(X, Y), path(Y, Z).
        Rule {
            head: Atom {
                predicate: Arc::from("path"),
                terms: vec![Term::Variable("X".into()), Term::Variable("Z".into())],
                negated: false,
            },
            body: vec![
                Atom {
                    predicate: Arc::from("edge"),
                    terms: vec![Term::Variable("X".into()), Term::Variable("Y".into())],
                    negated: false,
                },
                Atom {
                    predicate: Arc::from("path"),
                    terms: vec![Term::Variable("Y".into()), Term::Variable("Z".into())],
                    negated: false,
                },
            ],
            stratum: 0,
        },
    ]
}

/// Create ancestor rules for hierarchy
fn create_ancestor_rules() -> Vec<Rule> {
    vec![
        // Base case: ancestor(X, Y) :- parent(X, Y).
        Rule {
            head: Atom {
                predicate: Arc::from("ancestor"),
                terms: vec![Term::Variable("X".into()), Term::Variable("Y".into())],
                negated: false,
            },
            body: vec![Atom {
                predicate: Arc::from("parent"),
                terms: vec![Term::Variable("X".into()), Term::Variable("Y".into())],
                negated: false,
            }],
            stratum: 0,
        },
        // Recursive case: ancestor(X, Z) :- parent(X, Y), ancestor(Y, Z).
        Rule {
            head: Atom {
                predicate: Arc::from("ancestor"),
                terms: vec![Term::Variable("X".into()), Term::Variable("Z".into())],
                negated: false,
            },
            body: vec![
                Atom {
                    predicate: Arc::from("parent"),
                    terms: vec![Term::Variable("X".into()), Term::Variable("Y".into())],
                    negated: false,
                },
                Atom {
                    predicate: Arc::from("ancestor"),
                    terms: vec![Term::Variable("Y".into()), Term::Variable("Z".into())],
                    negated: false,
                },
            ],
            stratum: 0,
        },
    ]
}

/// Benchmark semi-naive evaluation on transitive closure
fn bench_transitive_closure(c: &mut Criterion) {
    let mut group = c.benchmark_group("datalog/transitive_closure");

    for size in [10, 50, 100, 500].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let facts = generate_edge_facts(size);
            let fact_store = Arc::new(FactStore::new());
            for fact in facts {
                fact_store.add_fact(fact);
            }

            let rules = create_transitive_closure_rules();

            b.iter(|| {
                let evaluator = Evaluator::new(rules.clone(), fact_store.clone());
                let result = evaluator.evaluate();
                black_box(result)
            });
        });
    }

    group.finish();
}

/// Benchmark evaluation on complete graphs (stress test)
fn bench_complete_graph(c: &mut Criterion) {
    let mut group = c.benchmark_group("datalog/complete_graph");

    for size in [5, 10, 20, 30].iter() {
        group.throughput(Throughput::Elements((*size * (*size - 1)) as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let facts = generate_complete_graph(size);
            let fact_store = Arc::new(FactStore::new());
            for fact in facts {
                fact_store.add_fact(fact);
            }

            let rules = create_transitive_closure_rules();

            b.iter(|| {
                let evaluator = Evaluator::new(rules.clone(), fact_store.clone());
                let result = evaluator.evaluate();
                black_box(result)
            });
        });
    }

    group.finish();
}

/// Benchmark hierarchical queries
fn bench_hierarchy(c: &mut Criterion) {
    let mut group = c.benchmark_group("datalog/hierarchy");

    // Test different hierarchy shapes
    let configurations = vec![
        (3, 3, "narrow"), // depth 3, fanout 3
        (2, 10, "wide"),  // depth 2, fanout 10
        (5, 2, "deep"),   // depth 5, fanout 2
    ];

    for (depth, fanout, name) in configurations {
        group.bench_with_input(
            BenchmarkId::new("ancestor", name),
            &(depth, fanout),
            |b, &(depth, fanout)| {
                let facts = generate_hierarchy(depth, fanout);
                let fact_store = Arc::new(FactStore::new());
                for fact in facts {
                    fact_store.add_fact(fact);
                }

                let rules = create_ancestor_rules();

                b.iter(|| {
                    let evaluator = Evaluator::new(rules.clone(), fact_store.clone());
                    let result = evaluator.evaluate();
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark incremental evaluation
fn bench_incremental(c: &mut Criterion) {
    let mut group = c.benchmark_group("datalog/incremental");

    for batch_size in [1, 10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            batch_size,
            |b, &batch_size| {
                // Start with initial facts
                let initial_facts = generate_edge_facts(100);
                let fact_store = Arc::new(FactStore::new());
                for fact in initial_facts {
                    fact_store.add_fact(fact);
                }

                let rules = create_transitive_closure_rules();
                let mut evaluator = IncrementalEvaluator::new(rules, fact_store.clone());

                // Initial evaluation
                let _ = evaluator.evaluate();

                b.iter(|| {
                    // Add new facts incrementally
                    for i in 0..batch_size {
                        let fact = Fact::new(
                            "edge",
                            vec![
                                Value::Integer(1000 + i as i64),
                                Value::Integer(1001 + i as i64),
                            ],
                        );
                        fact_store.add_fact(fact);
                    }

                    let result = evaluator.evaluate();
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark query planning
fn bench_query_planning(c: &mut Criterion) {
    let mut group = c.benchmark_group("datalog/query_planning");

    // Different rule patterns for testing the planner
    let test_rules = vec![
        (
            "simple",
            vec![
                // Single-atom rule
                Rule {
                    head: Atom {
                        predicate: Arc::from("result"),
                        terms: vec![Term::Variable("X".into()), Term::Variable("Y".into())],
                        negated: false,
                    },
                    body: vec![Atom {
                        predicate: Arc::from("edge"),
                        terms: vec![Term::Variable("X".into()), Term::Variable("Y".into())],
                        negated: false,
                    }],
                    stratum: 0,
                },
            ],
        ),
        (
            "join",
            vec![
                // Two-way join
                Rule {
                    head: Atom {
                        predicate: Arc::from("result"),
                        terms: vec![Term::Variable("X".into()), Term::Variable("Z".into())],
                        negated: false,
                    },
                    body: vec![
                        Atom {
                            predicate: Arc::from("edge"),
                            terms: vec![Term::Variable("X".into()), Term::Variable("Y".into())],
                            negated: false,
                        },
                        Atom {
                            predicate: Arc::from("edge"),
                            terms: vec![Term::Variable("Y".into()), Term::Variable("Z".into())],
                            negated: false,
                        },
                    ],
                    stratum: 0,
                },
            ],
        ),
        (
            "triangle",
            vec![
                // Triangle pattern (good for WCOJ)
                Rule {
                    head: Atom {
                        predicate: Arc::from("result"),
                        terms: vec![
                            Term::Variable("X".into()),
                            Term::Variable("Y".into()),
                            Term::Variable("Z".into()),
                        ],
                        negated: false,
                    },
                    body: vec![
                        Atom {
                            predicate: Arc::from("edge"),
                            terms: vec![Term::Variable("X".into()), Term::Variable("Y".into())],
                            negated: false,
                        },
                        Atom {
                            predicate: Arc::from("edge"),
                            terms: vec![Term::Variable("Y".into()), Term::Variable("Z".into())],
                            negated: false,
                        },
                        Atom {
                            predicate: Arc::from("edge"),
                            terms: vec![Term::Variable("Z".into()), Term::Variable("X".into())],
                            negated: false,
                        },
                    ],
                    stratum: 0,
                },
            ],
        ),
    ];

    for (name, rules) in test_rules {
        group.bench_with_input(BenchmarkId::new("plan", name), &rules, |b, rules| {
            let fact_store = Arc::new(FactStore::new());
            // Add some facts for statistics
            for fact in generate_edge_facts(100) {
                fact_store.add_fact(fact);
            }

            let planner = QueryPlanner::new(fact_store);

            b.iter(|| {
                let plan = planner.plan_rule(&rules[0]);
                black_box(plan)
            });
        });
    }

    group.finish();
}

/// Benchmark fact insertion performance
fn bench_fact_insertion(c: &mut Criterion) {
    let mut group = c.benchmark_group("datalog/fact_insertion");

    for size in [100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let facts = generate_edge_facts(size);

            b.iter(|| {
                let fact_store = FactStore::new();
                for fact in &facts {
                    fact_store.add_fact(fact.clone());
                }
                black_box(fact_store)
            });
        });
    }

    group.finish();
}

/// Benchmark fact lookup performance
fn bench_fact_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("datalog/fact_lookup");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let fact_store = FactStore::new();
            for fact in generate_edge_facts(size) {
                fact_store.add_fact(fact);
            }

            b.iter(|| {
                // Lookup facts by predicate
                let facts = fact_store.get_by_predicate("edge");
                black_box(facts.len())
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_transitive_closure,
    bench_complete_graph,
    bench_hierarchy,
    bench_incremental,
    bench_query_planning,
    bench_fact_insertion,
    bench_fact_lookup
);
criterion_main!(benches);
