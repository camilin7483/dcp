# API Reference

## Transport

DCP uses **JSON-RPC 2.0** over Unix sockets (default) or TLS WebSocket.

### Frame Format

All messages use a 4-byte big-endian length prefix:

```
[4 bytes: content length (BE u32)][N bytes: JSON payload]
```

### Request

```json
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "context.get",
    "params": {
        "selectors": ["activeWindow", "clipboard"]
    }
}
```

### Response (Success)

```json
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "activeWindow": { ... },
        "clipboard": { ... }
    }
}
```

### Response (Error)

```json
{
    "jsonrpc": "2.0",
    "id": 1,
    "error": {
        "code": -32001,
        "message": "Permission denied",
        "data": null
    }
}
```

## Error Codes

| Code | Name | Description |
|------|------|-------------|
| -32700 | ParseError | Invalid JSON |
| -32600 | InvalidRequest | Request object is invalid |
| -32601 | MethodNotFound | Unknown method |
| -32602 | InvalidParams | Invalid method params |
| -32603 | InternalError | Internal daemon error |
| -32000 | SessionExpired | Session token expired |
| -32001 | PermissionDenied | Missing capability |
| -32002 | CapabilityRevoked | Capability was revoked |
| -32003 | SelectorUnavailable | Context selector not available |
| -32004 | EventNotSubscribed | Not subscribed to event type |
| -32005 | PluginNotFound | Plugin not found |
| -32006 | AutomationBlocked | Automation blocked by policy |
| -32007 | VisionNotAvailable | Vision module not enabled |
| -32008 | CaptureFailed | Screen capture failed |
| -32009 | OcrFailed | OCR processing failed |
| -32010 | DaemonShuttingDown | Daemon is shutting down |
| -32011 | RateLimited | Too many requests |

## Methods

### session.create

Create a new authenticated session.

**Request:**
```json
{
    "method": "session.create",
    "params": {
        "clientName": "my-app",
        "capabilities": ["dcp:context:windows:read", "dcp:context:clipboard:read"],
        "encoding": "json"
    }
}
```

**Response:**
```json
{
    "sessionId": "uuid-v4",
    "token": "dcp_v1.<session_id>.<perm_hash>.<expires_at>.<signature>",
    "expiresAt": 1712345678,
    "grantedCapabilities": ["dcp:context:windows:read"],
    "deniedCapabilities": ["dcp:context:clipboard:read"],
    "requiresApproval": false
}
```

### session.close

Close an existing session.

**Request:**
```json
{
    "method": "session.close",
    "params": { "sessionId": "uuid-v4" }
}
```

**Response:**
```json
{
    "success": true
}
```

### context.get

Query desktop context with one or more selectors.

**Request:**
```json
{
    "method": "context.get",
    "params": {
        "selectors": ["activeWindow", "clipboard", "mouse"]
    }
}
```

**Response:**
```json
{
    "activeWindow": {
        "id": 12345,
        "title": "My Document - Editor",
        "application": "code",
        "pid": 45678,
        "bounds": { "x": 0, "y": 0, "width": 1920, "height": 1080 },
        "isFocused": true,
        "semanticContext": "Editing code in VS Code"
    },
    "clipboard": {
        "contentType": "text",
        "content": "copied text here",
        "timestamp": 1712345678000
    },
    "mouse": {
        "x": 960,
        "y": 540,
        "displayId": 0,
        "semanticContext": "Over the main editor panel"
    }
}
```

**Available Selectors:**

| Selector | Returns | Capability Required |
|----------|---------|---------------------|
| `activeWindow` | ActiveWindowInfo | dcp:context:windows:read |
| `windowTree` | [WindowInfo] | dcp:context:windows:read |
| `activeApplication` | ApplicationInfo | dcp:context:windows:read |
| `runningProcesses` | [ProcessInfo] | dcp:context:processes:read |
| `clipboard` | ClipboardData | dcp:context:clipboard:read |
| `mouse` | MouseInfo | dcp:context:mouse:read |
| `keyboardFocus` | FocusInfo | dcp:context:keyboardFocus:read |
| `monitors` | [MonitorInfo] | dcp:context:monitors:read |
| `systemResources` | SystemResources | dcp:context:systemResources:read |
| `network` | NetworkState | dcp:context:network:read |
| `audioDevices` | AudioDevicesInfo | dcp:context:audio:read |
| `power` | PowerState | dcp:context:power:read |
| `workspace` | WorkspaceInfo | dcp:context:workspace:read |
| `notifications` | [NotificationInfo] | dcp:context:notifications:read |
| `installedApps` | [InstalledApp] | dcp:context:installedApps:read |
| `selectedText` | string | dcp:context:selectedText:read |
| `terminals` | [TerminalSession] | dcp:context:terminals:read |
| `browser` | BrowserState | dcp:context:browser:read |
| `openFiles` | [OpenFile] | dcp:context:openFiles:read |

