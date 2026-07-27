# DCP API Reference

## Protocol

DCP uses JSON-RPC 2.0 over Unix domain sockets (local) or TLS WebSockets (remote).

### Frame Format

```
Content-Length: <N>\r\n
Content-Type: application/json\r\n
\r\n
<payload of N bytes>
```

### Connection

```
socket: $XDG_RUNTIME_DIR/dcpd.sock
```

---

## Methods

### `session.create`

Create a new session with the daemon.

**Request:**
```json
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "session.create",
    "params": {
        "clientName": "my-agent",
        "capabilities": ["dcp:context:windows:read", "dcp:events:window:subscribe"],
        "encoding": "json"
    }
}
```

**Response:**
```json
{
    "sessionId": "uuid",
    "token": "dcp_v1.<session_id>.<perm_hash>.<signature>",
    "expiresAt": 1234567890,
    "grantedCapabilities": [...],
    "deniedCapabilities": [...],
    "requiresApproval": false
}
```

---

### `context.get`

Query current desktop context.

**Request:**
```json
{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "context.get",
    "params": {
        "selectors": ["activeWindow", "clipboard", "processes", "mouse"]
    }
}
```

**Selectors:**
| Selector | Description |
|----------|-------------|
| `activeWindow` | Currently focused window |
| `windowTree` | All open windows |
| `activeApplication` | Application owning focused window |
| `runningProcesses` | All running processes |
| `clipboard` | Current clipboard content |
| `mouse` | Mouse position + semantic context |
| `keyboardFocus` | Current keyboard focus target |
| `monitors` | Connected displays |
| `systemResources` | CPU, memory, disk usage |
| `network` | Network interfaces |
| `audioDevices` | Audio input/output |
| `notifications` | Active notifications |
| `power` | Battery/power state |
| `workspace` | Virtual desktop state |
| `installedApps` | Installed applications |
| `terminals` | Terminal sessions |
| `browser` | Browser tabs/URLs (plugin) |
| `openFiles` | Open files in editors |
| `selectedText` | Current text selection |

**Response:**
```json
{
    "activeWindow": {
        "id": 12345,
        "title": "main.rs — dcp",
        "application": "Visual Studio Code",
        "pid": 1234,
        "bounds": {"x": 100, "y": 100, "width": 1200, "height": 800},
        "isFocused": true,
        "semanticContext": "Editing Rust source file"
    },
    "clipboard": {
        "contentType": "text",
        "content": "fn main() {}",
        "timestamp": 1234567890
    },
    "mouse": {
        "x": 1420,
        "y": 801,
        "displayId": 0,
        "semanticContext": "Hovering the 'Run' button"
    }
}
```

---

### `events.subscribe`

Subscribe to real-time events.

**Request:**
```json
{
    "jsonrpc": "2.0",
    "id": 3,
    "method": "events.subscribe",
    "params": {
        "events": ["window.focus", "clipboard", "terminal.output"],
        "batch": true,
        "batchIntervalMs": 100
    }
}
```

**Response:**
```json
{
    "subscriptionId": "sub_abc123"
}
```

**Event Types:**

| Category | Events |
|----------|--------|
| Window | `window.focus`, `window.opened`, `window.closed`, `window.moved`, `window.resized`, `window.title`, `window.minimized`, `window.restored` |
| Application | `app.launched`, `app.terminated`, `app.activated` |
| Clipboard | `clipboard`, `selection` |
| File | `file.changed`, `file.created`, `file.deleted`, `file.renamed` |
| Terminal | `terminal.exec`, `terminal.output`, `terminal.cwd` |
| Browser | `browser.tab`, `browser.url`, `browser.opened`, `browser.closed` |
| Notification | `notification`, `notification.action` |
| Display | `monitor.connected`, `monitor.disconnected`, `workspace.switch` |
| Audio | `audio.device.added`, `audio.device.removed`, `audio.default` |
| Network | `network.changed`, `network.interface` |
| System | `power.state`, `system.sleep`, `system.wake`, `screen.locked`, `screen.unlocked` |
| Plugin | `plugin.registered`, `plugin.unregistered` |

