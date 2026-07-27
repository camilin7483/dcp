# Getting Started with DCP

## Installation

### Prerequisites
- Rust 1.85+
- Linux: xdotool, xclip (X11) or wl-clipboard, grim (Wayland)
- macOS: osascript (built-in)
- Windows: PowerShell 5+

### From source
```bash
git clone https://github.com/camilin7483/dcp.git
cd dcp
cargo build --release
cp target/release/dcpd ~/.local/bin/
cp target/release/dcp ~/.local/bin/
```

### Systemd user service (Linux)
```bash
cp scripts/dcpd.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now dcpd
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

## Next Steps

| Document | Description |
|----------|-------------|
| [Architecture](architecture.md) | System design and data flow |
| [API Reference](api-reference.md) | Complete RPC method reference |
| [Security Model](security.md) | Permissions, tokens, threat model |
| [Plugin Development](plugins.md) | Write DCP plugins in Rust |
| [SDK Reference](sdks.md) | Using DCP from Rust, Python, TypeScript |
| [Multi-Platform](multi-platform.md) | Platform-specific features and limitations |
| [Daily Usage](daily-usage.md) | Workflows, automation ideas, scripts |
| [Deployment](deployment.md) | Production deployment, systemd, Docker |
| [Troubleshooting](troubleshooting.md) | Common issues and solutions |
