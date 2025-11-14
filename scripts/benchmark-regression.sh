#!/bin/bash
#
# Benchmark Regression Detection Script
#
# Runs criterion benchmarks and compares against baseline to detect performance regressions.
#
# Usage:
#   ./scripts/benchmark-regression.sh [--baseline] [--threshold PERCENT]
#
# Options:
#   --baseline        Save current results as new baseline
#   --threshold N     Set regression threshold (default: 10%)
#   --verbose         Show detailed output
#   --help            Show this help message
#
# Exit codes:
#   0 - No regressions detected
#   1 - Performance regression detected
#   2 - Error running benchmarks

set -e

# Configuration
BENCHMARK_DIR="target/criterion"
BASELINE_DIR=".benchmark-baselines"
THRESHOLD=10  # Default 10% regression threshold
VERBOSE=false
SAVE_BASELINE=false

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --baseline)
            SAVE_BASELINE=true
            shift
            ;;
        --threshold)
            THRESHOLD="$2"
            shift 2
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --help)
            grep '^#' "$0" | cut -c 3-
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 2
            ;;
    esac
done

# Ensure we're in the project root
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Must be run from project root${NC}"
    exit 2
fi

echo -e "${BLUE}=== RUNE Benchmark Regression Detection ===${NC}"
echo "Threshold: ${THRESHOLD}%"
echo

# Create baseline directory if it doesn't exist
mkdir -p "$BASELINE_DIR"

# Run benchmarks
echo -e "${BLUE}Running benchmarks...${NC}"
if ! cargo bench --no-fail-fast -- --save-baseline current 2>&1 | tee /tmp/benchmark-output.log; then
    echo -e "${RED}Error: Benchmark execution failed${NC}"
    exit 2
fi

echo

# If saving baseline, copy results and exit
if [ "$SAVE_BASELINE" = true ]; then
    echo -e "${BLUE}Saving current results as baseline...${NC}"

    # Copy criterion results
    if [ -d "$BENCHMARK_DIR" ]; then
        rm -rf "$BASELINE_DIR/criterion"
        cp -r "$BENCHMARK_DIR" "$BASELINE_DIR/criterion"
        echo -e "${GREEN}✓ Baseline saved successfully${NC}"
    else
        echo -e "${RED}Error: No benchmark results found${NC}"
        exit 2
    fi

    # Save metadata
    cat > "$BASELINE_DIR/metadata.json" <<EOF
{
    "timestamp": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
    "commit": "$(git rev-parse HEAD 2>/dev/null || echo 'unknown')",
    "branch": "$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'unknown')",
    "rust_version": "$(rustc --version)"
}
EOF

    echo
    echo -e "${GREEN}Baseline saved. Future runs will compare against this baseline.${NC}"
    exit 0
fi

# Check if baseline exists
if [ ! -d "$BASELINE_DIR/criterion" ]; then
    echo -e "${YELLOW}Warning: No baseline found${NC}"
    echo "Run with --baseline to create initial baseline:"
    echo "  ./scripts/benchmark-regression.sh --baseline"
    exit 0
fi

# Load baseline metadata
if [ -f "$BASELINE_DIR/metadata.json" ]; then
    echo -e "${BLUE}Baseline information:${NC}"
    if command -v jq &> /dev/null; then
        jq -r '"  Timestamp: \(.timestamp)\n  Commit: \(.commit[0:8])\n  Branch: \(.branch)"' "$BASELINE_DIR/metadata.json"
    else
        cat "$BASELINE_DIR/metadata.json"
    fi
    echo
fi

# Compare results
echo -e "${BLUE}Analyzing performance changes...${NC}"
echo

REGRESSION_FOUND=false
REGRESSION_COUNT=0
IMPROVEMENT_COUNT=0

# Parse criterion output for performance changes
# Criterion reports changes like "change: -5.2%" or "change: +12.3%"
while IFS= read -r line; do
    if [[ "$line" =~ change:\ *([+-]?[0-9]+\.[0-9]+)% ]]; then
        change="${BASH_REMATCH[1]}"

        # Get benchmark name from previous lines (simplified parsing)
        bench_name=$(echo "$line" | grep -oP '(?<=Benchmarking )[^:]+' || echo "unknown")

        # Check for regression (positive change = slower)
        if (( $(echo "$change > $THRESHOLD" | bc -l) )); then
            echo -e "${RED}✗ REGRESSION: $bench_name${NC}"
            echo "  Performance decreased by ${change}% (threshold: ${THRESHOLD}%)"
            REGRESSION_FOUND=true
            ((REGRESSION_COUNT++))
        elif (( $(echo "$change < -5" | bc -l) )); then
            echo -e "${GREEN}✓ IMPROVEMENT: $bench_name${NC}"
            echo "  Performance improved by ${change#-}%"
            ((IMPROVEMENT_COUNT++))
        elif [ "$VERBOSE" = true ]; then
            echo -e "${BLUE}○ STABLE: $bench_name${NC}"
            echo "  Performance change: ${change}%"
        fi
    fi
done < /tmp/benchmark-output.log

echo
echo -e "${BLUE}=== Summary ===${NC}"
echo "Regressions detected: $REGRESSION_COUNT"
echo "Improvements detected: $IMPROVEMENT_COUNT"
echo

if [ "$REGRESSION_FOUND" = true ]; then
    echo -e "${RED}❌ Performance regression detected!${NC}"
    echo
    echo "Next steps:"
    echo "  1. Review the regressed benchmarks above"
    echo "  2. Investigate recent changes that may have caused the regression"
    echo "  3. Profile the affected code paths"
    echo "  4. If the regression is acceptable, update the baseline:"
    echo "     ./scripts/benchmark-regression.sh --baseline"
    exit 1
else
    echo -e "${GREEN}✓ No performance regressions detected${NC}"

    if [ $IMPROVEMENT_COUNT -gt 0 ]; then
        echo
        echo "Consider updating the baseline to reflect these improvements:"
        echo "  ./scripts/benchmark-regression.sh --baseline"
    fi

    exit 0
fi
