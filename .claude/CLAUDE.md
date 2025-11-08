# RUNE Project Guidelines for Claude Code

**IMPORTANT**: This file provides RUNE-specific guidelines. For comprehensive development workflows, testing protocols, and repository organization, see [`AGENT_GUIDE.md`](../AGENT_GUIDE.md) in the project root.

## Quick Reference

**Project**: RUNE - High-Performance Authorization & Configuration Engine
**Stack**: Rust (core), Python bindings (future), Cedar Policy Language
**Performance**: Sub-millisecond latency, 5M+ ops/sec

## Critical Rules

### 1. Testing Protocol (MANDATORY)

**ALWAYS commit BEFORE testing. NEVER test uncommitted code.**

```bash
# Correct workflow:
git add .
git commit -m "feat: Add feature X"
cargo test

# Incorrect (will cause confusion):
cargo test  # ❌ Don't do this before committing
git commit -m "feat: Add feature X"
```

### 2. Performance Requirements

RUNE must maintain:
- **P99 latency**: <1ms
- **Throughput**: 100K+ ops/sec (current: 5M+ ops/sec)
- **Memory**: <100MB for 1M facts
- **Binary size**: <20MB

**Before merging performance-sensitive changes**:
```bash
./target/release/rune benchmark --requests 10000 --threads 8
```

### 3. Documentation Sync

When changing architecture, APIs, or performance characteristics:

| Change Type | Update These Files |
|-------------|-------------------|
| Architecture | `WHITEPAPER.md`, `diagrams/*.d2`, `docs/` |
| API | `README.md`, rustdoc comments |
| Performance | `README.md`, `WHITEPAPER.md` |
| Workflow | `AGENT_GUIDE.md` |

### 4. Lock-Free Concurrency

RUNE's performance depends on lock-free data structures. **DO NOT**:
- ❌ Use `Mutex` or `RwLock` in hot paths
- ❌ Clone large structures unnecessarily
- ❌ Allocate in tight loops

**DO**:
- ✅ Use `Arc` for zero-copy sharing
- ✅ Use `crossbeam` for epoch-based reclamation
- ✅ Use `DashMap` for concurrent hashmaps
- ✅ Use `rayon` for data parallelism

### 5. Cedar Integration Patterns

When working with Cedar Policy Language:

**Correct** (collect entities first):
```rust
let mut all_entities = Vec::new();
all_entities.push(principal_entity);
all_entities.push(resource_entity);
all_entities.push(action_entity);
let entities = Entities::from_entities(all_entities, None)?;
```

**Incorrect** (ownership issues):
```rust
let entities = Entities::new();
entities.add_entities(vec![principal_entity])?;  // Error: moved
entities.add_entities(vec![resource_entity])?;
```

## Project-Specific Commands

```bash
# Build release binary
cargo build --release

# Run benchmarks
./target/release/rune benchmark --requests 10000 --threads 8

# Validate example .rune files
./target/release/rune validate examples/basic.rune

# Validate whitepaper claims
./scripts/validate-whitepaper.sh

# Reproduce whitepaper benchmarks
./scripts/reproduce-benchmarks.sh

# Generate D2 diagrams
cd diagrams && ./generate-diagrams.sh

# Serve GitHub Pages locally
cd docs && bundle install && bundle exec jekyll serve
```

## mnemosyne Integration

If using mnemosyne for memory:

**Store architectural decisions**:
```bash
mnemosyne remember -c "Lock-free fact store uses crossbeam epoch for zero-copy reads" \
  -n "project:rune" -i 9 -t "architecture,performance,concurrency"

mnemosyne remember -c "Cedar 3.x requires collecting entities before creating Entities object" \
  -n "project:rune" -i 8 -t "cedar,gotcha,api"
```

**Recall project context**:
```bash
mnemosyne recall -q "cedar integration patterns" -n "project:rune" -l 5
mnemosyne recall -q "performance optimization" -n "project:rune" -l 10
```

