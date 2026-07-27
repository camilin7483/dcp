# Architecture Guide

## Overview

DCP is a modular, event-driven daemon that collects and exposes desktop environment state. It follows a **layered architecture** with clear separation of concerns.

## System Architecture

### Layer 1: Transport Layer

DCP supports two transport protocols with a unified framing format:

**Unix Socket** (default, local-only):
- Path: `$XDG_RUNTIME_DIR/dcpd.sock` or `/tmp/dcpd.sock`
- Native Linux abstract sockets for performance
- Unix peer credentials for implicit authentication
- LSP-style length-prefixed JSON-RPC 2.0 frames

**TLS WebSocket** (remote access):
- Configurable via `--remote` flag
- TLS 1.3 with rustls
- Optional client certificate authentication
- Same JSON-RPC 2.0 protocol over WebSocket

### Frame Format

All messages use a 4-byte big-endian length prefix followed by JSON or MessagePack payload:

```
┌───────────────────────────────────────┐
│ Content-Length: 4 bytes (BE u32)       │
├───────────────────────────────────────┤
│ Payload: JSON or MessagePack           │
│ (determined by session encoding)       │
└───────────────────────────────────────┘
```

### Layer 2: Session & Permission Layer

Every connection must establish a session before making requests:

1. **Session Creation**: Client sends `session.create` with requested capabilities
2. **Token Issuance**: Daemon validates request, issues HMAC-signed token
3. **Permission Check**: Every subsequent RPC checks capabilities against stored session
4. **Rate Limiting**: Token bucket algorithm enforces per-session quotas

### Layer 3: Service Layer

The daemon's core services:

```
Dispatcher
├── Context Service
│   ├── Window Provider (active window, window tree)
│   ├── Clipboard Provider (read/write, content-type)
│   ├── Process Provider (/proc, ps aux, WMI)
│   ├── Input Provider (mouse, keyboard, selection)
│   ├── Display Provider (monitors, workspaces)
│   ├── System Provider (CPU, memory, disk, load)
│   ├── Network Provider (interfaces, connectivity)
│   ├── Audio Provider (devices, volume, mute)
│   └── Power Provider (battery, charging state)
├── Event Service
│   ├── Window Events (focus, open, close, title)
│   ├── Clipboard Events (content change)
│   ├── File Events (create, modify, delete)
│   ├── Notification Events (D-Bus listener)
│   └── System Events (power, network, audio)
├── Automation Service
│   ├── Mouse Control (move, click, drag, scroll)
│   ├── Keyboard Control (type, hotkeys, modifiers)
│   ├── Clipboard Write
│   ├── App Launch
│   └── Window Management (focus, move, resize)
├── Vision Service
│   ├── Screen Capture (grim, ImageMagick, native)
│   ├── OCR (Tesseract integration)
│   └── Element Detection
├── Plugin Service
│   ├── Plugin Host (process lifecycle)
│   ├── Plugin Discovery (manifest scanning)
│   ├── Health Monitoring (auto-restart)
│   └── Plugin Communication (Unix socket IPC)
└── Admin Service
    ├── Session Management (create, close, list)
    ├── Rate Limiter Administration
    ├── Audit Log Access
    └── Metrics & Health
```

### Layer 4: Platform Abstraction Layer

Each platform implements a common `PlatformBackend` trait:

```rust
#[async_trait]
pub trait PlatformBackend: Send + Sync {
    async fn active_window(&self) -> Result<ActiveWindowInfo>;
    async fn window_tree(&self) -> Result<Vec<WindowInfo>>;
    async fn running_processes(&self) -> Result<Vec<ProcessInfo>>;
    async fn clipboard(&self) -> Result<ClipboardData>;
    async fn mouse_position(&self) -> Result<MouseInfo>;
    async fn monitors(&self) -> Result<Vec<MonitorInfo>>;
    async fn system_resources(&self) -> Result<SystemResources>;
    async fn network_state(&self) -> Result<NetworkState>;
    async fn audio_devices(&self) -> Result<AudioDevicesInfo>;
    async fn power_state(&self) -> Result<PowerState>;
    async fn workspace(&self) -> Result<WorkspaceInfo>;
    async fn notifications(&self) -> Result<Vec<NotificationInfo>>;
    async fn keyboard_focus(&self) -> Result<FocusInfo>;
    async fn installed_apps(&self) -> Result<Vec<InstalledApp>>;
    async fn selected_text(&self) -> Result<Option<String>>;
}
```

Platform backends use native OS APIs:
- **Linux**: `/proc` filesystem, xdotool, xclip, pactl, D-Bus, Hyprland IPC
- **macOS**: osascript (AppleScript/JXA), pbpaste, ps, vm_stat, system_profiler
- **Windows**: PowerShell, WMI, Win32 P/Invoke via PowerShell

## Data Flow

### Request Flow

```
Client                    dcpd
  │                        │
  │── Unix Connect ──────►│
  │                        │
  │── session.create ─────►│
  │                        ├── PermissionManager.create_token()
  │                        ├── SessionManager.store_session()
  │◄── session_id + token ─┤
  │                        │
  │── context.get ────────►│
  │                        ├── RateLimiter.allow() ✓
  │                        ├── PermissionManager.verify() ✓
  │                        ├── Dispatcher.route()
  │                        ├── PlatformBackend.active_window()
  │                        ├── AuditLogger.log()
  │◄── context snapshot ───┤
  │                        │
```

