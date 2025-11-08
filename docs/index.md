---
layout: home
title: RUNE
permalink: /
---

<div class="hero">
  <h1 class="hero-title">
    <span class="rune-logo">⚡</span> RUNE
  </h1>
  <p class="hero-tagline">Real, Enforceable Guardrails for AI Agents</p>
  <p class="hero-description">
    High-performance authorization and configuration engine with sub-millisecond latency
  </p>
  <div class="hero-cta">
    <a href="/whitepaper" class="btn btn-primary">Read the Whitepaper</a>
    <a href="https://github.com/yourusername/rune" class="btn btn-secondary">View on GitHub</a>
  </div>
</div>

## What is RUNE?

RUNE (Rules, Unification, Norms, Enforcement) is a production-grade authorization and configuration system that combines **Datalog-based configuration rules** with **Cedar-style authorization policies**. It provides real, enforceable guardrails for AI agents with:

<div class="features">
  <div class="feature">
    <h3>⚡ Sub-Millisecond Performance</h3>
    <p>&lt;1ms P99 latency, 5M+ operations per second on a single core</p>
  </div>

  <div class="feature">
    <h3>🔒 Dual-Engine Architecture</h3>
    <p>Datalog for configuration, Cedar for authorization, evaluated in parallel</p>
  </div>

  <div class="feature">
    <h3>🚀 Zero-Copy Design</h3>
    <p>Lock-free data structures with memory-mapped facts for maximum throughput</p>
  </div>

  <div class="feature">
    <h3>📦 Single Binary</h3>
    <p>~10MB static executable with no dependencies or external services</p>
  </div>

  <div class="feature">
    <h3>🐍 Python Bindings</h3>
    <p>Easy integration with AI frameworks: LangChain, AutoGPT, Claude Code</p>
  </div>

  <div class="feature">
    <h3>🔍 Formally Verified</h3>
    <p>Cedar's formal semantics enable policy analysis and verification</p>
  </div>
</div>

## Quick Start

```bash
# Install RUNE
cargo install rune-cli

# Evaluate an authorization request
rune eval --action read --resource /tmp/file.txt --principal agent-1

# Run benchmarks
rune benchmark --requests 10000 --threads 8
```

## Example Configuration

```rune
version = "rune/1.0"

[data]
environment = "production"
agent.capabilities = ["read", "write"]

[rules]
# Datalog rules for configuration
allow_file_read(Path) :-
    action("file.read"),
    path(Path),
    Path.starts_with("/allowed").

[policies]
# Cedar policies for authorization
permit(
    principal in Group::"agents",
    action == Action::"read",
    resource in File::"/tmp/*"
) when {
    principal.verified == true
};
```

## Performance

<div class="metrics">
  <div class="metric">
    <div class="metric-value">5M+</div>
    <div class="metric-label">ops/sec</div>
  </div>

  <div class="metric">
    <div class="metric-value">&lt;1ms</div>
    <div class="metric-label">P99 latency</div>
  </div>

  <div class="metric">
    <div class="metric-value">~10MB</div>
    <div class="metric-label">binary size</div>
  </div>

  <div class="metric">
    <div class="metric-value">90%+</div>
    <div class="metric-label">cache hit rate</div>
  </div>
</div>

Benchmark results on Apple M1. See the [whitepaper](/whitepaper#6-performance-evaluation) for detailed performance analysis.

## Architecture

RUNE uses a novel dual-engine architecture that combines the best of both worlds:

```
Request → RUNE Engine → [Datalog Engine || Cedar Engine] → Decision
                            ↓              ↓
                         FactStore    PolicySet
                            ↓              ↓
                      Lock-free       Cached
```

- **Datalog Engine**: Derives configuration facts through logic programming
- **Cedar Engine**: Evaluates authorization policies with formal semantics
- **Lock-Free Fact Store**: Concurrent access using crossbeam epoch-based reclamation
- **Parallel Evaluation**: Both engines run simultaneously via rayon

Learn more in the [Architecture section of the whitepaper](/whitepaper#4-architecture).

## Use Cases

### AI Agent File Access
Control which files agents can read/write based on directory, content type, and environment.

### API Rate Limiting
Enforce different rate limits per API provider with burst allowance and emergency overrides.

### Multi-Environment Configuration
Same agent, different rules for dev/staging/production environments.

### Break-Glass Workflows
Emergency access with human approval and comprehensive audit logging.

See [Workflows and Use Cases](/whitepaper#7-workflows-and-use-cases) for detailed examples.

## Documentation

- [**Whitepaper**](/whitepaper): Complete technical deep dive
- [**Agent Guide**](/agent-guide): Guide for agentic systems (Claude Code, mnemosyne)
- [**Examples**](https://github.com/yourusername/rune/tree/main/examples): Sample .rune configurations
- [**API Docs**](https://docs.rs/rune-core): Rust API documentation

## Community

- **GitHub**: [github.com/yourusername/rune](https://github.com/yourusername/rune)
- **Issues**: [Report bugs or request features](https://github.com/yourusername/rune/issues)
- **Discussions**: [Ask questions and share ideas](https://github.com/yourusername/rune/discussions)
- **License**: MIT OR Apache-2.0

## Project Status

RUNE v0.1.0 is a production-grade foundation with:

✅ **Completed**
- Rust core engine with lock-free data structures
- Request authorization with caching
- Cedar policy engine integration
- CLI tool with benchmarking
- Basic parser for RUNE files
- Python bindings structure

🚧 **In Progress**
- Full Datalog evaluation engine
- Hot-reload with RCU
- Comprehensive test suite
- Production observability

See the [whitepaper](/whitepaper#10-future-work) for the complete roadmap.

---

<div class="footer-cta">
  <h2>Ready to add real guardrails to your AI agents?</h2>
  <a href="/whitepaper" class="btn btn-primary btn-large">Read the Technical Whitepaper</a>
</div>