### events.subscribe

Subscribe to desktop events.

**Request:**
```json
{
    "method": "events.subscribe",
    "params": {
        "events": ["window.focus", "clipboard", "notification"],
        "batch": true,
        "batchIntervalMs": 100
    }
}
```

**Response:**
```json
{
    "subscriptionId": "sub-uuid"
}
```

**Event Types:**

| Event | Triggered When | Data |
|-------|---------------|------|
| `window.focus` | Window focus changes | WindowEventData |
| `window.opened` | New window created | WindowEventData |
| `window.closed` | Window destroyed | WindowEventData |
| `window.moved` | Window position changes | WindowEventData |
| `window.resized` | Window size changes | WindowEventData |
| `window.title` | Window title changes | WindowEventData |
| `window.minimized` | Window minimized | WindowEventData |
| `window.restored` | Window restored | WindowEventData |
| `app.launched` | Application starts | AppEventData |
| `app.terminated` | Application exits | AppEventData |
| `app.activated` | Application activated | AppEventData |
| `clipboard` | Clipboard content changes | ClipboardEventData |
| `selection` | Text selection changes | SelectionEventData |
| `file.changed` | Watched file changes | FileEventData |
| `file.created` | New file in watched dir | FileEventData |
| `file.deleted` | File deleted in watched dir | FileEventData |
| `file.renamed` | File renamed in watched dir | FileEventData |
| `terminal.exec` | Terminal command executed | TerminalEventData |
| `terminal.output` | Terminal output received | TerminalEventData |
| `terminal.cwd` | Terminal cwd changed | TerminalEventData |
| `browser.tab` | Browser tab activated | BrowserEventData |
| `browser.url` | Browser URL changed | BrowserEventData |
| `browser.opened` | Browser window opened | BrowserEventData |
| `browser.closed` | Browser window closed | BrowserEventData |
| `notification` | System notification received | NotificationEventData |
| `notification.action` | Notification action triggered | NotificationActionEventData |
| `monitor.connected` | Display connected | MonitorEventData |
| `monitor.disconnected` | Display disconnected | MonitorEventData |
| `workspace.switch` | Virtual desktop switched | WorkspaceEventData |
| `audio.device.added` | Audio device connected | AudioEventData |
| `audio.device.removed` | Audio device removed | AudioEventData |
| `audio.default` | Default audio device changed | AudioEventData |
| `network.changed` | Network connectivity changes | NetworkEventData |
| `network.interface` | Network interface changes | NetworkInterfaceEventData |
| `power.state` | Power state changes | PowerEventData |
| `system.sleep` | System going to sleep | SystemEventData |
| `system.wake` | System woke up | SystemEventData |
| `screen.locked` | Screen locked | SystemEventData |
| `screen.unlocked` | Screen unlocked | SystemEventData |
| `plugin.registered` | Plugin registered | PluginEventData |
| `plugin.unregistered` | Plugin unregistered | PluginEventData |

### automation.execute

Execute automation commands.

**Request:**
```json
{
    "method": "automation.execute",
    "params": {
        "command": {
            "type": "MouseClick",
            "x": 100,
            "y": 200,
            "button": "left"
        },
        "dryRun": false
    }
}
```

**Response:**
```json
{
    "success": true,
    "message": "clicked at (100, 200)"
}
```

**Command Types:**

