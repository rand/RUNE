// RUNE Sidebar - Project-specific comments with shared foundation
(function() {
    // Section-specific comments for index page
    const indexComments = {
        'abstract': '// Autonomous agents need safe boundaries',
        'key-features': '// <1ms latency • 5M+ ops/sec',
        'architecture': '// Dual-engine: Datalog + Cedar',
        'use-cases': '// Production workflows & integration',
        'getting-started': '// Quick start guide'
    };

    // Section-specific comments for whitepaper (h2 sections with dry humor)
    const whitepaperComments = {
        'abstract': "// Because 'just trust the AI' isn't a security model",
        'table-of-contents': '// Your roadmap to safe AI autonomy',
        '1-introduction': '// The problem: agents with root access to your life',
        '2-background-and-motivation': "// Why your RBAC can't handle AI chaos",
        '3-system-design': '// Where philosophy meets performance',
        '4-architecture': '// Two engines, zero regrets',
        '5-implementation': '// Rust go brrrr (lock-free edition)',
        '6-performance-evaluation': '// 5M ops/sec or we riot',
        '7-workflows-and-use-cases': '// Real world > toy examples',
        '8-lessons-learned': '// What we got wrong (and right)',
        '9-related-work': '// Standing on the shoulders of giants',
        '10-future-work': '// TODO: conquer the world',
        '11-conclusion': '// TL;DR with gravitas',
        'references': '// Citations for the academically inclined'
    };

    // Subsection commentary for whitepaper (h3 sections - more granular)
    const whitepaperSubsections = {
        '11-the-agent-authorization-problem': '// Tier 3 agents: file access + shell + APIs = chaos',
        '12-the-rune-approach': '// Datalog for config, Cedar for authz, <1ms decisions',
        '13-design-goals': '// Performance + Expressiveness + Deployability + Safety',
        '14-document-structure': '// 11 sections of authorization goodness',
        '21-the-evolution-of-ai-agents': '// From sandboxed toys to production autonomy',
        '22-authorization-challenges': '// 59 auth checks per feature = 5.9s overhead',
        '23-existing-approaches-and-limitations': '// RBAC too coarse, ABAC too slow, OPA too heavy',
        '24-why-a-dual-engine-architecture': '// Authorization ≠ Configuration (different models)',
        '31-core-concepts': '// Facts, Rules, Policies, Requests, Decisions',
        '32-design-principles': '// Lock-free • Zero-copy • Fail-fast • Observable',
        '33-the-rune-configuration-format': '// TOML + Datalog + Cedar in one file',
        '34-request-flow': '// Cache check → Parallel engines → Merge results',
        '35-caching-strategy': '// DashMap + TTL = 90%+ hit rate, 100x speedup',
        '41-system-architecture': '// Engine + Datalog + Cedar + FactStore',
        '42-lock-free-fact-store': '// Crossbeam epoch + Arc = zero-copy reads',
        '43-parallel-dual-engine-evaluation': '// Rayon work-stealing for true parallelism',
        '44-cedar-integration': '// Cedar 3.x entity ownership: collect first, create once',
        '45-memory-management': '// Arc for sharing, epoch for reclamation, <50MB for 1M facts',
        '51-technology-stack': '// Rust 1.75+ • cedar-policy 3.1 • crossbeam 0.8',
        '52-parser-implementation': '// TOML data + Datalog rules + Cedar policies',
        '53-datalog-engine-future': '// Semi-naive evaluation with stratified negation',
        '54-cli-implementation': '// eval, validate, benchmark, serve (future)',
        '55-error-handling': '// Result<T,E> everywhere, thiserror for DX',
        '61-benchmark-methodology': '// M1 8-core, 1K requests, 4 threads, randomized workload',
        '62-throughput-results': '// 5,080,423 req/sec • 50x better than goal',
        '63-cache-performance': '// 90.9% hit rate • 5x throughput improvement',
        '64-scalability': '// Linear thread scaling • Sublinear latency growth',
        '65-comparison-to-opa': '// 240,000x faster • 600x more throughput',
        '66-production-readiness': '// Zero panics in 10M+ evals • <10ms startup',
        '71-ai-agent-file-access': '// Read source, write output, no secrets, rate limited',
        '72-api-rate-limiting': '// Per-provider limits with burst allowance',
        '73-multi-environment-configuration': '// Dev permissive, staging safe, prod locked down',
        '74-human-in-the-loop-break-glass': '// Emergency access with human approval',
        '81-lock-free-data-structures-are-worth-it': '// RwLock P99: 5ms → Epoch P99: <1ms',
        '82-cedar-integration-requires-care': '// Entity ownership: batch transformations win',
        '83-caching-makes-or-breaks-performance': '// Simple TTL cache = 5x throughput',
        '84-dont-prematurely-optimize': '// Profile first, optimize second',
        '85-error-messages-matter': '// Context (policy ID, line, entity) = 10x faster debug',
        '86-the-power-of-rusts-type-system': '// Send + Sync caught races at compile time',
        '91-authorization-systems': '// Zanzibar, IAM/Cedar, OPA, OSO/Polar',
        '92-logic-programming-systems': '// Datalog, Prolog, Soufflé',
        '93-agent-frameworks': '// LangChain, AutoGPT, Semantic Kernel integration',
        '101-full-datalog-implementation': '// Semi-naive + stratification + aggregates',
        '102-hot-reload-with-rcu': '// Zero-downtime policy updates via RCU',
        '103-python-bindings': '// PyO3 FFI • GIL-free eval • @authorize decorator',
        '104-comprehensive-benchmarks': '// P50/P90/P99/P99.9 latencies + flamegraphs',
        '105-observability': '// Prometheus metrics • OpenTelemetry traces',
        '106-schema-validation': '// Auto-generate Cedar schemas from [data]'
    };

    // Agent guide section comments (h2 sections)
    const agentGuideComments = {
        'table-of-contents': '// Navigation for the organized mind',
        'quick-start': '// Test after commit, never commit to main',
        'project-overview': '// Dual-engine authz at 5M+ ops/sec',
        'repository-structure': '// rune-core + rune-cli + rune-python workspace',
        'development-workflow': '// Feature branches • Commit before test • Quality gates',
        'documentation-protocols': '// When and how to update docs',
        'release-management': '// SemVer 2.0 • Tagged validation • GitHub releases',
        'repository-organization': '// Non-destructive tidying with git mv',
        'testing-protocols': '// RULE 1: Commit before testing. RULE 2: See rule 1',
        'performance-requirements': '// P99 <1ms • 100K+ req/sec • <100MB memory',
        'common-tasks': '// Recipes for adding rules, policies, benchmarks, docs',
        'integration-with-mnemosyne': '// Store decisions, recall context, track TODOs',
        'troubleshooting': '// Common build errors and debugging strategies',
        'references': '// Cedar docs, Crossbeam, Rust perf book, SemVer'
    };

    // Agent guide subsection comments (h3 sections)
    const agentGuideSubsections = {
        'what-is-rune': '// Dual-engine authz: what + how agents act',
        'core-value-proposition': '// <1ms decisions • 5M+ ops/sec • 10MB binary',
        'architecture': '// Lock-free • Zero-copy • Parallel eval • DashMap cache',
        'key-files': '// README, WHITEPAPER, CHANGELOG, CLAUDE.md, CONTRIBUTING.md',
        'branching-strategy': '// feature/, fix/, refactor/, docs/, perf/',
        'commit-protocol': '// Commit → Test → Fix → Commit → Re-test',
        'pull-requests': '// Tests pass • No perf regression • Docs updated',
        'when-to-update-documentation': '// New feature, API change, architecture change, perf improvement',
        'documentation-standards': '// Keep README current, validate whitepaper claims',
        'code-reference-format': '// Link to tagged versions for verification',
        'semantic-versioning': '// 0.x.y dev • 1.0.0 stable • SemVer 2.0',
        'release-process': '// Version bump → Tag → Build → GitHub release',
        'special-tags': '// v0.1.0-whitepaper for validation',
        'organization-principles': '// Non-destructive • Reference-preserving • Context-efficient',
        'tidying-guidelines': '// git mv preserves history, update cross-refs',
        'adding-new-components': '// Add to workspace, update docs, add tests',
        'critical-testing-rules': '// Commit BEFORE testing, NEVER test uncommitted',
        'testing-workflow': '// Commit → Kill old tests → Run tests → Fix if needed',
        'test-types': '// Unit, integration, benchmarks, CLI, full suite',
        'performance-testing': '// Benchmark on every perf-sensitive change',
        'test-coverage': '// Critical 90%+ • Authz 95%+ • Parser 80%+ • Overall 85%+',
        'hard-requirements': '// P99 <1ms • 100K+ req/sec • <100MB • <20MB binary',
        'performance-patterns': '// Arc zero-copy • crossbeam lock-free • rayon parallel',
        'benchmarking-new-features': '// Baseline → Implement → Compare → Validate <5% regression',
        'add-a-new-datalog-rule-type': '// Edit parser → Add representation → Update eval → Test',
        'add-a-new-cedar-policy-pattern': '// Edit policy.rs → Add entity conversion → Test',
        'update-performance-benchmarks': '// Run benchmarks → Update README + WHITEPAPER',
        'add-new-documentation': '// Create doc → Add to site → Link from README',
        'create-a-release': '// Version bump → Tag → Build → Test → Push → GitHub release',
        'common-build-errors': '// unsafe code, Cedar API compat, Python linker',
        'performance-debugging': '// flamegraph, perf, rustc --emit asm',
        'documentation-build': '// cargo doc, Jekyll serve for GitHub Pages'
    };

    // Detect which page we're on and use appropriate comments
    function getSectionComments() {
        const path = window.location.pathname;
        if (path.includes('whitepaper')) {
            return { sections: whitepaperComments, subsections: whitepaperSubsections };
        } else if (path.includes('agent-guide')) {
            return { sections: agentGuideComments, subsections: agentGuideSubsections };
        }
        return { sections: indexComments, subsections: {} };
    }

    // Initialize sidebar with RUNE-specific content
    function init() {
        const { sections, subsections } = getSectionComments();
        const defaultComment = '// High-Performance Authorization';

        // Use the shared foundation's sidebar core
        if (window.sidebarCore) {
            window.sidebarCore.init({
                sectionComments: sections,
                subsectionComments: subsections,
                defaultComment: defaultComment
            });
        } else {
            console.error('sidebar-core.js not loaded. Make sure to include it before sidebar.js');
        }
    }

    // Run on DOMContentLoaded
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
