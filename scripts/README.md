# RUNE Validation Scripts

This directory contains scripts for validating whitepaper claims and reproducing benchmarks.

## Scripts

### validate-whitepaper.sh

Validates that all claims in the whitepaper are backed by code at the tagged version.

**Checks:**
- ✅ Whitepaper file exists
- ✅ Validation tag exists (v0.1.0-whitepaper)
- ✅ Code references are valid file paths
- ✅ Performance claims are documented
- ✅ Benchmark binary runs
- ✅ Example .rune files are valid
- ✅ D2 diagrams have valid syntax
- ✅ Required sections are present
- ✅ Word count meets minimum (3000+)

**Usage:**
```bash
./scripts/validate-whitepaper.sh
```

**Exit codes:**
- 0: All validations passed
- 1: One or more validations failed

### reproduce-benchmarks.sh

Reproduces the performance benchmarks documented in the whitepaper.

**Benchmarks:**
1. Basic throughput (1000 req, 4 threads)
2. High load (10000 req, 8 threads)
3. Single-threaded baseline (1000 req, 1 thread)

**Usage:**
```bash
./scripts/reproduce-benchmarks.sh
```

**Notes:**
- Automatically builds release binary if not present
- Results may vary based on hardware
- Whitepaper benchmarks were on Apple M1

## CI Integration

These scripts are designed to run in CI pipelines to ensure continuous validation of whitepaper claims.

Example GitHub Actions usage:
```yaml
- name: Validate whitepaper
  run: ./scripts/validate-whitepaper.sh

- name: Reproduce benchmarks
  run: ./scripts/reproduce-benchmarks.sh
```

## Development Workflow

Run validation before creating PRs to ensure documentation stays synchronized with code:

```bash
# Full validation
./scripts/validate-whitepaper.sh

# Quick benchmark check
./scripts/reproduce-benchmarks.sh
```

## Requirements

- **validate-whitepaper.sh**: bash, git, grep, cargo (for building)
- **reproduce-benchmarks.sh**: bash, cargo (for building)
- **Optional**: d2 (for diagram validation)
