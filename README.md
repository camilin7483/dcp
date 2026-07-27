# DCP — Desktop Context Protocol

## Overview

DCP is a high-performance, cross-platform desktop context protocol that enables AI agents to understand and interact with the user's desktop environment in real-time.

**Current Status**: Phase 2 Complete (Prototype)  
**Version**: 0.1.0  
**License**: MIT OR Apache-2.0

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    dcp CLI / SDKs                           │
│         (query/subscribe/inspect/benchmark)                 │
└──────────────────────────┬──────────────────────────────────┘
                           │ Unix Socket
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                      dcpd Daemon                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              Core Modules                            │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐            │  │
│  │  │  Server  │ │  Events  │ │  Cache   │            │  │
│  │  │ (RPC +   │ │   Bus    │ │  (TTL)   │            │  │
│  │  │ Session) │ │(Batching)│ │          │            │  │
│  │  └──────────┘ └──────────┘ └──────────┘            │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐            │  │
│  │  │Automaton │ │ Platform │ │   D-Bus  │            │  │
│  │  │ Executor │ │  Backend │ │ Listener │            │  │
│  │  └──────────┘ └──────────┘ └──────────┘            │  │
│  └──────────────────────────────────────────────────────┘  │
└──────────────────────────┬──────────────────────────────────┘
                           │
        ┌──────────────────┼──────────────────┐
        ▼                  ▼                  ▼
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│    Linux     │  │   Windows    │  │    macOS     │
│  (xdotool,   │  │  (Win32,     │  │  (AX APIs,   │
│   /proc,     │  │   UIA,       │  │   NSWorkspace│
│   grim,      │  │   COM)       │  │   CGWindow)  │
│   zbus)      │  │              │  │              │
└──────────────┘  └──────────────┘  └──────────────┘
```

## Features Implemented

### Phase 1: Foundation
- ✅ Core protocol types (JSON-RPC 2.0, MessagePack support)
- ✅ Workspace structure (7 crates)
- ✅ Platform backend trait + 3 OS stubs
- ✅ Daemon skeleton with Unix socket server
- ✅ Event bus with basic pub/sub
- ✅ Permission system (HMAC-SHA256 tokens)
- ✅ CLI tool (query/subscribe/status/session/inspect/benchmark)
- ✅ Plugin SDK (trait-based, process isolation)
- ✅ Python SDK (async client)
- ✅ TypeScript SDK (Node.js client)

### Phase 2: Core Functionality
- ✅ Event batching with time windows and coalescing
- ✅ Automation executor (Linux: xdotool, xclip, wl-copy)
  - Mouse: move, click, double-click, drag, scroll
  - Keyboard: type text, press keys, hotkeys
  - Clipboard: set text (X11 + Wayland)
  - Window management: focus, move, resize, minimize, maximize, close
  - Applications: launch, open files
- ✅ Enhanced Linux backend
  - Active window with semantic context (source code, terminal, browser)
  - Window tree (X11 via xdotool)
  - Process list with memory/CPU from /proc
  - Clipboard (X11: xclip, Wayland: wl-paste)
  - Mouse position with screen info
  - Monitors (X11: xrandr)
  - System resources (CPU, memory, load average from /proc)
  - Network state (from /sys/class/net)
  - Audio devices (PulseAudio via pactl)
  - Power state (battery from /sys/class/power_supply)
  - Workspace detection (X11: xprop, tiling WMs)
- ✅ D-Bus integration
  - Notification listener (org.freedesktop.Notifications)
- ✅ Vision capture
  - Screen capture (X11: ImageMagick import, Wayland: grim)
  - Window capture
  - Region capture
  - PNG dimension parsing

## Project Structure

```
dcp/
├── Cargo.toml                 # Workspace root
├── dcp-types/                 # Protocol types (shared)
├── dcpd/                      # Core daemon
│   ├── src/
│   │   ├── server/            # RPC dispatcher, session manager
│   │   ├── events/            # Event bus with batching
│   │   ├── automation/        # Mouse/keyboard/clipboard control
│   │   ├── platform/          # OS-specific backends
│   │   ├── dbus.rs            # D-Bus notification listener
│   │   ├── vision/            # Screen capture
│   │   ├── cache/             # TTL cache
│   │   ├── permissions/       # Capability tokens
│   │   ├── audit/             # Audit logging
│   │   └── plugins/           # Plugin host
├── dcp-cli/                   # CLI tool
├── plugins/
│   ├── dcp-plugin-sdk/        # Plugin development SDK
│   └── example-plugin/        # Reference implementation
├── sdk/
│   ├── python/                # Python async client
│   └── typescript/            # TypeScript client
├── spec/                      # Protocol specification
└── tests/                     # Integration tests + benchmarks
```

## Quick Start

### Build
```bash
cd ~/Projects/dcp
cargo build --workspace
```

### Run Daemon
```bash
./target/debug/dcpd --verbose
```

### Query Context
```bash
./target/debug/dcp query activeWindow clipboard processes
```

### Subscribe to Events
```bash
./target/debug/dcp subscribe window.focus clipboard
```

### Inspect Full Context
```bash
./target/debug/dcp inspect
```

### Benchmark
```bash
./target/debug/dcp benchmark context.get
```

## Protocol Examples

### Query Active Window
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "context.get",
  "params": {
    "selectors": ["activeWindow"]
  }
}
```

