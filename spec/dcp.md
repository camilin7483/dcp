# DCP Protocol Specification

> Version 0.1.0 — Initial Draft

## Table of Contents

1. [Overview](#overview)
2. [Transport](#transport)
3. [Framing](#framing)
4. [Session Lifecycle](#session-lifecycle)
5. [Context Query](#context-query)
6. [Event Subscriptions](#event-subscriptions)
7. [Plugin Protocol](#plugin-protocol)
8. [Automation](#automation)
9. [Permissions](#permissions)
10. [Vision](#vision)

## Overview

DCP is a structured protocol that exposes desktop state to AI agents as semantic context.
It follows the JSON-RPC 2.0 specification with custom extensions for event streaming.

### Design Goals

- **Semantic over pixel** — expose meaning, not just visual data
- **Event-driven** — subscribe to changes, don't poll
- **Secure by default** — capability-based permissions, audit logging
- **Cross-platform** — consistent API across Linux, Windows, macOS
- **Extensible** — plugins can add context providers and event sources

## Transport

### Local (default)

Unix domain socket at `$XDG_RUNTIME_DIR/dcpd.sock` (Linux), `\\.\pipe\dcp` (Windows), `$TMPDIR/dcpd.sock` (macOS).

### Remote (optional)

TLS WebSocket with capability token authentication.
Requires explicit user approval for each remote connection.

## Framing

Length-prefixed frames (LSP-style):

```
Content-Length: <N>\r\n
\r\n
<payload of N bytes>
```

Payload is UTF-8 JSON (default) or MessagePack (negotiated via `encoding` parameter in `session.create`).

## Session Lifecycle

```
Client → Server: session.create { capabilities: [...] }
Server → Client: { sessionId, token, expiresAt }

Client → Server: events.subscribe { events: [...] }
Server → Client: { subscriptionId }
Server → Client: { method: "event", params: { ... } }

Client → Server: context.get { selectors: [...] }
Server → Client: { activeWindow: {...}, clipboard: {...} }

Client → Server: session.close { sessionId }
```

## Context Query

### Method: `context.get`

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "context.get",
  "params": {
    "selectors": ["activeWindow", "clipboard", "processes"]
  }
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "activeWindow": {
      "title": "main.rs",
      "application": "Visual Studio Code",
      "semanticContext": "Editing Rust source file"
    },
    "clipboard": {
      "type": "text",
      "content": "fn main() {}"
    },
    "processes": [
      { "pid": 1234, "name": "code", "cpuPercent": 2.5 }
    ]
  }
}
```

### Selectors

| Selector | Description |
|----------|-------------|
| `activeWindow` | Currently focused window |
| `windowTree` | All open windows with metadata |
| `activeApplication` | Application owning the focused window |
| `runningProcesses` | All running processes |
| `clipboard` | Current clipboard content |
| `mouse` | Mouse position + semantic context |
| `keyboardFocus` | Current keyboard focus target |
| `monitors` | Connected displays |
| `systemResources` | CPU, memory, disk usage |
| `network` | Network interfaces + connectivity |
| `audioDevices` | Audio input/output devices |
| `notifications` | Active notifications |
| `power` | Battery/power state |
| `workspace` | Virtual desktop state |
| `installedApps` | Installed applications |
| `terminal` | Active terminal sessions |
| `browser` | Open browser tabs/URLs |
| `files` | Open file handles |

## Event Subscriptions

### Method: `events.subscribe`

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "events.subscribe",
  "params": {
    "events": ["window.focus", "clipboard", "terminal.output"],
    "batch": true,
    "batchIntervalMs": 100
  }
}
```

### Event Notification

```json
{
  "jsonrpc": "2.0",
  "method": "event",
  "params": {
    "subscriptionId": "sub_abc123",
    "event": "window.focus",
    "data": {
      "previous": { "title": "browser", "app": "Firefox" },
      "current": { "title": "main.rs", "app": "VS Code" }
    },
    "timestamp": 1234567890
  }
}
```

### Event Types

| Event | Description |
|-------|-------------|
| `window.focus` | Window focus changed |
| `window.opened` | New window created |
| `window.closed` | Window destroyed |
| `window.moved` | Window position changed |
| `window.resized` | Window size changed |
| `window.title` | Window title changed |
| `window.minimized` | Window minimized |
| `window.restored` | Window restored from minimize |
| `app.launched` | Application started |
| `app.terminated` | Application exited |
| `app.activated` | Application gained focus |
| `clipboard` | Clipboard content changed |
| `selection` | Text selection changed |
| `file.changed` | File modified on disk |
| `file.created` | New file created |
| `file.deleted` | File removed |
| `terminal.exec` | Terminal command executed |
| `terminal.output` | Terminal output received |
| `terminal.cwd` | Terminal working directory changed |
| `browser.tab` | Browser tab activated |
| `browser.url` | Browser URL changed |
| `browser.opened` | New browser tab opened |
| `browser.closed` | Browser tab closed |
| `notification` | Notification received |
| `notification.action` | Notification action triggered |
| `workspace.switch` | Virtual desktop changed |
| `monitor.connected` | Display connected |
| `monitor.disconnected` | Display disconnected |
| `audio.device` | Audio device added/removed |
| `audio.default` | Default audio device changed |
| `network.changed` | Network connectivity changed |
| `power.state` | Power/battery state changed |
| `system.sleep` | System entering sleep |
| `system.wake` | System waking from sleep |
| `screen.locked` | Screen locked |
| `screen.unlocked` | Screen unlocked |
| `plugin.registered` | Plugin registered with daemon |
| `plugin.unregistered` | Plugin unregistered from daemon |

## Plugin Protocol

Plugins communicate with `dcpd` via a dedicated Unix socket per plugin.

### Plugin Registration

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "plugin.register",
  "params": {
    "pluginId": "chrome",
    "version": "1.0.0",
    "capabilities": {
      "providesContext": ["browser"],
      "emitsEvents": ["browser.tab", "browser.url"],
      "handlesAutomation": ["browser.navigate"]
    }
  }
}
```

## Automation

Automation commands are separated from observation via permissions.

### Method: `automation.execute`

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "automation.execute",
  "params": {
    "command": "mouse.move",
    "args": { "x": 100, "y": 200 }
  }
}
```

### Automation Commands

| Command | Description |
|---------|-------------|
| `mouse.move` | Move cursor |
| `mouse.click` | Click at position |
| `mouse.drag` | Drag from/to |
| `keyboard.type` | Type text |
| `keyboard.key` | Press key combo |
| `clipboard.set` | Set clipboard content |
| `app.launch` | Launch application |
| `window.focus` | Focus window |
| `window.move` | Move window |
| `window.resize` | Resize window |
| `file.open` | Open file in default app |

## Permissions

### Capability Format

```
dcp_v1.<session_id_b64>.<perm_hash_b64>.<hmac_signature>
```

### Permission Hierarchy

```
dcp:context:windows:read
dcp:context:clipboard:read
dcp:context:filesystem:read
dcp:context:processes:read
dcp:context:audio:read
dcp:context:network:read
dcp:context:power:read
dcp:automation:mouse:write
dcp:automation:keyboard:write
dcp:automation:clipboard:write
dcp:automation:filesystem:write
dcp:events:window:subscribe
dcp:events:clipboard:subscribe
dcp:events:terminal:subscribe
dcp:events:browser:subscribe
dcp:vision:screen:capture
dcp:vision:window:capture
dcp:vision:ocr:execute
dcp:admin:session:approve
dcp:admin:plugin:install
```

## Vision

Vision is an optional module. When enabled, it provides pixel-based context when native APIs are insufficient.

### Method: `vision.capture`

```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "vision.capture",
  "params": {
    "target": "screen",
    "region": { "x": 0, "y": 0, "width": 1920, "height": 1080 },
    "format": "png",
    "quality": 90
  }
}
```

### Method: `vision.ocr`

```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "method": "vision.ocr",
  "params": {
    "image": "base64_encoded_image_data",
    "region": { "x": 100, "y": 200, "width": 400, "height": 300 }
  }
}
```
