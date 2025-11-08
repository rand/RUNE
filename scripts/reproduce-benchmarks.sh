#!/usr/bin/env bash
# Reproduce performance benchmarks from whitepaper

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors
BLUE='\033[0;34m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${BLUE}RUNE Benchmark Reproduction${NC}"
echo -e "${BLUE}===========================${NC}\n"

# Check if release binary exists
if [[ ! -f "$PROJECT_ROOT/target/release/rune" ]]; then
    echo -e "${YELLOW}Release binary not found. Building...${NC}"
    cd "$PROJECT_ROOT"
    cargo build --release
fi

echo -e "${BLUE}Running benchmarks to reproduce whitepaper claims...${NC}\n"

# Benchmark 1: Basic throughput (1000 requests, 4 threads)
echo -e "${GREEN}Benchmark 1: Basic Throughput${NC}"
echo "Configuration: 1000 requests, 4 threads"
echo ""
"$PROJECT_ROOT/target/release/rune" benchmark --requests 1000 --threads 4
echo ""

# Benchmark 2: High load (10000 requests, 8 threads)
echo -e "${GREEN}Benchmark 2: High Load${NC}"
echo "Configuration: 10000 requests, 8 threads"
echo ""
"$PROJECT_ROOT/target/release/rune" benchmark --requests 10000 --threads 8
echo ""

# Benchmark 3: Single-threaded baseline
echo -e "${GREEN}Benchmark 3: Single-Threaded Baseline${NC}"
echo "Configuration: 1000 requests, 1 thread"
echo ""
"$PROJECT_ROOT/target/release/rune" benchmark --requests 1000 --threads 1
echo ""

# Extract metrics for comparison
echo -e "${BLUE}Whitepaper Claims vs. Current Results${NC}"
echo -e "${BLUE}======================================${NC}"
echo ""
echo "Claimed in whitepaper:"
echo "  Throughput: 5,080,423 req/sec (1000 req, 4 threads, Apple M1)"
echo "  Latency: <1ms (sub-millisecond)"
echo "  Cache hit rate: 90.9%"
echo ""
echo "Current results shown above."
echo ""
echo -e "${YELLOW}Note:${NC} Results may vary based on hardware, system load, and configuration."
echo "The whitepaper was benchmarked on Apple M1 hardware."
echo ""

# Check if we're on Apple Silicon
if [[ "$(uname -s)" == "Darwin" ]] && [[ "$(uname -m)" == "arm64" ]]; then
    echo -e "${GREEN}Running on Apple Silicon (similar to whitepaper benchmarks)${NC}"
else
    echo -e "${YELLOW}Running on different hardware than whitepaper (Apple M1)${NC}"
fi
