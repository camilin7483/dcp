use crate::platform::*;
use serde::{Deserialize, Serialize};

/// Selectors for querying context from the daemon.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContextSelector {
    /// Currently focused window.
    ActiveWindow,
    /// All open windows with metadata.
    WindowTree,
    /// Application owning the focused window.
    ActiveApplication,
    /// All running processes.
    RunningProcesses,
    /// Current clipboard content.
    Clipboard,
    /// Mouse position + semantic context.
    Mouse,
    /// Current keyboard focus target.
    KeyboardFocus,
    /// Connected displays.
    Monitors,
    /// CPU, memory, disk usage.
    SystemResources,
    /// Network interfaces + connectivity.
    Network,
    /// Audio input/output devices.
    AudioDevices,
    /// Active notifications.
    Notifications,
    /// Battery/power state.
    Power,
    /// Virtual desktop state.
    Workspace,
    /// Installed applications.
    InstalledApps,
    /// Active terminal sessions.
    Terminals,
    /// Open browser tabs/URLs.
    Browser,
    /// Open file handles in editors.
    OpenFiles,
    /// Selected/highlighted text.
    SelectedText,
    /// Plugin-provided context (extension key).
    Extension(String),
}

impl ContextSelector {
    pub fn name(&self) -> &str {
        match self {
            Self::ActiveWindow => "activeWindow",
            Self::WindowTree => "windowTree",
            Self::ActiveApplication => "activeApplication",
            Self::RunningProcesses => "runningProcesses",
            Self::Clipboard => "clipboard",
            Self::Mouse => "mouse",
            Self::KeyboardFocus => "keyboardFocus",
            Self::Monitors => "monitors",
            Self::SystemResources => "systemResources",
            Self::Network => "network",
            Self::AudioDevices => "audioDevices",
            Self::Notifications => "notifications",
            Self::Power => "power",
            Self::Workspace => "workspace",
            Self::InstalledApps => "installedApps",
            Self::Terminals => "terminals",
            Self::Browser => "browser",
            Self::OpenFiles => "openFiles",
            Self::SelectedText => "selectedText",
            Self::Extension(key) => key.as_str(),
        }
    }
}

/// Unified response containing all requested context data.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextSnapshot {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_window: Option<ActiveWindowInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_tree: Option<Vec<WindowInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_application: Option<ApplicationInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub running_processes: Option<Vec<ProcessInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clipboard: Option<ClipboardData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mouse: Option<MouseInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyboard_focus: Option<FocusInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monitors: Option<Vec<MonitorInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_resources: Option<SystemResources>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<NetworkState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_devices: Option<AudioDevicesInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notifications: Option<Vec<NotificationInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power: Option<PowerState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<WorkspaceInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_apps: Option<Vec<InstalledApp>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminals: Option<Vec<TerminalInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser: Option<BrowserInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_files: Option<Vec<OpenFile>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_text: Option<String>,
    /// Plugin-contributed context, keyed by plugin ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<std::collections::HashMap<String, serde_json::Value>>,
}

/// Active window with semantic context.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveWindowInfo {
    pub id: u64,
    pub title: String,
    pub application: String,
    pub pid: u32,
    pub bounds: Rect,
    pub is_focused: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_context: Option<String>,
}

/// Full application info.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationInfo {
    pub name: String,
    pub pid: u32,
    pub executable_path: String,
    pub version: Option<String>,
    pub is_responding: bool,
}

/// Mouse position with optional semantic context.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MouseInfo {
    pub x: i32,
    pub y: i32,
    pub display_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_context: Option<String>,
}

/// Keyboard focus target.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusInfo {
    pub element_type: String,
    pub description: Option<String>,
    pub window_id: Option<u64>,
}

/// Clipboard data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardData {
    pub content_type: ContentType,
    pub content: String,
    pub timestamp: i64,
}

/// Content type of clipboard or text data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentType {
    Text,
    RichText,
    Html,
    Image,
    File,
    Unknown,
}

/// System resource utilization.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemResources {
    pub cpu_usage_percent: f64,
    pub memory_total_mb: u64,
    pub memory_used_mb: u64,
    pub memory_percent: f64,
    pub swap_total_mb: u64,
    pub swap_used_mb: u64,
    pub disk_read_mb: Option<f64>,
    pub disk_write_mb: Option<f64>,
    pub load_average: Option<[f64; 3]>,
}

/// Network state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkState {
    pub interfaces: Vec<NetworkInterface>,
    pub is_connected: bool,
    pub connectivity_type: ConnectivityType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInterface {
    pub name: String,
    pub ip_addresses: Vec<String>,
    pub mac_address: Option<String>,
    pub is_up: bool,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectivityType {
    None,
    Ethernet,
    Wifi,
    Cellular,
    Vpn,
    Unknown,
}

/// Audio device state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevicesInfo {
    pub inputs: Vec<AudioDevice>,
    pub outputs: Vec<AudioDevice>,
    pub default_input: Option<String>,
    pub default_output: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub volume: Option<f32>,
    pub is_muted: bool,
}

/// Power/battery state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerState {
    pub source: PowerSource,
    pub battery_percent: Option<f64>,
    pub is_charging: bool,
    pub time_remaining_seconds: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PowerSource {
    Battery,
    Ac,
    Unknown,
}

/// Virtual workspace/desktop info.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceInfo {
    pub current: u32,
    pub total: u32,
    pub names: Vec<String>,
}

/// Open browser context.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserInfo {
    pub tabs: Vec<BrowserTab>,
    pub active_tab_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserTab {
    pub id: String,
    pub title: String,
    pub url: String,
    pub is_active: bool,
    pub favicon: Option<String>,
}

/// Terminal session context.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalInfo {
    pub id: String,
    pub pid: u32,
    pub cwd: String,
    pub shell: String,
    pub last_command: Option<String>,
    pub last_output: Option<String>,
    pub title: Option<String>,
}

/// An open file in an editor.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenFile {
    pub path: String,
    pub editor: String,
    pub cursor_line: Option<u32>,
    pub cursor_column: Option<u32>,
    pub is_modified: bool,
}

/// Installed application entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledApp {
    pub name: String,
    pub executable: Option<String>,
    pub version: Option<String>,
    pub category: Option<String>,
}

/// Parameters for `context.get`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextGetParams {
    pub selectors: Vec<ContextSelector>,
}
