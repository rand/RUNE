#!/usr/bin/env bash
# Validate whitepaper claims against code

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WHITEPAPER="$PROJECT_ROOT/WHITEPAPER.md"
TAG="v0.1.0-whitepaper"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}RUNE Whitepaper Validation${NC}"
echo -e "${BLUE}===========================${NC}\n"

FAILURES=0
WARNINGS=0
PASSES=0

# Helper functions
fail() {
    echo -e "${RED}✗ FAIL:${NC} $1"
    FAILURES=$((FAILURES + 1))
}

warn() {
    echo -e "${YELLOW}⚠ WARN:${NC} $1"
    WARNINGS=$((WARNINGS + 1))
}

pass() {
    echo -e "${GREEN}✓ PASS:${NC} $1"
    PASSES=$((PASSES + 1))
}

info() {
    echo -e "${BLUE}→${NC} $1"
}

# 1. Check whitepaper exists
info "Checking whitepaper file exists..."
if [[ -f "$WHITEPAPER" ]]; then
    pass "Whitepaper found at $WHITEPAPER"
else
    fail "Whitepaper not found at $WHITEPAPER"
    exit 1
fi

# 2. Check validation tag exists
info "Checking validation tag exists..."
if git rev-parse "$TAG" >/dev/null 2>&1; then
    pass "Validation tag '$TAG' exists"
    TAG_COMMIT=$(git rev-parse "$TAG")
    info "  Tag points to commit: ${TAG_COMMIT:0:8}"
else
    fail "Validation tag '$TAG' not found"
fi

# 3. Extract code references from whitepaper
info "Extracting code references from whitepaper..."
CODE_REFS=$(grep -oE '\[.*\]\(https://github.com/[^)]+/blob/v[0-9.a-z-]+/[^)]+\)' "$WHITEPAPER" || true)
REF_COUNT=$(echo "$CODE_REFS" | wc -l | tr -d ' ')

if [[ $REF_COUNT -gt 0 ]]; then
    pass "Found $REF_COUNT code references"
else
    warn "No code references found (expected format: [text](https://github.com/.../blob/v0.1.0-whitepaper/...))"
fi

# 4. Validate code file paths exist in tagged version
info "Validating code file paths..."
while IFS= read -r ref; do
    if [[ -z "$ref" ]]; then
        continue
    fi

    # Extract file path from URL
    FILE_PATH=$(echo "$ref" | grep -oE '/blob/v[0-9.a-z-]+/[^)#]+' | sed 's|^/blob/v[0-9.a-z-]*-*[a-z]*-*[0-9]*/||' | cut -d'#' -f1)

    if [[ -z "$FILE_PATH" ]]; then
        continue
    fi

    # Check if file exists at tag
    if git show "$TAG:$FILE_PATH" >/dev/null 2>&1; then
        pass "  File exists: $FILE_PATH"
    else
        fail "  File missing at tag: $FILE_PATH"
    fi
done <<< "$CODE_REFS"

# 5. Check performance claims
info "Checking performance claims..."
PERF_CLAIMS=(
    "5M+ ops/sec"
    "5,080,423"
    "sub-millisecond"
    "<1ms"
    "90%+ cache hit"
)

for claim in "${PERF_CLAIMS[@]}"; do
    if grep -q "$claim" "$WHITEPAPER"; then
        pass "  Claim documented: $claim"
    else
        warn "  Claim missing: $claim"
    fi
done

# 6. Verify benchmark can run
info "Verifying benchmark binary..."
cd "$PROJECT_ROOT"

if [[ -f "target/release/rune" ]]; then
    pass "Release binary exists"

    info "  Running quick benchmark (100 requests)..."
    if ./target/release/rune benchmark --requests 100 --threads 2 >/dev/null 2>&1; then
        pass "  Benchmark runs successfully"
    else
        fail "  Benchmark failed to run"
    fi
else
    warn "Release binary not found (run: cargo build --release)"
