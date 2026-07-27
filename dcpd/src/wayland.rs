//! Wayland native protocol support.
//!
//! Provides native Wayland integration using:
//! - wlr-foreign-toplevel-management (window management)
//! - zwlr-screencopy (screen capture)
//! - ext-foreign-toplevel-list (window enumeration)
//!
//! Note: Full implementation requires compositor support for these protocols.
//! Many compositors (Sway, Hyprland) support these, but GNOME/KDE may not.

use anyhow::Result;
use dcp_types::*;
use tracing::{info, warn};

/// Check if we're running on Wayland.
pub fn is_wayland() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
}

/// Get the Wayland display name.
pub fn wayland_display() -> Option<String> {
    std::env::var("WAYLAND_DISPLAY").ok()
}

/// Wayland backend for window management.
///
/// This is a stub implementation. Full support requires:
/// 1. Custom protocol bindings for wlr-foreign-toplevel-management
/// 2. Event loop integration with wayland-client
/// 3. Compositor-specific extensions (e.g., Hyprland IPC)
pub struct WaylandBackend {
    display_name: String,
}

impl WaylandBackend {
    pub fn new() -> Result<Self> {
        let display_name = wayland_display()
            .ok_or_else(|| anyhow::anyhow!("Not running on Wayland"))?;
        
        info!("Wayland backend initialized: {display_name}");
        Ok(Self { display_name })
    }

    /// Get active window (stub - would use wlr-foreign-toplevel).
    pub async fn active_window(&self) -> Result<ActiveWindowInfo> {
        // In a full implementation, this would:
        // 1. Connect to Wayland display
        // 2. Bind to wlr-foreign-toplevel-manager
        // 3. Listen for toplevel events
        // 4. Track focused window
        
        // For now, fall back to xdotool if XWayland is available
        warn!("Wayland native window tracking not fully implemented, falling back to XWayland");
        
        let output = std::process::Command::new("xdotool")
            .args(["getactivewindow", "getwindowname"])
            .output()?;
        
        let title = String::from_utf8_lossy(&output.stdout).trim().to_string();
        
        Ok(ActiveWindowInfo {
            id: 0,
            title,
            application: "unknown".to_string(),
            pid: 0,
            bounds: Rect::new(0, 0, 0, 0),
            is_focused: true,
            semantic_context: None,
        })
    }

    /// Get window list (stub - would use wlr-foreign-toplevel).
    pub async fn window_list(&self) -> Result<Vec<WindowInfo>> {
        warn!("Wayland native window enumeration not fully implemented");
        Ok(vec![])
    }

    /// Capture screen (stub - would use zwlr-screencopy).
    pub async fn capture_screen(&self) -> Result<Vec<u8>> {
        warn!("Wayland native screen capture not fully implemented, using grim fallback");
        
        let output = std::process::Command::new("grim")
            .args(["-t", "png", "-"])
            .output()?;
        
        Ok(output.stdout)
    }
}

/// Compositor-specific IPC for Hyprland.
pub mod hyprland {
    use anyhow::Result;
    use std::os::unix::net::UnixStream;
    use std::io::{Read, Write};

    /// Query Hyprland IPC socket.
    pub fn query_hyprland(request: &str) -> Result<String> {
        let socket_path = std::env::var("HYPRLAND_INSTANCE_SIGNATURE")
            .map(|sig| format!("/tmp/hypr/{sig}/.socket.sock"))
            .map_err(|_| anyhow::anyhow!("HYPRLAND_INSTANCE_SIGNATURE not set"))?;

        let mut stream = UnixStream::connect(&socket_path)?;
        stream.write_all(request.as_bytes())?;
        
        let mut response = String::new();
        stream.read_to_string(&mut response)?;
        
        Ok(response)
    }

    /// Get active window from Hyprland.
    pub fn active_window() -> Result<String> {
        let response = query_hyprland("activewindow")?;
        Ok(response)
    }

    /// Get all windows from Hyprland.
    pub fn windows() -> Result<String> {
        let response = query_hyprland("clients")?;
        Ok(response)
    }
}

/// Compositor-specific IPC for Sway.
pub mod sway {
    use anyhow::Result;
    use std::os::unix::net::UnixStream;
    use std::io::{Read, Write};

    /// Query Sway IPC socket.
    pub fn query_sway(request_type: u32, payload: &str) -> Result<String> {
        let socket_path = std::env::var("SWAYSOCK")
            .map_err(|_| anyhow::anyhow!("SWAYSOCK not set"))?;

        let mut stream = UnixStream::connect(&socket_path)?;
        
        // Sway IPC protocol: magic + length + type + payload
        let magic = b"i3-ipc";
        let length = payload.len() as u32;
        
        stream.write_all(magic)?;
        stream.write_all(&length.to_ne_bytes())?;
        stream.write_all(&request_type.to_ne_bytes())?;
        stream.write_all(payload.as_bytes())?;
        
        // Read response
        let mut header = [0u8; 14]; // magic(6) + length(4) + type(4)
        stream.read_exact(&mut header)?;
        
        let length = u32::from_ne_bytes([header[6], header[7], header[8], header[9]]) as usize;
        let mut response = vec![0u8; length];
        stream.read_exact(&mut response)?;
        
        Ok(String::from_utf8_lossy(&response).to_string())
    }

    /// Get active window from Sway.
    pub fn active_window() -> Result<String> {
        let response = query_sway(1, "")?; // 1 = GET_TREE
        Ok(response)
    }
}
