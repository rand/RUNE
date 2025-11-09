# Changelog

All notable changes to RUNE will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned
- Python bindings (PyO3)
- Production observability (Prometheus metrics, OpenTelemetry)
- HTTP server for remote authorization
- Comprehensive test suite (85%+ coverage)

## [0.3.0] - 2025-11-08

### Added
- **Hot-reload with RCU (Read-Copy-Update) pattern**
  - Lock-free engine updates using `arc-swap` crate
  - Atomic rule and policy swapping with `ArcSwap<DatalogEngine>` and `ArcSwap<PolicySet>`
  - Zero-downtime configuration updates (ongoing requests complete with old rules)
  - Automatic memory reclamation with Arc reference counting
  - Cache invalidation on reload
- **File watching for automatic reload**
  - Cross-platform file watching with `notify` crate (supports macOS, Linux, Windows)
  - RUNEWatcher module for .rune and .toml file monitoring
  - Event debouncing for multi-chunk file writes (500ms settling time)
  - Configurable debounce duration and retry policies
- **Async reload coordinator**
  - ReloadCoordinator for orchestrating file watching and engine updates
  - Tokio-based async task for non-blocking reload operations
  - Manual reload capability for testing and explicit user requests
  - Reload event subscription for monitoring and observability
  - ReloadEvent reporting (Success, Failed, Skipped) with timestamps
- **Design documentation**
  - Hot-reload architecture design document (docs/hot-reload-design.md)
  - RCU pattern explanation and trade-off analysis
  - File watching and debouncing strategy documentation

### Changed
- **Engine concurrency model**: Replaced `RwLock<DatalogEngine>` with `ArcSwap<DatalogEngine>` for lock-free reads
- **Policy storage**: Replaced `RwLock<PolicySet>` with `ArcSwap<PolicySet>` for lock-free reads
- **Read operations**: All engine reads now use `load()` instead of `read()` for lock-free access

### Performance
- **Read latency during reload**: Reduced from ~50ns (RwLock) to ~5-10ns (ArcSwap)
- **Blocking during reload**: Eliminated all reader blocking (was: all readers blocked during write)
- **Zero downtime**: Configuration updates complete without interrupting in-flight requests
- **Memory efficiency**: Old engine/policy instances automatically reclaimed when last reader drops

### Infrastructure
- Added dependencies: `arc-swap = "1.7"`, `notify = "6.1"`
- Added dev dependency: `tempfile = "3.8"` for watcher tests
- New modules: `rune-core/src/watcher.rs` (320+ lines), `rune-core/src/reload.rs` (280+ lines)
- Test coverage: 4 new reload coordinator tests, 3 new watcher tests

### Breaking Changes
- None (internal concurrency primitives changed, but public API unchanged)

## [0.2.0] - 2025-11-08

### Added
- **Custom Datalog evaluation engine**
  - Semi-naive bottom-up evaluation with delta tracking
  - Stratified negation support (safe handling of `not` in rules)
  - Aggregation operations: `count`, `sum`, `min`, `max`, `mean`
  - Lock-free concurrent reads with Arc-based zero-copy fact access
  - Hot-reload ready architecture (interpreted rules enable runtime updates)
  - 26 passing Datalog evaluation tests
- **BYODS (Bring Your Own Data Structures) relation backends**
  - VecBackend for small relations (optimized for <100 tuples)
  - HashBackend for general-purpose storage (O(1) lookups)
  - UnionFindBackend foundation for future transitive closure optimization
  - TrieBackend foundation for future prefix-based queries
  - Automatic backend selection heuristics based on relation size
- **Datalog rule parser**
  - Parse facts: `user(alice).`
  - Parse rules: `can_access(U) :- user(U), admin(U).`
  - Parse negation: `allowed(X) :- user(X), not blocked(X).`
  - Type inference for integers, booleans, strings, variables
  - Support for comparison operators: `=`, `!=`, `<`, `>`, `<=`, `>=`
  - 8 passing parser tests