fi

# 7. Check example configurations
info "Checking example .rune files..."
EXAMPLE_DIR="$PROJECT_ROOT/examples"

if [[ -d "$EXAMPLE_DIR" ]]; then
    EXAMPLE_COUNT=$(find "$EXAMPLE_DIR" -name "*.rune" | wc -l | tr -d ' ')
    if [[ $EXAMPLE_COUNT -gt 0 ]]; then
        pass "Found $EXAMPLE_COUNT example .rune files"

        for example in "$EXAMPLE_DIR"/*.rune; do
            if [[ -f "$example" ]]; then
                if ./target/release/rune validate "$example" >/dev/null 2>&1; then
                    pass "  Valid: $(basename "$example")"
                else
                    fail "  Invalid: $(basename "$example")"
                fi
            fi
        done
    else
        warn "No .rune examples found"
    fi
else
    warn "Examples directory not found"
fi

# 8. Check D2 diagrams
info "Checking D2 diagram sources..."
DIAGRAM_DIR="$PROJECT_ROOT/diagrams"

if [[ -d "$DIAGRAM_DIR" ]]; then
    D2_COUNT=$(find "$DIAGRAM_DIR" -name "*.d2" | wc -l | tr -d ' ')
    if [[ $D2_COUNT -gt 0 ]]; then
        pass "Found $D2_COUNT D2 diagram sources"

        # Check if d2 is installed
        if command -v d2 &> /dev/null; then
            info "  d2 is installed, validating diagrams..."
            for diagram in "$DIAGRAM_DIR"/*.d2; do
                TMPFILE=$(mktemp /tmp/d2-validate-XXXXXX.svg)
                if d2 "$diagram" "$TMPFILE" 2>/dev/null; then
                    pass "  Valid: $(basename "$diagram")"
                    rm -f "$TMPFILE"
                else
                    fail "  Invalid: $(basename "$diagram")"
                    rm -f "$TMPFILE"
                fi
            done
        else
            warn "  d2 not installed, skipping syntax validation"
        fi
    else
        warn "No D2 diagram sources found"
    fi
else
    warn "Diagrams directory not found"
fi

# 9. Check sections present in whitepaper
info "Checking whitepaper sections..."
REQUIRED_SECTIONS=(
    "Abstract"
    "Introduction"
    "Background and Motivation"
    "System Design"
    "Architecture"
    "Implementation"
    "Performance Evaluation"
    "Workflows and Use Cases"
    "Lessons Learned"
    "Related Work"
    "Future Work"
    "Conclusion"
)

for section in "${REQUIRED_SECTIONS[@]}"; do
    if grep -q "## .*$section" "$WHITEPAPER"; then
        pass "  Section present: $section"
    else
        fail "  Section missing: $section"
    fi
done

# 10. Check word count
info "Checking whitepaper length..."
WORD_COUNT=$(wc -w < "$WHITEPAPER" | tr -d ' ')
if [[ $WORD_COUNT -ge 3000 ]]; then
    pass "Word count: $WORD_COUNT (≥3000 recommended for technical whitepapers)"
else
    warn "Word count: $WORD_COUNT (<3000, may be too brief)"
fi

# Summary
echo ""
echo -e "${BLUE}Validation Summary${NC}"
echo -e "${BLUE}==================${NC}"
echo -e "Passed: ${GREEN}$PASSES${NC}"
echo -e "Warnings: ${YELLOW}$WARNINGS${NC}"
echo -e "Failures: ${RED}$FAILURES${NC}"
echo ""

if [[ $FAILURES -gt 0 ]]; then
    echo -e "${RED}Validation FAILED with $FAILURES failures${NC}"
    exit 1
elif [[ $WARNINGS -gt 0 ]]; then
    echo -e "${YELLOW}Validation completed with $WARNINGS warnings${NC}"
    exit 0
else
    echo -e "${GREEN}All validations PASSED${NC}"
    exit 0
fi
