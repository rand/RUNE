# RUNE Agent Guide

**For**: Agentic systems (Claude Code, mnemosyne, etc.)
**Purpose**: Efficient, comprehensive guidance for working on RUNE
**Version**: 0.1.0

---

## Table of Contents

1. [Quick Start](#quick-start)
2. [Project Overview](#project-overview)
3. [Repository Structure](#repository-structure)
4. [Development Workflow](#development-workflow)
5. [Documentation Protocols](#documentation-protocols)
6. [Documentation Management](#documentation-management)
7. [Release Management](#release-management)
8. [Repository Organization](#repository-organization)
9. [Testing Protocols](#testing-protocols)
10. [Performance Requirements](#performance-requirements)
11. [Common Tasks](#common-tasks)

---

## Quick Start

**Project**: RUNE - High-Performance Authorization & Configuration Engine
**Stack**: Rust (core), Python bindings, Cedar Policy Language
**Performance**: Sub-millisecond latency, 5M+ ops/sec
**License**: MIT OR Apache-2.0

**Critical Context**:
- Work in feature branches, never commit to `main` directly
- Test AFTER committing (never test uncommitted code)
- Update ALL relevant docs when making significant changes
- Maintain sub-millisecond authorization latency

**First Steps**:
```bash
# Build and test
cargo build --release
cargo test
./target/release/rune --help

# Run benchmarks to validate performance
./target/release/rune benchmark --requests 10000 --threads 8
```

---

## Project Overview

### What is RUNE?

RUNE provides **real, enforceable guardrails** for AI agents through a dual-engine architecture:

1. **Datalog Engine**: Configuration rules and logic programming
2. **Cedar Engine**: Authorization policies (Amazon Cedar Policy Language)

### Core Value Proposition

- **Performance**: Sub-millisecond authorization decisions (<1ms P99)
- **Throughput**: 5M+ operations per second on a single core
- **Deployment**: Single binary (~10MB static executable)
- **Integration**: Python bindings for AI frameworks (LangChain, AutoGPT, etc.)

### Architecture

```
Request → RUNEEngine → [Datalog Engine || Cedar Engine] → Decision
                            ↓              ↓
                         FactStore    PolicySet
                            ↓              ↓
                      Lock-free       Cached
```

**Key Design Patterns**:
- **Lock-free data structures**: Crossbeam epoch-based memory reclamation
- **Zero-copy architecture**: Arc-wrapped values, memory-mapped facts
- **Parallel evaluation**: Rayon for dual-engine concurrency
- **DashMap caching**: Concurrent hashmap for authorization results

---

## Repository Structure

```
RUNE/
├── rune-core/          # Rust core engine
│   ├── src/
│   │   ├── lib.rs      # Public API surface
│   │   ├── engine.rs   # Main authorization engine
│   │   ├── facts.rs    # Lock-free fact store
│   │   ├── policy.rs   # Cedar integration
│   │   ├── datalog.rs  # Datalog evaluation
│   │   ├── parser.rs   # .rune file parser
│   │   ├── request.rs  # Request abstraction
│   │   └── types.rs    # Core types (Value, Entity, etc.)
│   └── Cargo.toml
├── rune-cli/           # Command-line interface
│   ├── src/main.rs     # CLI with eval, validate, benchmark, serve
│   └── Cargo.toml
├── rune-python/        # Python bindings (PyO3)
│   └── (disabled until Python dev env configured)
├── examples/           # Example .rune configurations
├── specs/              # Project specifications
│   └── origination/    # Original competing plans
├── docs/               # GitHub Pages documentation
├── diagrams/           # D2 architecture diagrams
├── scripts/            # Validation and build scripts
├── Cargo.toml          # Workspace manifest
├── README.md           # User-facing documentation
├── WHITEPAPER.md       # Technical whitepaper
├── AGENT_GUIDE.md      # This file
├── CONTRIBUTING.md     # Contribution guidelines
├── CHANGELOG.md        # Version history
├── LICENSE-MIT         # MIT license
└── LICENSE-APACHE      # Apache 2.0 license
```

### Key Files

| File | Purpose | Update When |
|------|---------|-------------|
| `README.md` | User documentation, quick start | API changes, feature additions |
| `WHITEPAPER.md` | Technical deep dive, architecture | Architecture changes, new concepts |
| `AGENT_GUIDE.md` | Agent workflow guidance | Workflow changes, new protocols |
| `CHANGELOG.md` | Version history | Every semantic version change |
| `CONTRIBUTING.md` | Contribution guidelines | Process changes |
| `Cargo.toml` | Workspace dependencies | New crates, dependency updates |

---

## Development Workflow

### Branching Strategy

**ALWAYS use feature branches**:
```bash
git checkout -b feature/typed-holes
git checkout -b fix/cache-invalidation
git checkout -b docs/architecture-diagrams
```

**Branch Naming**:
- `feature/`: New functionality
- `fix/`: Bug fixes
- `refactor/`: Code restructuring
- `docs/`: Documentation updates
- `perf/`: Performance improvements

### Commit Protocol

**CRITICAL**: Commit BEFORE testing, never test uncommitted code.

```bash
# 1. Make changes
# 2. Commit changes
git add .
git commit -m "Add typed holes for Datalog integration"
git log -1 --oneline

# 3. Run tests
cargo test

# 4. If tests fail: Fix → Commit → Re-test
```

**Commit Message Format**:
```
<type>: <short summary>

<optional detailed explanation>

<optional breaking changes>
```

**Types**: `feat`, `fix`, `refactor`, `perf`, `docs`, `test`, `chore`

**Examples**:
```
feat: Add semi-naive Datalog evaluation engine
fix: Resolve race condition in cache invalidation
perf: Optimize fact store with lock-free epoch reclamation
docs: Update whitepaper with performance benchmarks
```

### Pull Requests

```bash
# Push branch
git push -u origin feature/typed-holes

# Create PR (GitHub CLI)
gh pr create --title "Add typed holes for Datalog integration" \
  --body "Implements typed holes pattern for clean component boundaries..."
```

**PR Checklist**:
- [ ] All tests pass
- [ ] Benchmarks show no performance regression
- [ ] Documentation updated (README, WHITEPAPER, etc.)
- [ ] CHANGELOG.md updated if semantic version bump
- [ ] Code follows Rust best practices (clippy clean)

---

## Documentation Protocols

### When to Update Documentation

| Change Type | Update These Docs |
|-------------|-------------------|
| New feature | README.md, WHITEPAPER.md, examples/ |
| API change | README.md, rune-core/src/lib.rs (rustdoc) |
| Architecture change | WHITEPAPER.md, diagrams/, docs/ |
| Performance improvement | README.md (benchmarks), WHITEPAPER.md |
| Bug fix | CHANGELOG.md |
| Breaking change | CHANGELOG.md, README.md, migration guide |

### Documentation Standards

**README.md**:
- Keep quick start up-to-date
- Update benchmark results when performance changes
- Maintain feature list accuracy
- Update development status checklist

**WHITEPAPER.md**:
- Validate all claims against code
- Link to specific tagged versions for code references
- Update architecture diagrams when design changes
- Maintain narrative flow and technical precision

**AGENT_GUIDE.md**:
- Update when workflows change
- Add new common tasks as they emerge
- Keep repository structure current

**Rustdoc**:
- Document all public APIs with `///` comments
- Provide examples for non-trivial functions
- Explain safety requirements for unsafe code

### Code Reference Format

When updating docs, use tagged version links:

```markdown
The [fact store implementation](https://github.com/user/rune/blob/v0.1.0/rune-core/src/facts.rs#L45-L67)
uses epoch-based memory reclamation.
```

**Format**: `https://github.com/{user}/{repo}/blob/{tag}/{path}#{lines}`

---

## Documentation Management

### Structure

```
docs/                     # Markdown source files
├── index.md             # Home page
├── whitepaper.md        # Technical documentation
├── css/styles.css       # Custom design (project-specific colors)
├── js/                  # Theme toggle, sidebar behavior
└── assets/              # Images, diagrams, favicons

templates/               # Jinja2 HTML templates
├── base.html           # Navbar, sidebar, theme toggle
├── index.html          # Home page layout
└── whitepaper.html     # Documentation layout

scripts/build-docs.py   # Python build script
site/                    # Generated static HTML (ignored by git)
```

### Updating Documentation

1. **Edit markdown files** in `docs/` (index.md, whitepaper.md, etc.)
2. **Test locally** (optional):
   ```bash
   python scripts/build-docs.py
   cd site && python -m http.server 8000
   ```
3. **Commit and push** to main
4. **Verify deployment**: GitHub Actions builds and deploys automatically (~1-2 min)

### Design System

**Project-specific**:
- **Glyph**: ∮ (Closed contour integral) in navbar
- **Colors**: Accent color in `docs/css/styles.css` (`:root` CSS variables)
- **Tagline**: "// High-Performance Authorization" in fixed right sidebar

**Shared features**:
- Geist font + JetBrains Mono for code
- Theme toggle (light/dark)
- Responsive design (sidebar hides <1200px)
- SVG diagrams with light/dark variants

### Build Process

1. Python-Markdown parses `.md` files
2. YAML front matter stripped automatically
3. Jinja2 templates apply HTML structure
4. Static HTML + CSS/JS copied to `site/`
5. GitHub Actions deploys to GitHub Pages

### Troubleshooting

| Issue | Solution |
|-------|----------|
| Build fails | Check Python dependencies: `pip install markdown jinja2 pygments` |
| Styles missing | Verify `docs/css/styles.css` exists |
| Theme toggle broken | Check `docs/js/theme.js` loaded |
| Diagrams missing | Verify SVG files in `docs/assets/diagrams/` |
| Old content showing | Hard refresh browser (Cmd+Shift+R) |

---

## Release Management

### Semantic Versioning

RUNE follows [SemVer 2.0.0](https://semver.org/):

- **0.x.y**: Development releases (pre-1.0, API unstable)
  - **0.x.0**: Minor features, non-breaking changes
  - **0.x.y**: Patches, bug fixes
- **1.0.0**: First stable API release
- **x.0.0**: Major version (breaking changes)
- **x.y.0**: Minor version (new features, backward compatible)
- **x.y.z**: Patch version (bug fixes, backward compatible)

### Release Process

**1. Version Bump**:
```bash
# Update version in Cargo.toml (workspace.package.version)
# Update CHANGELOG.md with release notes
git add Cargo.toml CHANGELOG.md
git commit -m "chore: Bump version to 0.2.0"
```

**2. Create Tag**:
```bash
git tag -a v0.2.0 -m "Release v0.2.0: Semi-naive Datalog evaluation"
git push origin v0.2.0
```

**3. Build Release Artifacts**:
```bash
cargo build --release
# Binary: target/release/rune
```

**4. Create GitHub Release**:
```bash
gh release create v0.2.0 \
  --title "v0.2.0: Semi-naive Datalog evaluation" \
  --notes "$(cat CHANGELOG.md | sed -n '/## \[0.2.0\]/,/## \[0.1.0\]/p')" \
  target/release/rune#rune-v0.2.0-$(uname -m)-$(uname -s)
```

### Special Tags

- **Validation Tags**: For whitepaper claims validation (e.g., `v0.1.0-whitepaper`)
- **Pre-release Tags**: For testing (e.g., `v0.2.0-rc.1`)

---

## Repository Organization

### Organization Principles

1. **Non-destructive**: Reorganize without losing references or history
2. **Reference-preserving**: Maintain links, cross-references, and citations
3. **Context-efficient**: Keep structure clear and navigable
4. **Documentation-driven**: Structure reflects documented architecture

### Tidying Guidelines

**DO**:
- ✅ Use `git mv` to preserve history when moving files
- ✅ Update all documentation cross-references after moves
- ✅ Group related files in logical directories
- ✅ Remove generated artifacts (covered by .gitignore)
- ✅ Clean up commented-out code in favor of git history

**DON'T**:
- ❌ Delete files without checking for references
- ❌ Rename files without updating imports/documentation
- ❌ Move files between commits (do it as separate commit)
- ❌ Restructure without updating AGENT_GUIDE.md

**Example Tidy Workflow**:
```bash
# Move file with history preservation
git mv old/path/file.rs new/path/file.rs

# Update imports and references
# (edit affected files)

# Commit move and updates together
git add .
git commit -m "refactor: Reorganize fact store modules

- Move fact store to rune-core/src/storage/
- Update all imports and cross-references
- Update AGENT_GUIDE.md repository structure"
```

### Adding New Components

When adding new crates, modules, or major features:

1. **Create structure**:
```bash
mkdir -p rune-new-component/src
cd rune-new-component
```

2. **Add to workspace** (`Cargo.toml`):
```toml
[workspace]
members = [
    "rune-core",
    "rune-cli",
    "rune-new-component",  # Add here
]
```

3. **Update documentation**:
- Add to repository structure in AGENT_GUIDE.md
- Add to architecture diagram
- Update README.md if user-facing

4. **Add tests**:
```bash
mkdir -p rune-new-component/tests
# Add integration tests
```

---

## Testing Protocols

### Critical Testing Rules

**RULE 1**: Commit BEFORE testing
**RULE 2**: NEVER test uncommitted code
**RULE 3**: Kill old test processes before running new tests

### Testing Workflow

```bash
# 1. Make changes and commit
git add .
git commit -m "feat: Add feature X"

# 2. Kill any running tests
pkill -f "cargo test"

# 3. Run tests
cargo test

# 4. Run benchmarks (for performance-critical changes)
cargo bench

# 5. If tests fail:
#    - Fix issue
#    - Commit fix
#    - Re-test (goto step 2)
```

### Test Types

| Test Type | Command | When to Run |
|-----------|---------|-------------|
| Unit | `cargo test --lib` | After every change |
| Integration | `cargo test --test '*'` | Before PR |
| Benchmarks | `cargo bench` | Performance changes |
| CLI | `cargo test --bin rune` | CLI changes |
| Full suite | `cargo test --workspace` | Before release |

### Performance Testing

**Critical**: RUNE must maintain sub-millisecond latency.

```bash
# Run standard benchmark
./target/release/rune benchmark --requests 10000 --threads 8

# Expected output:
# Throughput: >5,000,000 req/sec
# Avg latency: <0.001ms
# P99 latency: <0.001ms
# Cache hit rate: >90%
```

**If performance degrades**:
1. Profile with `cargo flamegraph` or `perf`
2. Identify bottleneck
3. Optimize hot path
4. Re-benchmark
5. Update README.md with new benchmark results

### Test Coverage

Target coverage (when tooling is added):
- Critical path: 90%+
- Authorization logic: 95%+
- Parser: 80%+
- Overall: 85%+

---

## Performance Requirements

### Hard Requirements

| Metric | Requirement | Current |
|--------|-------------|---------|
| P99 Latency | <1ms | ~0.0005ms |
| Throughput | 100K+ req/sec | 5M+ req/sec |
| Memory | <100MB for 1M facts | <50MB |
| Binary Size | <20MB | ~10MB |
| Cache Hit Rate | >85% | >90% |

### Performance Patterns

**DO**:
- ✅ Use Arc for zero-copy sharing
- ✅ Use crossbeam for lock-free data structures
- ✅ Use rayon for data parallelism
- ✅ Use DashMap for concurrent caching
- ✅ Minimize allocations in hot paths
- ✅ Use `#[inline]` for small, hot functions

**DON'T**:
- ❌ Use `Mutex` or `RwLock` in hot paths
- ❌ Clone large structures unnecessarily
- ❌ Allocate in tight loops
- ❌ Use async where sync suffices (added overhead)
- ❌ Ignore `clippy::perf` warnings

### Benchmarking New Features

When adding features:

1. **Baseline**:
```bash
cargo bench -- baseline
```

2. **Implement feature**

3. **Compare**:
```bash
cargo bench -- new_feature
```

4. **Validate**: No regression >5% on critical paths

5. **Update docs**: If performance characteristics change

---

## Common Tasks

### Add a New Datalog Rule Type

```bash
# 1. Edit parser
vim rune-core/src/parser.rs

# 2. Add rule representation
vim rune-core/src/datalog.rs

# 3. Update evaluation
vim rune-core/src/datalog.rs

# 4. Add tests
vim rune-core/src/datalog.rs  # Add #[cfg(test)] tests

# 5. Add example
vim examples/new-rule.rune

# 6. Commit and test
git add .
git commit -m "feat: Add negation support to Datalog rules"
cargo test
```

### Add a New Cedar Policy Pattern

```bash
# 1. Edit policy integration
vim rune-core/src/policy.rs

# 2. Add entity conversion if needed
vim rune-core/src/types.rs

# 3. Add tests
vim rune-core/src/policy.rs

# 4. Add example
vim examples/new-policy.rune

# 5. Commit and test
git add .
git commit -m "feat: Support hierarchical resource policies"
cargo test
```

### Update Performance Benchmarks

```bash
# 1. Run benchmarks
./target/release/rune benchmark --requests 100000 --threads 16

# 2. Update README.md
vim README.md  # Update benchmark results section

# 3. Update WHITEPAPER.md
vim WHITEPAPER.md  # Update performance section

# 4. Commit
git add README.md WHITEPAPER.md
git commit -m "docs: Update benchmark results (10M ops/sec achieved)"
```

### Add New Documentation

```bash
# 1. Create doc file
vim docs/new-guide.md

# 2. Add to docs site
vim docs/_config.yml  # Add to navigation

# 3. Link from README
vim README.md  # Add link to new guide

# 4. Commit
git add docs/ README.md
git commit -m "docs: Add advanced Datalog guide"
```

### Create a Release

```bash
# 1. Update version
vim Cargo.toml  # workspace.package.version

# 2. Update CHANGELOG.md
vim CHANGELOG.md  # Add release notes

# 3. Commit version bump
git add Cargo.toml CHANGELOG.md
git commit -m "chore: Bump version to 0.3.0"

# 4. Create tag
git tag -a v0.3.0 -m "Release v0.3.0: Hot-reload with RCU"

# 5. Build release
cargo build --release

# 6. Test release binary
./target/release/rune --version
./target/release/rune benchmark --requests 10000

# 7. Push tag
git push origin v0.3.0

# 8. Create GitHub release
gh release create v0.3.0 \
  --title "v0.3.0: Hot-reload with RCU" \
  --notes-file <(grep -A 20 "## \[0.3.0\]" CHANGELOG.md)
```

---

## Integration with mnemosyne

If using mnemosyne for memory and orchestration:

**Store Key Decisions**:
```bash
mnemosyne remember -c "Using Cedar 3.x API with entity ownership pattern" \
  -n "project:rune" -i 9 -t "architecture,cedar"

mnemosyne remember -c "Lock-free fact store requires unsafe code allowance in facts.rs" \
  -n "project:rune" -i 8 -t "implementation,concurrency"
```

**Recall Context**:
```bash
mnemosyne recall -q "cedar policy integration" -n "project:rune" -l 5
mnemosyne recall -q "performance optimization" -n "project:rune" -l 10
```

**Store TODO Items**:
```bash
mnemosyne remember -c "Implement semi-naive Datalog evaluation" \
  -n "project:rune" -i 9 -t "todo,datalog"

mnemosyne remember -c "Add Python bindings tests" \
  -n "project:rune" -i 7 -t "todo,python"
```

---

## Troubleshooting

### Common Build Errors

**Error**: `unsafe code is forbidden`
**Fix**: Add `#![allow(unsafe_code)]` to module using crossbeam/unsafe patterns

**Error**: Cedar API compatibility
**Fix**: Check Cedar version (must be 3.x), update to latest API patterns

**Error**: Python linker errors
**Fix**: Ensure Python dev environment configured, or disable rune-python in workspace

### Performance Debugging

```bash
# Profile with flamegraph
cargo flamegraph --bin rune -- benchmark --requests 10000

# Profile with perf (Linux)
perf record --call-graph dwarf ./target/release/rune benchmark
perf report

# Check assembly output
cargo rustc --release -- --emit asm
```

### Documentation Build

```bash
# Build rustdoc
cargo doc --no-deps --open

# Build GitHub Pages locally (requires Jekyll)
cd docs
bundle install
bundle exec jekyll serve
```

---

## References

- [Cedar Policy Language Docs](https://docs.cedarpolicy.com/)
- [Crossbeam Documentation](https://docs.rs/crossbeam/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [SemVer Specification](https://semver.org/)
- [D2 Diagram Language](https://d2lang.com/)

---

**Last Updated**: 2025-11-08
**Maintainer**: RUNE Contributors
**Status**: Living document - update as workflows evolve