| Command | Params | Capability |
|---------|--------|------------|
| MouseMove | x, y | automation:mouse:write |
| MouseClick | x, y, button | automation:mouse:write |
| MouseDoubleClick | x, y | automation:mouse:write |
| MouseDrag | from_x, from_y, to_x, to_y | automation:mouse:write |
| MouseScroll | x, y, delta_x, delta_y | automation:mouse:write |
| KeyboardType | text | automation:keyboard:write |
| KeyboardKey | key, modifiers | automation:keyboard:write |
| KeyboardHotkey | keys | automation:keyboard:write |
| ClipboardSet | content, content_type | automation:clipboard:write |
| AppLaunch | executable, args, working_dir | automation:appLaunch:write |
| WindowFocus | window_id | automation:windowManagement:write |
| WindowMove | window_id, x, y | automation:windowManagement:write |
| WindowResize | window_id, width, height | automation:windowManagement:write |
| WindowMinimize | window_id | automation:windowManagement:write |
| WindowMaximize | window_id | automation:windowManagement:write |
| WindowRestore | window_id | automation:windowManagement:write |
| WindowClose | window_id | automation:windowManagement:write |
| FileOpen | path | automation:filesystem:write |

### vision.capture

Capture screen content.

**Request:**
```json
{
    "method": "vision.capture",
    "params": {
        "target": {
            "type": "Screen",
            "monitorId": null
        },
        "format": "png"
    }
}
```

**Response:**
```json
{
    "width": 1920,
    "height": 1080,
    "format": "png",
    "dataBase64": "iVBORw0KGgo...",
    "timestamp": 1712345678000
}
```

**Target Types:**

| Target | Params | Description |
|--------|--------|-------------|
| Screen | monitor_id (optional) | Full screen or specific monitor |
| Window | window_id | Specific window contents |
| Region | bounds (x, y, width, height) | Screen region |

### vision.ocr

Perform OCR on a previously captured image.

**Request:**
```json
{
    "method": "vision.ocr",
    "params": {
        "imageBase64": "iVBORw0KGgo...",
        "language": "eng",
        "region": { "x": 0, "y": 0, "width": 100, "height": 50 }
    }
}
```

**Response:**
```json
{
    "text": "Hello World",
    "confidence": 0.95,
    "textBoxes": [
        {
            "bounds": { "x": 10, "y": 5, "width": 80, "height": 20 },
            "text": "Hello",
            "confidence": 0.95
        }
    ]
}
```

### daemon.status

Get daemon status and statistics.

**Request:**
```json
{
    "method": "daemon.status",
    "params": {}
}
```

**Response:**
```json
{
    "version": "1.0.0",
    "platform": "Linux",
    "uptimeSeconds": 123456,
    "activeSessions": 3,
    "activePlugins": 2,
    "rateLimitedClients": 0
}
```

### daemon.health

Health check endpoint.

**Request:**
```json
{
    "method": "daemon.health",
    "params": {}
}
```

**Response:**
```json
{
    "status": "ok",
    "version": "1.0.0",
    "uptimeSeconds": 123456,
    "activeSessions": 3
}
```

### daemon.metrics

Get Prometheus-style metrics.

**Request:**
```json
{
    "method": "daemon.metrics",
    "params": {}
}
```

**Response:**
```json
{
    "uptimeSeconds": 123456,
    "counters": {
        "rpc_calls_total": 15234
    },
    "gauges": {
        "active_sessions": 3
    },
    "histograms": {
        "rpc_duration_seconds": {
            "buckets": [0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0],
            "counts": [1234, 4567, 7890, 12345, 15000, 15200, 15230, 15234],
            "total": 15234,
            "sum": 45.67,
            "avg": 0.003
        }
    }
}
```

## Capabilities Reference

### Context Read

| Capability String | Permission |
|-------------------|------------|
| dcp:context:windows:read | Read window info and titles |
| dcp:context:clipboard:read | Read clipboard contents |
| dcp:context:processes:read | List running processes |
| dcp:context:filesystem:read | Read file system state |
| dcp:context:audio:read | List audio devices |
| dcp:context:network:read | Read network state |
| dcp:context:power:read | Read battery status |
| dcp:context:monitors:read | Read display info |
| dcp:context:notifications:read | Read notification history |
| dcp:context:workspace:read | Read workspace info |
| dcp:context:mouse:read | Read mouse position |
| dcp:context:keyboardFocus:read | Read keyboard focus |
| dcp:context:systemResources:read | Read CPU/memory/disk |
| dcp:context:installedApps:read | List installed applications |
| dcp:context:selectedText:read | Read selected text |
| dcp:context:terminals:read | Read terminal sessions |
| dcp:context:browser:read | Read browser tabs/URLs |
| dcp:context:openFiles:read | Read open files in editors |

### Automation Write

