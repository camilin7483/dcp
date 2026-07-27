use serde::{Deserialize, Serialize};

/// Platform-agnostic window information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    pub id: u64,
    pub title: String,
    pub application: String,
    pub pid: u32,
    pub bounds: Rect,
    pub is_focused: bool,
    pub is_minimized: bool,
    pub is_maximized: bool,
    pub is_visible: bool,
    pub monitor_id: Option<u64>,
    pub workspace_id: Option<u32>,
    pub parent_id: Option<u64>,
    pub children: Vec<u64>,
}

/// Platform-agnostic process information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInfo {
    pub pid: u32,
    pub parent_pid: Option<u32>,
    pub name: String,
    pub executable_path: Option<String>,
    pub command_line: Option<String>,
    pub cpu_percent: f64,
    pub memory_mb: u64,
    pub status: ProcessStatus,
    pub start_time: i64,
    pub user: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProcessStatus {
    Running,
    Sleeping,
    Stopped,
    Zombie,
    Idle,
    Unknown,
}

/// Rectangle (position + size).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x
            && px < self.x + self.width as i32
            && py >= self.y
            && py < self.y + self.height as i32
    }
}

/// Connected monitor/display information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub id: u64,
    pub name: String,
    pub bounds: Rect,
    pub work_area: Rect,
    pub scale_factor: f64,
    pub refresh_rate_hz: Option<u32>,
    pub is_primary: bool,
    pub rotation: Option<u32>,
}

/// Operating system type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Linux,
    Windows,
    MacOs,
}

/// Desktop session type (Linux-specific).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionType {
    Wayland,
    X11,
    Unknown,
}

/// Notification from the OS.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationInfo {
    pub id: String,
    pub app_name: String,
    pub app_icon: Option<String>,
    pub title: String,
    pub body: Option<String>,
    pub timestamp: i64,
    pub urgency: NotificationUrgency,
    pub actions: Vec<NotificationAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationUrgency {
    Low,
    Normal,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationAction {
    pub id: String,
    pub label: String,
}
