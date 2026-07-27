use serde::{Deserialize, Serialize};

/// Types of events the daemon can emit.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EventType {
    // Window events
    WindowFocusChanged,
    WindowOpened,
    WindowClosed,
    WindowMoved,
    WindowResized,
    WindowTitleChanged,
    WindowMinimized,
    WindowRestored,

    // Application events
    ApplicationLaunched,
    ApplicationTerminated,
    ApplicationActivated,

    // Clipboard events
    ClipboardChanged,
    SelectionChanged,

    // File system events
    FileChanged,
    FileCreated,
    FileDeleted,
    FileRenamed,

    // Terminal events
    TerminalCommandExecuted,
    TerminalOutputReceived,
    TerminalCwdChanged,

    // Browser events
    BrowserTabActivated,
    BrowserUrlChanged,
    BrowserTabCreated,
    BrowserTabClosed,

    // Notification events
    NotificationReceived,
    NotificationActionTriggered,

    // Display events
    MonitorConnected,
    MonitorDisconnected,
    WorkspaceSwitched,

    // Audio events
    AudioDeviceAdded,
    AudioDeviceRemoved,
    AudioDefaultChanged,

    // Network events
    NetworkConnectivityChanged,
    NetworkInterfaceChanged,

    // System events
    PowerStateChanged,
    SystemSleep,
    SystemWake,
    ScreenLocked,
    ScreenUnlocked,

    // Plugin events
    PluginRegistered,
    PluginUnregistered,

    // Extension events (plugin-defined)
    Extension(String),
}

impl EventType {
    pub fn name(&self) -> &str {
        match self {
            Self::WindowFocusChanged => "window.focus",
            Self::WindowOpened => "window.opened",
            Self::WindowClosed => "window.closed",
            Self::WindowMoved => "window.moved",
            Self::WindowResized => "window.resized",
            Self::WindowTitleChanged => "window.title",
            Self::WindowMinimized => "window.minimized",
            Self::WindowRestored => "window.restored",
            Self::ApplicationLaunched => "app.launched",
            Self::ApplicationTerminated => "app.terminated",
            Self::ApplicationActivated => "app.activated",
            Self::ClipboardChanged => "clipboard",
            Self::SelectionChanged => "selection",
            Self::FileChanged => "file.changed",
            Self::FileCreated => "file.created",
            Self::FileDeleted => "file.deleted",
            Self::FileRenamed => "file.renamed",
            Self::TerminalCommandExecuted => "terminal.exec",
            Self::TerminalOutputReceived => "terminal.output",
            Self::TerminalCwdChanged => "terminal.cwd",
            Self::BrowserTabActivated => "browser.tab",
            Self::BrowserUrlChanged => "browser.url",
            Self::BrowserTabCreated => "browser.opened",
            Self::BrowserTabClosed => "browser.closed",
            Self::NotificationReceived => "notification",
            Self::NotificationActionTriggered => "notification.action",
            Self::MonitorConnected => "monitor.connected",
            Self::MonitorDisconnected => "monitor.disconnected",
            Self::WorkspaceSwitched => "workspace.switch",
            Self::AudioDeviceAdded => "audio.device.added",
            Self::AudioDeviceRemoved => "audio.device.removed",
            Self::AudioDefaultChanged => "audio.default",
            Self::NetworkConnectivityChanged => "network.changed",
            Self::NetworkInterfaceChanged => "network.interface",
            Self::PowerStateChanged => "power.state",
            Self::SystemSleep => "system.sleep",
            Self::SystemWake => "system.wake",
            Self::ScreenLocked => "screen.locked",
            Self::ScreenUnlocked => "screen.unlocked",
            Self::PluginRegistered => "plugin.registered",
            Self::PluginUnregistered => "plugin.unregistered",
            Self::Extension(name) => name.as_str(),
        }
    }
}

/// A concrete system event with payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemEvent {
    pub event_type: EventType,
    pub data: EventData,
    pub timestamp: i64,
}

impl SystemEvent {
    pub fn new(event_type: EventType, data: EventData) -> Self {
        Self {
            event_type,
            data,
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }
}

/// Event payload data — varies by event type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EventData {
    Window(WindowEventData),
    Application(ApplicationEventData),
    Clipboard(ClipboardEventData),
    File(FileEventData),
    Terminal(TerminalEventData),
    Browser(BrowserEventData),
    Notification(NotificationEventData),
    Monitor(MonitorEventData),
    Audio(AudioEventData),
    Network(NetworkEventData),
    Power(PowerEventData),
    Plugin(PluginEventData),
    Generic(serde_json::Value),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowEventData {
    pub window_id: u64,
    pub title: Option<String>,
    pub application: Option<String>,
    pub pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounds: Option<crate::platform::Rect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationEventData {
    pub name: String,
    pub pid: u32,
    pub executable_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardEventData {
    pub content_type: crate::context::ContentType,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEventData {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalEventData {
    pub terminal_id: String,
    pub pid: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserEventData {
    pub tab_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationEventData {
    pub id: String,
    pub app_name: String,
    pub title: String,
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorEventData {
    pub monitor_id: u64,
    pub name: Option<String>,
    pub resolution: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioEventData {
    pub device_id: String,
    pub device_name: Option<String>,
    pub is_input: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkEventData {
    pub interface_name: Option<String>,
    pub is_connected: bool,
    pub connectivity_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerEventData {
    pub source: String,
    pub battery_percent: Option<f64>,
    pub is_charging: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginEventData {
    pub plugin_id: String,
    pub version: Option<String>,
}

/// Parameters for `events.subscribe`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventsSubscribeParams {
    pub events: Vec<EventType>,
    #[serde(default)]
    pub batch: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_interval_ms: Option<u64>,
}

/// A subscription notification sent from server to client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventNotification {
    pub subscription_id: String,
    pub event: String,
    pub data: EventData,
    pub timestamp: i64,
}
