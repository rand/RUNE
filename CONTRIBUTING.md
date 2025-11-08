# Contributing to RUNE

Thank you for your interest in contributing to RUNE! This document provides guidelines and instructions for contributing.

## Table of Contents

1. [Code of Conduct](#code-of-conduct)
2. [Getting Started](#getting-started)
3. [Development Workflow](#development-workflow)
4. [Code Standards](#code-standards)
5. [Testing Requirements](#testing-requirements)
6. [Documentation](#documentation)
7. [Pull Request Process](#pull-request-process)
8. [Release Process](#release-process)

## Code of Conduct

This project adheres to a code of conduct that all contributors are expected to follow:

- **Be respectful**: Treat all contributors with respect and courtesy
- **Be collaborative**: Work together to achieve project goals
- **Be inclusive**: Welcome contributors from all backgrounds
- **Be constructive**: Provide helpful, actionable feedback

Unacceptable behavior will not be tolerated. Please report violations to the project maintainers.

## Getting Started

### Prerequisites

- Rust 1.75 or later
- Git
- (Optional) D2 for diagram generation
- (Optional) Ruby for GitHub Pages development

### Fork and Clone

```bash
# Fork the repository on GitHub
# Then clone your fork
git clone https://github.com/YOUR_USERNAME/rune.git
cd rune

# Add upstream remote
git remote add upstream https://github.com/yourusername/rune.git

# Fetch upstream changes
git fetch upstream
```

### Build and Test

```bash
# Build the project
cargo build

# Run tests
cargo test

# Build release version
cargo build --release

# Run benchmarks
./target/release/rune benchmark --requests 1000
```

## Development Workflow

### Branching Strategy

1. **Create a feature branch**:
```bash
git checkout -b feature/your-feature-name
```

Branch naming conventions:
- `feature/` - New features
- `fix/` - Bug fixes
- `refactor/` - Code refactoring
- `docs/` - Documentation updates
- `perf/` - Performance improvements

2. **Make your changes**:
- Write code following project standards
- Add tests for new functionality
- Update documentation as needed

3. **Commit your changes**:
```bash
git add .
git commit -m "feat: Add your feature description"
```

Commit message format:
```
<type>: <short summary>

<optional detailed explanation>

<optional breaking changes>
```

Types: `feat`, `fix`, `refactor`, `perf`, `docs`, `test`, `chore`

4. **Keep your branch up to date**:
```bash
git fetch upstream
git rebase upstream/main
```

5. **Push to your fork**:
```bash
git push origin feature/your-feature-name
```

6. **Create a pull request** on GitHub

## Code Standards

### Rust Code Style

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for formatting
- Pass `cargo clippy` without warnings
- Document all public APIs with `///` comments

**Example**:
```rust
/// Evaluates an authorization request against configured policies.
///
/// # Arguments
///
/// * `request` - The authorization request to evaluate
///
/// # Returns
///
/// Returns an `AuthorizationResult` with the decision and metadata.
///
/// # Errors
///
/// Returns `RUNEError` if policy evaluation fails.
///
/// # Example
///
/// ```
/// let result = engine.authorize(&request)?;
/// assert!(result.decision.is_permitted());
/// ```
pub fn authorize(&self, request: &Request) -> Result<AuthorizationResult> {
    // Implementation
}
```

### Code Quality Checklist

Before submitting a PR:

- [ ] Code follows Rust formatting (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] All tests pass (`cargo test`)
- [ ] New code has tests
- [ ] Documentation is updated
- [ ] No unnecessary `println!` or debug code
- [ ] No `TODO` or `FIXME` comments (create issues instead)

## Testing Requirements

### Test Coverage

RUNE aims for:
- **Critical path**: 90%+ coverage
- **Authorization logic**: 95%+ coverage
- **Parser**: 80%+ coverage
- **Overall**: 85%+ coverage

### Writing Tests

**Unit tests** (in the same file as the code):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fact_store_insert() {
        let store = FactStore::new();
        let fact = Fact::new("predicate", vec![]);
        store.insert(fact.clone());

        let facts = store.get_facts("predicate").unwrap();
        assert_eq!(facts.len(), 1);
    }
}
```

**Integration tests** (in `tests/` directory):
```rust
// tests/authorization.rs
use rune_core::{RUNEEngine, Request, RequestBuilder, Principal, Action, Resource};

#[test]
fn test_basic_authorization() {
    let engine = RUNEEngine::new();
    let request = RequestBuilder::new()
        .principal(Principal::agent("test-agent"))
        .action(Action::new("read"))
        .resource(Resource::file("/tmp/test.txt"))
        .build()
        .unwrap();

    let result = engine.authorize(&request).unwrap();
    assert!(result.decision.is_permitted());
}
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_fact_store_insert

# Run with output
cargo test -- --nocapture

# Run integration tests only
cargo test --test '*'
```

## Documentation

### When to Update Documentation

Update documentation when you:
- Add new features
- Change APIs
- Modify architecture
- Fix bugs (if user-visible)
- Improve performance significantly

### Documentation Files

| File | Update When |
|------|-------------|
| `README.md` | Feature additions, API changes, benchmark updates |
| `WHITEPAPER.md` | Architecture changes, new concepts, performance changes |
| `AGENT_GUIDE.md` | Workflow changes, new protocols |
| `CONTRIBUTING.md` | Process changes |
| `CHANGELOG.md` | Every release |
| Rustdoc | Any public API changes |

### Rustdoc Standards

- Document all public items
- Include examples for non-trivial functions
- Explain safety requirements for unsafe code
- Link to related items with `[`crate::module::Item`]`

## Pull Request Process

### Before Submitting

1. **Rebase on latest main**:
```bash
git fetch upstream
git rebase upstream/main
```

2. **Run full test suite**:
```bash
cargo test --workspace
```

3. **Check formatting and lints**:
```bash
cargo fmt --check
cargo clippy -- -D warnings
```

4. **Update CHANGELOG.md** if applicable

5. **Validate whitepaper claims** (if changing performance or architecture):
```bash
./scripts/validate-whitepaper.sh
```

### PR Checklist

Your PR description should include:

```markdown
## Summary
Brief description of changes

## Motivation
Why is this change needed?

## Changes
- Change 1
- Change 2
- Change 3

## Testing
How was this tested?

## Documentation
What documentation was updated?

## Checklist
- [ ] Tests pass
- [ ] No clippy warnings
- [ ] Documentation updated
- [ ] CHANGELOG.md updated (if applicable)
- [ ] Whitepaper validated (if applicable)
```

### Review Process

1. **Automated checks** run (CI)
2. **Maintainer review** (1-2 business days)
3. **Address feedback** if requested
4. **Approval and merge**

## Release Process

Releases follow [Semantic Versioning](https://semver.org/):

- **0.x.y**: Development releases (pre-1.0)
  - **0.x.0**: Minor features, non-breaking changes
  - **0.x.y**: Patches, bug fixes
- **1.0.0**: First stable API
- **x.y.z**: SemVer after 1.0

### Creating a Release

1. **Update version** in `Cargo.toml` (workspace.package.version)
2. **Update CHANGELOG.md** with release notes
3. **Commit version bump**:
```bash
git add Cargo.toml CHANGELOG.md
git commit -m "chore: Bump version to x.y.z"
```
4. **Create and push tag**:
```bash
git tag -a vx.y.z -m "Release vx.y.z: Brief description"
git push origin vx.y.z
```
5. **Build release binary**:
```bash
cargo build --release
```
6. **Create GitHub release**:
```bash
gh release create vx.y.z \
  --title "vx.y.z: Release Title" \
  --notes "$(cat CHANGELOG.md | sed -n '/## \[x.y.z\]/,/## \[prev\]/p')" \
  target/release/rune
```

## Questions?

- **Issues**: [GitHub Issues](https://github.com/yourusername/rune/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yourusername/rune/discussions)
- **Security**: Report security vulnerabilities privately to [security contact]

Thank you for contributing to RUNE! 🎉
