//! macOS platform backend: Accessibility APIs + NSWorkspace.

use anyhow::Result;
use async_trait::async_trait;
use dcp_types::*;

use super::PlatformBackend;

pub struct MacOsBackend;

impl MacOsBackend {
    pub async fn new() -> Result<Self> {
        Ok(Self)
    }
}

#[async_trait]
impl PlatformBackend for MacOsBackend {
    async fn active_window(&self) -> Result<ActiveWindowInfo> {
        // macOS: would use NSWorkspace, AX APIs
        Ok(ActiveWindowInfo {
            id: 0,
            title: "macOS (stub)".to_string(),
            application: "unknown".to_string(),
            pid: 0,
            bounds: Rect::new(0, 0, 0, 0),
            is_focused: true,
            semantic_context: None,
        })
    }

    async fn window_tree(&self) -> Result<Vec<WindowInfo>> {
        Ok(vec![])
    }

    async fn running_processes(&self) -> Result<Vec<ProcessInfo>> {
        Ok(vec![])
    }

    async fn clipboard(&self) -> Result<ClipboardData> {
        Ok(ClipboardData {
            content_type: ContentType::Text,
            content: String::new(),
            timestamp: chrono::Utc::now().timestamp_millis(),
        })
    }

    async fn mouse_position(&self) -> Result<MouseInfo> {
        Ok(MouseInfo {
            x: 0,
            y: 0,
            display_id: None,
            semantic_context: None,
        })
    }

    async fn monitors(&self) -> Result<Vec<MonitorInfo>> {
        Ok(vec![])
    }

    async fn system_resources(&self) -> Result<SystemResources> {
        Ok(SystemResources {
            cpu_usage_percent: 0.0,
            memory_total_mb: 0,
            memory_used_mb: 0,
            memory_percent: 0.0,
            swap_total_mb: 0,
            swap_used_mb: 0,
            disk_read_mb: None,
            disk_write_mb: None,
            load_average: None,
        })
    }

    async fn network_state(&self) -> Result<NetworkState> {
        Ok(NetworkState {
            interfaces: vec![],
            is_connected: false,
            connectivity_type: ConnectivityType::Unknown,
        })
    }

    async fn audio_devices(&self) -> Result<AudioDevicesInfo> {
        Ok(AudioDevicesInfo {
            inputs: vec![],
            outputs: vec![],
            default_input: None,
            default_output: None,
        })
    }

    async fn power_state(&self) -> Result<PowerState> {
        Ok(PowerState {
            source: PowerSource::Ac,
            battery_percent: None,
            is_charging: false,
            time_remaining_seconds: None,
        })
    }

    async fn workspace(&self) -> Result<WorkspaceInfo> {
        Ok(WorkspaceInfo {
            current: 0,
            total: 1,
            names: vec!["default".to_string()],
        })
    }

    async fn notifications(&self) -> Result<Vec<NotificationInfo>> {
        Ok(vec![])
    }
}
