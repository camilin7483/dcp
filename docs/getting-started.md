# Getting Started with DCP

## Installation

### Prerequisites
- Rust 1.85+
- Linux: xdotool, xclip (X11) or wl-clipboard, grim (Wayland)
- macOS: osascript (built-in)
- Windows: PowerShell 5+

### From source
```bash
git clone https://github.com/your-org/dcp
cd dcp
cargo build --release
cp target/release/dcpd ~/.local/bin/
cp target/release/dcp ~/.local/bin/
```

### Quick start
```bash
# Start the daemon
dcpd --foreground --verbose &

# Query desktop context
dcp query activeWindow clipboard

# Get full desktop state
dcp inspect

# Subscribe to window focus events
dcp subscribe window.focus
```

### Python client
```bash
cd sdk/python
pip install -e .
```

### TypeScript client
```bash
cd sdk/typescript
npm install
npm run build
```

## Next steps
- Read the API reference: `docs/api-reference.md`
- Write a plugin: `docs/plugins.md`
- Security model: `docs/security.md`
