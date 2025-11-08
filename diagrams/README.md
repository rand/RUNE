# RUNE Architecture Diagrams

This directory contains D2 diagram source files for RUNE system architecture visualizations.

## Diagrams

- **`request-flow.d2`**: Request flow through RUNE engine
- **`architecture.d2`**: Complete system architecture with all components

## Generating Diagrams

### Install D2

**macOS (Homebrew)**:
```bash
brew install d2
```

**Linux/macOS (curl)**:
```bash
curl -fsSL https://d2lang.com/install.sh | sh -s --
```

**Other platforms**: See https://d2lang.com/tour/install

### Generate SVG

```bash
# Generate all diagrams
d2 request-flow.d2 request-flow.svg
d2 architecture.d2 architecture.svg

# Or use the provided script
./generate-diagrams.sh
```

### Generate PNG (for non-web use)

```bash
d2 request-flow.d2 request-flow.png
d2 architecture.d2 architecture.png
```

### Live Preview

```bash
# Watch mode (regenerates on file change)
d2 --watch request-flow.d2 request-flow.svg
```

## Integration

These diagrams are referenced in:
- `WHITEPAPER.md` - Technical whitepaper
- `docs/index.md` - GitHub Pages site
- `README.md` - Project overview

## D2 Resources

- Documentation: https://d2lang.com/
- Playground: https://play.d2lang.com/
- Examples: https://d2lang.com/tour/intro