Response:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "activeWindow": {
      "id": 12345,
      "title": "main.rs — dcp",
      "application": "Visual Studio Code",
      "pid": 1234,
      "bounds": {"x": 100, "y": 100, "width": 1200, "height": 800},
      "isFocused": true,
      "semanticContext": "Editing source code"
    }
  }
}
```

### Subscribe to Events
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "events.subscribe",
  "params": {
    "events": ["window.focus", "clipboard.changed"],
    "batch": true,
    "batchIntervalMs": 100
  }
}
```

Event notification:
```json
{
  "jsonrpc": "2.0",
  "method": "event",
  "params": {
    "subscriptionId": "sub_abc123",
    "eventType": "window.focus",
    "data": {
      "windowId": 67890,
      "title": "Terminal",
      "application": "Alacritty"
    },
    "timestamp": 1234567890
  }
}
```

### Automation
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "automation.execute",
  "params": {
    "command": "keyboard.type",
    "args": {"text": "Hello, world!"}
  }
}
```

### Vision Capture
```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "vision.capture",
  "params": {
    "target": {"Screen": {"monitorId": null}},
    "format": "png",
    "quality": 90
  }
}
```

## Platform Support

| Feature | Linux (X11) | Linux (Wayland) | Windows | macOS |
|---------|-------------|-----------------|---------|-------|
| Active window | ✅ xdotool | ⚠️ stub | ⚠️ stub | ⚠️ stub |
| Window tree | ✅ xdotool | ⚠️ stub | ⚠️ stub | ⚠️ stub |
| Process list | ✅ /proc | ✅ /proc | ⚠️ stub | ⚠️ stub |
| Clipboard | ✅ xclip | ✅ wl-paste | ⚠️ stub | ⚠️ stub |
| Mouse position | ✅ xdotool | ✅ xdotool | ⚠️ stub | ⚠️ stub |
| Monitors | ✅ xrandr | ⚠️ stub | ⚠️ stub | ⚠️ stub |
| System resources | ✅ /proc | ✅ /proc | ⚠️ stub | ⚠️ stub |
| Network state | ✅ /sys | ✅ /sys | ⚠️ stub | ⚠️ stub |
| Audio devices | ✅ pactl | ✅ pactl | ⚠️ stub | ⚠️ stub |
| Power state | ✅ /sys | ✅ /sys | ⚠️ stub | ⚠️ stub |
| Workspace | ✅ xprop | ✅ env | ⚠️ stub | ⚠️ stub |
| Notifications | ✅ D-Bus | ✅ D-Bus | ⚠️ stub | ⚠️ stub |
| Automation | ✅ full | ✅ full | ⚠️ stub | ⚠️ stub |
| Vision capture | ✅ import | ✅ grim | ⚠️ stub | ⚠️ stub |

## Dependencies

### Rust
- `tokio` — Async runtime
- `serde` + `serde_json` — Serialization
- `zbus` — D-Bus integration (Linux)
- `hmac` + `sha2` — Capability token signing
- `uuid` — Session/subscription IDs
- `clap` — CLI parsing
- `async-trait` — Async trait support

### External Tools (Linux)
- `xdotool` — Window management, mouse/keyboard automation
- `xclip` / `wl-paste` — Clipboard operations
- `xrandr` — Monitor information
- `import` (ImageMagick) — X11 screenshots
- `grim` — Wayland screenshots
- `pactl` — Audio device information

## Statistics

- **Total files**: 50
- **Rust code**: ~4,946 lines
- **Python SDK**: ~484 lines
- **TypeScript SDK**: ~376 lines
- **Total**: ~5,800 lines

## Next Steps (Phase 3)

- [ ] Wayland native protocols (wlr-foreign-toplevel, zwlr-screencopy)
- [ ] Windows backend (Win32, UI Automation)
- [ ] macOS backend (Accessibility APIs)
- [ ] OCR integration (Tesseract)
- [ ] UI element detection
- [ ] File system watcher (inotify)
- [ ] Terminal output capture
- [ ] Browser integration plugins
- [ ] WASM plugin sandbox
- [ ] TLS WebSocket transport
- [ ] More comprehensive tests

## License

SPDX-License-Identifier: `MIT OR Apache-2.0`

See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE) for details.
