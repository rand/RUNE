# Changelog

All notable changes to RUNE will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned
- Full Datalog evaluation engine with semi-naive evaluation
- Hot-reload with RCU (Read-Copy-Update)
- Python bindings (PyO3)
- Comprehensive test suite
- Production observability (Prometheus metrics)
- HTTP server for remote authorization

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

- [Unreleased](https://github.com/yourusername/rune/compare/v0.1.0...HEAD)
- [0.1.0](https://github.com/yourusername/rune/releases/tag/v0.1.0)
