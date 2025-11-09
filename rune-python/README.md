# RUNE Python Bindings

Python bindings for the RUNE authorization engine using PyO3.

## Status

⚠️ **Currently disabled in workspace** - Requires proper Python development environment setup.

## Prerequisites

To build the Python bindings, you need:

1. **Python 3.9+** with development headers
2. **PyO3** compatible Python installation
3. Properly configured Python library paths

### macOS Setup

```bash
# Install Python via Homebrew
brew install python@3.11

# Verify Python configuration
python3-config --ldflags
```

### Linux Setup

```bash
# Install Python development packages
sudo apt-get install python3-dev python3-pip  # Debian/Ubuntu
sudo dnf install python3-devel python3-pip     # Fedora
```

## Building

Once prerequisites are met:

1. Enable in workspace by uncommenting `rune-python` in `/Cargo.toml`
2. Build the extension:
   ```bash
   cargo build -p rune-python --release
   ```

## API Updates

The Python bindings have been updated to use the modern `RequestBuilder` API pattern:

```python
from rune_python import RUNE

# Create engine
engine = RUNE()

# Authorize request
result = engine.authorize(
    action="read",
    principal="user-123",
    resource="/data/file.txt",
    context={"ip": "192.168.1.1"}
)
```

## Features

- **Authorization**: Single and batch authorization requests
- **Fact Management**: Add facts to the engine
- **Cache Control**: Clear cache and get statistics
- **Decorator Support**: `@RequirePermission` decorator (in development)

## Development

### Running Tests

```bash
cargo test -p rune-python
```

### Python Integration Tests

```bash
cd rune-python
python3 -m pytest tests/
```

## Known Issues

1. **Linking errors on macOS with Python 3.14**: PyO3 0.20 may not be fully compatible with bleeding-edge Python versions
2. **Missing Python symbols**: Ensure Python is installed with shared libraries enabled

## Future Work

- [ ] Async/await support for authorization
- [ ] Context manager for scoped rules
- [ ] Complete decorator implementation
- [ ] Python wheel packaging
- [ ] Documentation generation from Rust docstrings