**Event Notification:**
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
            "application": "Alacritty",
            "pid": 5678
        },
        "timestamp": 1234567890
    }
}
```

---

### `automation.execute`

Execute an automation command.

**Request:**
```json
{
    "jsonrpc": "2.0",
    "id": 4,
    "method": "automation.execute",
    "params": {
        "command": "mouse.move",
        "args": {"x": 100, "y": 200},
        "dryRun": false
    }
}
```

**Commands:**
| Command | Args |
|---------|------|
| `mouse.move` | `x`, `y` |
| `mouse.click` | `x`, `y`, `button` (left/right/middle) |
| `mouse.doubleClick` | `x`, `y` |
| `mouse.drag` | `fromX`, `fromY`, `toX`, `toY` |
| `mouse.scroll` | `x`, `y`, `deltaX`, `deltaY` |
| `keyboard.type` | `text` |
| `keyboard.key` | `key`, `modifiers` |
| `keyboard.hotkey` | `keys` |
| `clipboard.set` | `content`, `contentType` |
| `app.launch` | `executable`, `args`, `workingDir` |
| `window.focus` | `windowId` |
| `window.move` | `windowId`, `x`, `y` |
| `window.resize` | `windowId`, `width`, `height` |
| `window.minimize` | `windowId` |
| `window.maximize` | `windowId` |
| `window.restore` | `windowId` |
| `window.close` | `windowId` |
| `file.open` | `path` |

**Response:**
```json
{
    "success": true,
    "message": "mouse moved"
}
```

---

### `vision.capture`

Capture screen/window/region as image.

**Request:**
```json
{
    "jsonrpc": "2.0",
    "id": 5,
    "method": "vision.capture",
    "params": {
        "target": {"Screen": {"monitorId": null}},
        "format": "png",
        "quality": 90
    }
}
```

**Target Types:**
- `Screen` — full screen or specific monitor
- `Window` — specific window by ID
- `Region` — rectangular region with bounds

**Response:**
```json
{
    "width": 1920,
    "height": 1080,
    "format": "png",
    "dataBase64": "iVBORw0KGgoAAAANS...",
    "timestamp": 1234567890
}
```

---

### `vision.ocr`

Perform OCR on an image.

**Request:**
```json
{
    "jsonrpc": "2.0",
    "id": 6,
    "method": "vision.ocr",
    "params": {
        "imageBase64": "iVBORw0KGgoAAAANS...",
        "region": {"x": 100, "y": 200, "width": 400, "height": 300},
        "language": "eng"
    }
}
```

**Response:**
```json
{
    "text": "Hello, world!",
    "confidence": 0.95,
    "textBoxes": [
        {
            "bounds": {"x": 100, "y": 200, "width": 150, "height": 30},
            "text": "Hello,",
            "confidence": 0.97
        },
        {
            "bounds": {"x": 260, "y": 200, "width": 100, "height": 30},
            "text": "world!",
            "confidence": 0.93
        }
    ]
}
```

---

### `daemon.status`

Get daemon status.

**Request:**
```json
{
    "jsonrpc": "2.0",
    "id": 7,
    "method": "daemon.status",
    "params": {}
}
```

**Response:**
```json
{
    "version": "0.1.0",
    "platform": "Linux",
    "activeSessions": 3,
    "uptimeSeconds": 3600
}
```

---

## Permissions

### Capability Hierarchy

```
dcp:context:windows:read
dcp:context:clipboard:read
dcp:context:filesystem:read
dcp:context:processes:read
dcp:context:audio:read
dcp:context:network:read
dcp:context:power:read
dcp:context:monitors:read
dcp:context:notifications:read
dcp:context:workspace:read
dcp:context:installedApps:read
dcp:context:terminals:read
dcp:context:browser:read
dcp:context:openFiles:read
dcp:context:selectedText:read
dcp:context:mouse:read
dcp:context:keyboardFocus:read
dcp:context:systemResources:read

dcp:automation:mouse:write
dcp:automation:keyboard:write
dcp:automation:clipboard:write
dcp:automation:filesystem:write
dcp:automation:appLaunch:write
dcp:automation:windowManagement:write

dcp:events:window:subscribe
dcp:events:clipboard:subscribe
dcp:events:file:subscribe
dcp:events:terminal:subscribe
dcp:events:browser:subscribe
dcp:events:notification:subscribe
dcp:events:monitor:subscribe
dcp:events:audio:subscribe
dcp:events:network:subscribe
dcp:events:system:subscribe
dcp:events:plugin:subscribe

dcp:vision:screen:capture
dcp:vision:window:capture
dcp:vision:ocr:execute
dcp:vision:elementDetection

dcp:admin:session:approve
dcp:admin:plugin:install
dcp:admin:plugin:configure
dcp:admin:audit:read
```

---

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
dcp benchmark 100

# Output formats
dcp --format json query activeWindow
dcp --format pretty query activeWindow
dcp --format table query activeWindow
```

---

## Python SDK

```python
from dcp_client import DcpClient

async with DcpClient() as client:
    # Query context
    snapshot = await client.query("activeWindow", "clipboard")
    print(snapshot.active_window.title)
    print(snapshot.clipboard.content)

    # Subscribe to events
    def on_event(event):
        print(f"Event: {event}")

    sub_id = await client.subscribe(["window.focus", "clipboard"], on_event)

    # Full inspect
    full = await client.inspect()
```

---

## TypeScript SDK

```typescript
import { DcpClient } from "dcp-client";

const client = new DcpClient();
await client.connect();

// Query context
const snapshot = await client.query("activeWindow", "clipboard");
console.log(snapshot.activeWindow?.title);

// Subscribe to events
const subId = await client.subscribe(["window.focus", "clipboard"], (event) => {
    console.log("Event:", event);
});

await client.close();
```

---

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

    async fn on_context_request(&self, ctx: &PluginContext, key: &str) -> Option<serde_json::Value> {
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
