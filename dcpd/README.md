# dcpd

The DCP daemon — exposes desktop environment state via JSON-RPC 2.0.

## Features

- **Unix socket transport** with LSP-style length-prefixed framing
- **TLS WebSocket** for secure remote access
- **Capability-based permissions** with HMAC-SHA256 signed tokens
- **Event system** with batching and coalescing
- **Plugin host** with process isolation and auto-restart
- **Rate limiting** with token bucket algorithm
- **Audit logging** in JSONL format
- **Prometheus-style metrics** via RPC
- **Cross-platform:** Linux (X11/Wayland), macOS, Windows

## Quick start

```bash
dcpd --foreground
```

See the [main repo](https://github.com/your-org/dcp) for full documentation.
