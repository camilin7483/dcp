# Changelog

## v1.0.0 (2026-07-27)

### Breaking Changes
- Session permissions are now **enforced** — all RPC methods require valid capability tokens
- Remote mode (`--remote`) now **requires** `--tls-cert` and `--tls-key` (TLS is mandatory)
- Plugin health checks now properly detect and restart crashed plugins

### Features
- **Multi-platform backends:** Linux (X11/Wayland/Hyprland/Sway), Windows (PowerShell+Win32), macOS (osascript/JXA)
- **Permission system:** HMAC-SHA256 signed tokens, capability-based access control, session expiry
- **WebSocket TLS:** Full TLS 1.3 support via rustls for remote connections
- **Rate limiting:** Token bucket algorithm (100 req/10s default per session)
- **Audit logging:** JSONL audit trail with RPC timing and selector tracking
- **Event system:** Reliable pub/sub with cancelable subscriptions and batching
- **Plugin system:** Process-isolated plugins with health monitoring and auto-restart
- **Metrics:** Prometheus-style counters, gauges, histograms via `daemon.metrics`
- **Health endpoint:** `daemon.health` with uptime, sessions, plugin status

### SDKs
- **Rust:** `dcp-client` crate with full API (query, execute, capture, OCR, subscribe)
- **Python:** Async client with async generator events, reconnect, automation methods
- **TypeScript:** Node.js client with robust buffer handling, reconnect, typed interfaces

### Automation
- Focus tracker, meeting mode, clipboard history scripts
- Open WebUI integration tool
- `dcp-workflow` shell commands for daily use
- systemd service with security hardening

### Performance
- All I/O moved to `tokio::task::spawn_blocking` (no event-loop blocking)
- Token bucket rate limiter prevents abuse
- Context cache with TTL for frequent queries

### Fixes
- Permission enforcement was completely missing (P0 security bug)
- WebSocket TLS was documented but not implemented
- Plugin health check was a no-op (crashes never detected)
- Event subscriptions leaked tasks silently
- Sway IPC used wrong endianness
- OCR temp file had race condition
- `/proc/meminfo` had overflow potential
- Clipboard content-type detection was naive

## v0.1.0 (Initial MVP)
- Basic JSON-RPC daemon with Unix socket transport
- Linux-only platform backend (X11 + Hyprland)
- Session management with HMAC tokens (unenforced)
- Plugin host with example plugins
- Python + TypeScript SDKs (basic)
- CLI tool for query, subscribe, inspect