| Capability String | Permission |
|-------------------|------------|
| dcp:automation:mouse:write | Move/click/drag mouse |
| dcp:automation:keyboard:write | Type text, press keys |
| dcp:automation:clipboard:write | Set clipboard content |
| dcp:automation:filesystem:write | Open files/launch apps |
| dcp:automation:appLaunch:write | Launch applications |
| dcp:automation:windowManagement:write | Focus/move/resize windows |

### Event Subscribe

| Capability String | Permission |
|-------------------|------------|
| dcp:events:window:subscribe | Window focus/open/close |
| dcp:events:clipboard:subscribe | Clipboard changes |
| dcp:events:file:subscribe | File system changes |
| dcp:events:terminal:subscribe | Terminal command/output |
| dcp:events:browser:subscribe | Browser tab/URL changes |
| dcp:events:notification:subscribe | System notifications |
| dcp:events:monitor:subscribe | Display connect/disconnect |
| dcp:events:audio:subscribe | Audio device changes |
| dcp:events:network:subscribe | Network connectivity |
| dcp:events:system:subscribe | Power/sleep/wake events |
| dcp:events:plugin:subscribe | Plugin lifecycle events |

### Vision

| Capability String | Permission |
|-------------------|------------|
| dcp:vision:screen:capture | Capture screen/window |
| dcp:vision:window:capture | Capture specific window |
| dcp:vision:ocr:execute | Perform OCR |
| dcp:vision:elementDetection | Detect UI elements |

### Admin

| Capability String | Permission |
|-------------------|------------|
| dcp:admin:session:approve | Approve session requests |
| dcp:admin:plugin:install | Install plugins |
| dcp:admin:plugin:configure | Configure plugins |
| dcp:admin:audit:read | Read audit logs |

## CLI Usage

```bash
# Query context
dcp query activeWindow clipboard processes

# Subscribe to events
dcp subscribe window.focus clipboard terminal.output

# Show daemon status
dcp status

# Full desktop inspect
dcp inspect

# Create session
dcp session create --name my-agent

# List sessions
dcp session list

# Benchmark
dcp benchmark context.get

# Output formats
dcp --format json query activeWindow
dcp --format pretty query activeWindow
dcp --format table query activeWindow
```

## Python SDK

```python
from dcp_client import DcpClient

async with DcpClient() as client:
    snapshot = await client.query("activeWindow", "clipboard")
    print(snapshot.active_window.title)
    print(snapshot.clipboard.content)

    sub_id = await client.subscribe(["window.focus", "clipboard"], on_event)

    # Full inspect
    full = await client.inspect()

    # Automation
    await client.execute("MouseClick", {"x": 100, "y": 200, "button": "left"})

    # Vision
    capture = await client.capture_screen()
    ocr_result = await client.ocr(capture.image_base64)
```

## TypeScript SDK

```typescript
import { DcpClient } from "dcp-client";

const client = new DcpClient();
await client.connect();

const snapshot = await client.query("activeWindow", "clipboard");
console.log(snapshot.activeWindow?.title);

const subId = await client.subscribe(
  ["window.focus", "clipboard"],
  (event) => console.log("Event:", event)
);

await client.close();
```

## Plugin Development

### Plugin Manifest (dcp-plugin.json)

```json
{
    "plugin_id": "my-plugin",
    "version": "1.0.0",
    "description": "My custom plugin",
    "author": "Your Name",
    "executable": "my-plugin",
    "auto_start": true,
    "capabilities": {
        "provides_context": ["myPlugin.data"],
        "emits_events": ["myPlugin.changed"],
        "handles_automation": ["myPlugin.doThing"]
    }
}
```

### Rust Plugin

```rust
use dcp_plugin_sdk::{Plugin, PluginContext, PluginRegistration, run_plugin};
use async_trait::async_trait;

struct MyPlugin;

#[async_trait]
impl Plugin for MyPlugin {
    fn registration(&self) -> PluginRegistration {
        PluginRegistration {
            plugin_id: "my-plugin".into(),
            version: "1.0.0".into(),
            provides_context: vec!["myPlugin.data".into()],
            emits_events: vec!["myPlugin.changed".into()],
            handles_automation: vec![],
        }
    }

    async fn on_context_request(
        &self,
        ctx: &PluginContext,
        key: &str,
    ) -> Option<serde_json::Value> {
        match key {
            "myPlugin.data" => Some(serde_json::json!({"hello": "world"})),
            _ => None,
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run_plugin(MyPlugin).await
}
```