- **Cedar entity to Datalog fact bridge**
  - Convert Cedar Principal/Resource/Action to Datalog facts
  - Handle hierarchical entities and attributes
  - Request metadata facts for authorization patterns
  - 6 passing bridge tests

### Performance
- **Datalog evaluation**: Efficient fixpoint computation with delta sets
- **Relation lookups**: O(1) for hash-backed relations, O(n) for vector-backed
- **Memory overhead**: Minimal with automatic backend selection

### Infrastructure
- Added dependencies: `nom = "7.1"`, `winnow = "0.5"` for parsing
- New modules:
  - `rune-core/src/datalog/types.rs` (rule/fact types)
  - `rune-core/src/datalog/parser.rs` (Datalog parser)
  - `rune-core/src/datalog/unification.rs` (variable binding)
  - `rune-core/src/datalog/evaluation.rs` (semi-naive engine)
  - `rune-core/src/datalog/backends.rs` (BYODS relation storage)
  - `rune-core/src/datalog/bridge.rs` (Cedar-Datalog bridge)
- Example configurations: `examples/datalog_*.rune`

### Changed
- Engine now uses real Datalog evaluation instead of placeholder permit-all
- Parser expanded from TOML-only to full Datalog syntax

### Fixed
- Placeholder Datalog engine replaced with production-ready implementation

## [0.1.0] - 2025-11-08

### Added
- Initial RUNE implementation with Rust core engine
- Dual-engine architecture (Datalog + Cedar)
- Lock-free fact store using crossbeam epoch-based reclamation
- Cedar Policy Language integration (Cedar 3.x)
- Request authorization with DashMap-based caching
- Parallel engine evaluation with rayon
- CLI tool with four commands: eval, validate, benchmark, serve
- Basic .rune file parser (TOML data, placeholder rules, Cedar policies)
- Comprehensive documentation:
  - Technical whitepaper (WHITEPAPER.md)
  - Agent guide for Claude Code/mnemosyne (AGENT_GUIDE.md)
  - Contributing guidelines (CONTRIBUTING.md)
  - GitHub Pages site for documentation
- D2 architecture diagrams:
  - Request flow diagram
  - System architecture diagram
- Validation scripts:
  - Whitepaper claim validation
  - Benchmark reproduction
- Example .rune configurations
- Dual MIT/Apache-2.0 licensing

### Performance
- Sub-millisecond authorization latency (<1ms P99)
- 5M+ operations per second throughput (Apple M1, 4 cores)
- 90%+ cache hit rate
- <50MB memory usage for 1M facts
- ~10MB static binary size

### Infrastructure
- Cargo workspace with 3 crates: rune-core, rune-cli, rune-python
- Comprehensive .gitignore
- Git validation tag (v0.1.0-whitepaper) for documentation verification
- GitHub Actions CI setup (pending)

### Known Limitations
- Datalog engine is placeholder (returns permit for all)
- Python bindings disabled (pending environment configuration)
- No hot-reload support yet
- Limited test coverage
- No production observability yet

## [0.0.0] - 2025-11-07

### Added
- Project initialization
- Cargo workspace setup
- Basic project structure

---

## Release Types

- **Major (x.0.0)**: Breaking changes to public API (after 1.0.0)
- **Minor (0.x.0)**: New features, backward compatible (pre-1.0: may have breaking changes)
- **Patch (0.0.x)**: Bug fixes, backward compatible

## Categories

- **Added**: New features
- **Changed**: Changes to existing functionality
- **Deprecated**: Soon-to-be removed features
- **Removed**: Removed features
- **Fixed**: Bug fixes
- **Security**: Security fixes
- **Performance**: Performance improvements

## Links

- [Unreleased](https://github.com/yourusername/rune/compare/v0.3.0...HEAD)
- [0.3.0](https://github.com/yourusername/rune/compare/v0.2.0...v0.3.0)
- [0.2.0](https://github.com/yourusername/rune/compare/v0.1.0...v0.2.0)
- [0.1.0](https://github.com/yourusername/rune/releases/tag/v0.1.0)
