# Deployment Guide

## Production Deployment

### Systemd Service (Linux)

DCP includes a systemd user service for production use:

```bash
# Install service
cp scripts/dcpd.service ~/.config/systemd/user/
systemctl --user daemon-reload

# Start on boot
systemctl --user enable dcpd

# Start now
systemctl --user start dcpd

# Check status
systemctl --user status dcpd

# View logs
journalctl --user -u dcpd -f
```

### Security Hardening

The systemd service includes hardening:

```ini
[Service]
NoNewPrivileges=yes
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=%h/.local/share/dcpd
ReadWritePaths=%t/dcpd
SystemCallFilter=@system-service
SystemCallArchitectures=native
MemoryDenyWriteExecute=no

Environment=RUST_LOG=info
```

### Docker Deployment

For CI/testing environments:

```bash
# Build
docker build -t dcpd .

# Run
docker run -d \
  --name dcpd \
  -v /tmp:/tmp \
  -v $HOME/.config/dcpd:/etc/dcpd \
  dcpd
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DCP_HMAC_SECRET` | HMAC signing key | `/etc/machine-id` |
| `DCP_HMAC_SECRET_FILE` | File containing HMAC secret | None |
| `DCP_HMAC_PREVIOUS_SECRETS` | Old secrets for token rotation | None |
| `XDG_RUNTIME_DIR` | Socket directory | `/tmp` |
| `RUST_LOG` | Log level (trace/debug/info/warn/error) | `info` |

### Production Checklist

- [ ] Configure `~/.config/dcpd/config.toml` with desired permissions
- [ ] Set `DCP_HMAC_SECRET` to a strong random value
- [ ] Enable systemd service for auto-start
- [ ] Configure firewall if using remote access
- [ ] Set up log rotation for audit files
- [ ] Configure monitoring (health endpoint)
- [ ] Test backup/restore of config
- [ ] Review plugin permissions

## Scaling

### Connection Limits

DCP limits concurrent connections to 64 by default. Each connection maintains:
- 1 TCP socket (or Unix socket)
- ~16KB memory for frame buffers
- Session state (~1KB)

### Rate Limiting

Default: 100 requests per 10 seconds per session. Configure in `config.toml`:

```toml
[rate_limiter]
max_requests = 200
window_seconds = 10
```

### Audit Log Rotation

Audit logs are stored as daily JSONL files in `~/.local/share/dcpd/audit/`.
Configure logrotate:

```conf
# /etc/logrotate.d/dcpd
/home/user/.local/share/dcpd/audit/*.jsonl {
    daily
    rotate 30
    compress
    missingok
    notifempty
}
```

## Monitoring

### Health Check Endpoint

```bash
dcp status
# or via CLI
watch -n 60 dcp status
```

### Prometheus Metrics

DCP exposes metrics via `daemon.metrics` RPC method. Integrate with Prometheus:

```bash
dcp metrics
```

### Key Metrics to Monitor

| Metric | Alert Threshold | Description |
|--------|----------------|-------------|
| `rpc_calls_total` | - | Total RPC requests |
| `active_sessions` | >50 | Concurrent sessions |
| `rate_limited_total` | >0 | Rate-limited requests |
| `rpc_duration_seconds` | p99 > 1s | Request latency |

## Installation Script

DCP includes an installation script for automated deployment:

```bash
# Install from source
scripts/install.sh

# Uninstall
scripts/uninstall.sh
```

## Release Process

DCP follows semantic versioning. The release script automates tagging and building:

```bash
# Create a new release
scripts/release.sh v1.0.0
```
