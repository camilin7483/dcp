<div align="center">

# 🖥️ DCP — Desktop Context Protocol

**Expose your desktop environment to AI agents via JSON-RPC**

[![CI](https://github.com/camilin7483/dcp/actions/workflows/ci.yml/badge.svg)](https://github.com/camilin7483/dcp/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/dcpd)](https://crates.io/crates/dcpd)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](#license)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange)](https://rustup.rs)

[Installation](#installation) • [Quick Start](#quick-start) • [Documentation](docs/) • [SDKs](#sdks) • [Contributing](#contributing)

---

DCP is a **production-grade daemon** that exposes your desktop environment state — windows, clipboard, processes, audio, network, notifications, and more — over **JSON-RPC 2.0** via Unix sockets and TLS WebSocket. Built for AI agents, automation tools, and desktop-aware applications.

**Linux · macOS · Windows**

</div>

## Features

| Category | Capabilities |
|----------|-------------|
| 🪟 **Windows** | Active window, window tree, focus tracking, workspace info |
| 📋 **Clipboard** | Read/write clipboard, content-type detection, selection tracking |
| ⚙️ **Processes** | Running processes, CPU, memory, disk, load average |
| 🖱️ **Input** | Mouse position, keyboard focus, selected text |
| 🖥️ **Display** | Monitor info, resolution, scale, refresh rate |
| 🌐 **Network** | Interfaces, connectivity status, traffic stats |
| 🔊 **Audio** | Input/output devices, volume, mute state |
| 🔋 **Power** | Battery percentage, charging status, power source |
| 🔔 **Notifications** | System notification listener (D-Bus) |
| 📸 **Vision** | Screen capture, OCR (Tesseract), element detection |
| 🤖 **Automation** | Mouse control, keyboard input, clipboard write, app launch |

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        AI Agent / App                        │
├───────────────────────┬─────────────────────────────────────┤
│    Rust SDK / CLI     │  Python SDK  │  TypeScript SDK      │
├───────────────────────┴─────────────────────────────────────┤
│                    JSON-RPC 2.0 (TCP/Unix)                    │
├─────────────────────────────────────────────────────────────┤
│                     dcpd (Rust Daemon)                        │
│  ┌─────────┐ ┌──────────┐ ┌──────────┐ ┌────────────────┐  │
│  │Session  │ │Permission│ │ Rate     │ │ Event Bus      │  │
│  │Manager  │ │Manager   │ │ Limiter  │ │ Pub/Sub        │  │
│  ├─────────┤ ├──────────┤ ├──────────┤ ├────────────────┤  │
│  │ Plugin  │ │ Platform │ │ Vision   │ │ Automation     │  │
│  │ Host    │ │ Backend  │ │ Module   │ │ Executor       │  │
│  └─────────┘ └──────────┘ └──────────┘ └────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                   Linux · macOS · Windows                     │
└─────────────────────────────────────────────────────────────┘
```

## Installation

### From source (recommended)

```bash
git clone https://github.com/camilin7483/dcp.git
cd dcp
cargo build --release
cp target/release/dcpd ~/.local/bin/
cp target/release/dcp ~/.local/bin/
```

### Systemd user service

```bash
cp scripts/dcpd.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now dcpd
```

### Docker

```bash
docker build -t dcpd .
docker run -d --name dcpd -v /tmp:/tmp dcpd
```

### Dependencies by platform

| Platform | Required | Optional |
|----------|----------|----------|
| **Linux X11** | `xdotool`, `xclip`, `xrandr` | `import` (ImageMagick) for screenshots |
| **Linux Wayland** | `wl-clipboard`, `grim` | `hyprctl` (Hyprland) |
| **macOS** | `osascript` (built-in) | None |
| **Windows** | PowerShell 5+ (built-in) | None |

## Quick Start

```bash
# Start the daemon
dcpd --foreground &

# Query desktop context
dcp query activeWindow clipboard

# Full desktop snapshot
dcp inspect

# Listen for window focus events
dcp subscribe window.focus

# Get daemon status
dcp status
```

### Python example

```python
import asyncio
from dcp_client import DcpClient

async def main():
    async with DcpClient() as client:
        ctx = await client.query("activeWindow", "runningProcesses")
        print(f"Active: {ctx.active_window.title}")
        print(f"Processes: {len(ctx.running_processes)}")

asyncio.run(main())
```

### TypeScript example

```typescript
import { DcpClient } from 'dcp-client';

const client = new DcpClient();
await client.connect();
const ctx = await client.query('activeWindow', 'clipboard');
console.log(`Working on: ${ctx.activeWindow.title}`);
await client.close();
```

## Documentation

Comprehensive documentation is available in the [docs/](docs/) directory:

| Document | Description |
|----------|-------------|
| [Getting Started](docs/getting-started.md) | Installation, configuration, first query |
| [Architecture](docs/architecture.md) | System design, data flow, components |
| [API Reference](docs/api-reference.md) | Complete RPC method reference |
| [Security Model](docs/security.md) | Permissions, tokens, threat model |
| [Plugin Development](docs/plugins.md) | Write DCP plugins in Rust |
| [SDK Reference](docs/sdks.md) | Using DCP from Rust, Python, TypeScript |
| [Multi-Platform](docs/multi-platform.md) | Platform-specific features and limitations |
| [Daily Usage](docs/daily-usage.md) | Workflows, automation ideas, scripts |
| [Deployment](docs/deployment.md) | Production deployment, systemd, Docker |
| [Troubleshooting](docs/troubleshooting.md) | Common issues and solutions |

## SDKs

| Language | Status | Location |
|----------|--------|----------|
| **Rust** (`dcp-client`) | ✅ Production | `sdks/rust/dcp-client/` |
| **Python** (`dcp-client`) | ✅ Production | `sdk/python/` |
| **TypeScript** (`dcp-client`) | ✅ Beta | `sdk/typescript/` |

## Security

DCP implements a **capability-based security model**:

- **HMAC-SHA256 tokens** sign sessions with device-specific secrets
- **Fine-grained capabilities** control access to each data type and action
- **Rate limiting** prevents abuse (100 requests per 10 seconds per session)
- **Audit logging** records all RPC calls in JSONL format
- **TLS 1.3** encryption for remote WebSocket connections
- **Session expiry** with automatic cleanup
- **Plugin sandboxing** with process isolation

## Project Status

✅ **v1.0.0 — Production Ready**

The daemon is actively used in daily workflows. All core features are stable and tested.

## License

This project is licensed under either of:

- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.