## Common Tasks

See [`AGENT_GUIDE.md`](../AGENT_GUIDE.md#common-tasks) for detailed instructions on:
- Adding new Datalog rule types
- Adding new Cedar policy patterns
- Updating performance benchmarks
- Creating releases

## Beads Integration

```bash
# Track RUNE development tasks
bd create "Implement semi-naive Datalog evaluation" -t feature -p 0 --json
bd create "Add Python bindings tests" -t task -p 1 --json
bd create "Hot-reload with RCU" -t feature -p 1 --json

# Link to architectural decisions
bd dep add bd-<new-task> bd-<parent-epic> --type discovered-from
```

## Anti-Patterns Specific to RUNE

```
❌ Skip benchmark validation after performance changes
❌ Use blocking I/O in authorization path
❌ Bypass whitepaper validation before documentation PRs
❌ Hard-code GitHub username in documentation (use placeholder)
❌ Commit .DS_Store files (already gitignored)
❌ Test before committing (breaks debugging)
❌ Use panic! instead of Result in core engine
❌ Allocate in hot authorization path
```

## File Organization

```
RUNE/
├── rune-core/           # Core Rust engine (DO NOT: add I/O or CLI code here)
├── rune-cli/            # CLI binary (DO NOT: add core logic here)
├── rune-python/         # Python bindings (currently disabled)
├── examples/            # .rune configuration examples
├── diagrams/            # D2 architecture diagrams
├── scripts/             # Validation and benchmark scripts
├── docs/                # GitHub Pages site
├── WHITEPAPER.md        # Technical deep dive
├── AGENT_GUIDE.md       # Comprehensive agent guide (READ THIS)
├── CONTRIBUTING.md      # Contribution guidelines
└── CHANGELOG.md         # Version history
```

## Code Quality Gates

Before marking work complete:
- [ ] Intent satisfied (check original request)
- [ ] Tests written and passing (after commit!)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Documentation updated (README, WHITEPAPER if applicable)
- [ ] CHANGELOG.md updated (if semantic version bump)
- [ ] Benchmarks pass with no regression
- [ ] Whitepaper validated (if claims changed): `./scripts/validate-whitepaper.sh`

## When to Consult AGENT_GUIDE.md

**Always** reference [`AGENT_GUIDE.md`](../AGENT_GUIDE.md) for:
- Detailed repository structure
- Complete development workflow
- Testing protocols and coverage targets
- Documentation update protocols
- Release management process
- Performance debugging techniques
- Comprehensive common tasks

**This file** (CLAUDE.md) provides quick reference only. AGENT_GUIDE.md is the authoritative source.

## Quick Debugging

**Performance regression**:
1. Run benchmark: `./target/release/rune benchmark --requests 10000`
2. Compare to whitepaper claims (WHITEPAPER.md#6-performance-evaluation)
3. Profile with flamegraph: `cargo flamegraph --bin rune -- benchmark`

**Cedar integration issues**:
1. Check entity ownership (collect all entities first)
2. Verify Cedar version is 3.x in Cargo.toml
3. See policy.rs:40-145 for integration patterns

**Build failures**:
1. Check Rust version: `rustc --version` (need 1.75+)
2. Clean build: `cargo clean && cargo build`
3. Check workspace consistency: `cargo check --workspace`

## References

- [`AGENT_GUIDE.md`](../AGENT_GUIDE.md) - **Primary reference** for development
- [`WHITEPAPER.md`](../WHITEPAPER.md) - Architecture and design decisions
- [`CONTRIBUTING.md`](../CONTRIBUTING.md) - Contribution process
- [Cedar Docs](https://docs.cedarpolicy.com/) - Cedar Policy Language reference

---

**Last Updated**: 2025-11-08
**Status**: Living document - update as patterns emerge
**Priority**: Always consult AGENT_GUIDE.md for comprehensive guidance