### Event Flow

```
System Event                  dcpd                    Client
  │                           │                        │
  │── Window Focus Change ──► │                        │
  │                           ├── EventBus.publish()   │
  │                           ├── Match subscriptions  │
  │                           ├── Serialize + queue    │
  │                           │── JSON-RPC event ─────►│
  │                           │                        │
```

## Security Architecture

### Permission Model

```
Client Request
    │
    ▼
┌─────────────────────┐
│ Rate Limiter Check  │── Exceeded? → ErrorCode::RateLimited
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│ Session Valid?      │── Expired? → ErrorCode::SessionExpired
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│ Has Capability?     │── No? → ErrorCode::PermissionDenied
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│ Execute + Audit Log │
└─────────────────────┘
```

### Capability Strings

Capabilities use a hierarchical dot-separated format:

```
dcp:context:windows:read
 │    │       │       │
 │    │       │       └── action (read/write/subscribe)
 │    │       └────────── resource (windows, clipboard, audio...)
 │    └────────────────── domain (context, automation, events, vision)
 └─────────────────────── protocol prefix (dcp)
```

## Plugin Architecture

Plugins are **separate processes** communicating with the daemon over Unix sockets:

```
dcpd                        Plugin Process
  │                            │
  │── Spawn child process ───► │
  │                            │
  │◄── Plugin handshake ───────┤
  │                            │
  │── Health check (30s) ────► │
  │◄── OK ─────────────────────┤
  │                            │
  │── Context request ────────►│
  │◄── Plugin context data ────┤
  │                            │
  │ (if crash detected)        │
  │── Auto-restart (max 3x) ──►│
  │                            │
```

## Performance

The daemon achieves sub-millisecond latency for most queries:

| Operation | Latency (p50) | Latency (p95) |
|-----------|--------------|--------------|
| `daemon.status` | 0.1ms | 0.3ms |
| `context.get` (activeWindow) | 0.8ms | 2.1ms |
| `context.get` (clipboard) | 1.2ms | 3.5ms |
| `context.get` (multiple selectors) | 2.5ms | 8.0ms |
| `events.subscribe` | 0.5ms | 1.5ms |
| `session.create` | 0.3ms | 0.8ms |

Benchmark results on Ryzen 7 5800X, Linux 6.8, Hyprland.

## Directory Structure

```
dcp/
├── dcpd/                  # Core daemon
│   ├── src/
│   │   ├── main.rs        # Entry point with CLI args
│   │   ├── lib.rs         # Daemon bootstrap + signal handling
│   │   ├── server/        # JSON-RPC server + session management
│   │   ├── permissions/   # HMAC tokens + capability enforcement
│   │   ├── platform/      # OS-specific backends (linux/macos/windows)
│   │   ├── plugins/       # Plugin host + IPC
│   │   ├── events/        # Event bus + subscriptions
│   │   ├── automation/    # Mouse/keyboard/app automation
│   │   ├── vision/        # Screen capture + OCR
│   │   ├── cache/         # Context cache with TTL
│   │   ├── audit/         # Audit logging
│   │   ├── metrics.rs     # Prometheus-style metrics
│   │   ├── ratelimit.rs   # Token bucket rate limiter
│   │   ├── config.rs      # TOML configuration
│   │   ├── terminal.rs    # Terminal session detection
│   │   ├── watcher.rs     # File system watcher
│   │   ├── websocket.rs   # TLS WebSocket transport
│   │   ├── dbus.rs        # D-Bus notification listener
│   │   └── wayland.rs     # Native Wayland protocol support
│   └── tests/
│       └── protocol_test.rs  # Integration tests
├── dcp-types/             # Shared protocol types
├── dcp-cli/               # Command-line client
├── plugins/               # Plugin SDK + examples
├── sdks/                  # Rust/Python/TypeScript SDKs
├── scripts/               # Daily usage scripts
├── docs/                  # Documentation
└── integrations/          # Third-party integrations
```

## Configuration

DCP uses TOML configuration at `~/.config/dcpd/config.toml`:

```toml
log_level = "info"
vision = false

remote = false
# remote_addr = "0.0.0.0:9527"
# tls_cert = "/path/to/cert.pem"
# tls_key = "/path/to/key.pem"

default_permissions = [
    "dcp:context:windows:read",
    "dcp:context:clipboard:read",
    "dcp:context:processes:read",
]

watch_paths = ["/home/user/projects"]
```

## Testing

DCP has a comprehensive test suite:

```
Unit Tests (57)
├── Protocol Types (13)
├── Permissions (11)
├── Rate Limiter (9)
├── Metrics (7)
├── Audit (5)
├── OCR (8)
├── Capture (5)

Integration Tests (13)
├── Daemon Lifecycle
├── RPC Methods
├── Permission Enforcement
├── Event Subscription
├── Automation
├── Vision
├── Health & Metrics

Doc Tests (2)
├── dcp-client
├── dcp-plugin-sdk
```
