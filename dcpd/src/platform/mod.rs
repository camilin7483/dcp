//! Platform backend: OS-specific implementations for context collection.

pub mod linux;
pub mod windows;
pub mod macos;

use anyhow::Result;
use dcp_types::*;
use std::sync::Arc;
use async_trait::async_trait;

use crate::automation::AutomationBackend;

/// Current platform type.
pub fn current_platform() -> Platform {
    if cfg!(target_os = "linux") {
        Platform::Linux
    } else if cfg!(target_os = "windows") {
        Platform::Windows
    } else if cfg!(target_os = "macos") {
        Platform::MacOs
    } else {
        Platform::Linux
    }
}

/// Create the appropriate platform backend for the current OS.
pub async fn create_backend() -> Result<Arc<dyn PlatformBackend>> {
    match current_platform() {
        Platform::Linux => {
            let backend = linux::LinuxBackend::new().await?;
            Ok(Arc::new(backend))
        }
        Platform::Windows => {
            let backend = windows::WindowsBackend::new().await?;
            Ok(Arc::new(backend))
        }
        Platform::MacOs => {
            let backend = macos::MacOsBackend::new().await?;
            Ok(Arc::new(backend))
        }
    }
}

/// Create the automation backend for the current OS.
pub fn create_automation() -> Option<Arc<dyn AutomationBackend>> {
    match current_platform() {
        Platform::Linux => {
            use crate::automation::executor::LinuxAutomation;
            Some(Arc::new(LinuxAutomation::new()))
        }
        _ => None,
    }
}

/// Platform-agnostic backend trait.
#[async_trait]
pub trait PlatformBackend: Send + Sync {
    async fn active_window(&self) -> Result<ActiveWindowInfo>;
    async fn window_tree(&self) -> Result<Vec<WindowInfo>>;
    async fn running_processes(&self) -> Result<Vec<ProcessInfo>>;
    async fn clipboard(&self) -> Result<ClipboardData>;
    async fn mouse_position(&self) -> Result<MouseInfo>;
    async fn monitors(&self) -> Result<Vec<MonitorInfo>>;
    async fn system_resources(&self) -> Result<SystemResources>;
    async fn network_state(&self) -> Result<NetworkState>;
    async fn audio_devices(&self) -> Result<AudioDevicesInfo>;
    async fn power_state(&self) -> Result<PowerState>;
    async fn workspace(&self) -> Result<WorkspaceInfo>;
    async fn notifications(&self) -> Result<Vec<NotificationInfo>>;
}
