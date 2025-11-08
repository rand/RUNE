#!/usr/bin/env bash
# Generate all D2 diagrams to SVG format

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "Generating RUNE architecture diagrams..."

# Check if d2 is installed
if ! command -v d2 &> /dev/null; then
    echo "Error: d2 is not installed"
    echo "Install from: https://d2lang.com/tour/install"
    exit 1
fi

# Generate SVG diagrams
echo "→ Generating request-flow.svg..."
d2 --theme=200 request-flow.d2 request-flow.svg

echo "→ Generating architecture.svg..."
d2 --theme=200 architecture.d2 architecture.svg

echo "✓ All diagrams generated successfully!"
echo ""
echo "Generated files:"
ls -lh *.svg
