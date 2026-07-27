//! macOS platform backend using osascript and system_profiler.
//!
//! Uses AppleScript/JXA for Accessibility API access and standard macOS tools.

use anyhow::Result;
use async_trait::async_trait;
use dcp_types::*;
use tokio::process::Command;

use super::PlatformBackend;

pub struct MacOsBackend;

impl MacOsBackend {
    pub async fn new() -> Result<Self> {
        tracing::info!("macOS backend initialized");
        Ok(Self)
    }

    /// Run an osascript command and return stdout.
    async fn osascript(script: &str) -> Result<String> {
        let output = Command::new("osascript")
            .args(["-e", script])
            .output()
            .await?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Run a JXA (JavaScript for Automation) script.
    async fn jxa(script: &str) -> Result<String> {
        let output = Command::new("osascript")
            .args(["-l", "JavaScript", "-e", script])
            .output()
            .await?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

#[async_trait]
impl PlatformBackend for MacOsBackend {
    async fn active_window(&self) -> Result<ActiveWindowInfo> {
        let jxa_script = r#"
var app = Application.frontmostApplication();
var appName = app.name();
var win = app.windows[0];
var title = win ? win.name() : "";
JSON.stringify({app: appName, title: title, pid: app.unixId()});
"#;
        let result = Self::jxa(jxa_script).await.unwrap_or_default();
        let mut title = String::new();
        let mut app = String::new();
        let mut pid: u32 = 0;
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&result) {
            title = val
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            app = val
                .get("app")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            pid = val.get("pid").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        }
        let semantic = if !title.is_empty() {
            Some(format!("Working in: {title}"))
        } else {
            None
        };
        Ok(ActiveWindowInfo {
            id: pid as u64,
            title,
            application: app,
            pid,
            bounds: Rect::new(0, 0, 0, 0),
            is_focused: true,
            semantic_context: semantic,
        })
    }

    async fn window_tree(&self) -> Result<Vec<WindowInfo>> {
        Ok(vec![])
    }

    async fn running_processes(&self) -> Result<Vec<ProcessInfo>> {
        // ps aux on macOS: USER PID %CPU %MEM VSZ RSS TT STAT STARTED TIME COMMAND
        // RSS is column 5 (0-indexed), in KB. Skip header line.
        let output = Command::new("ps").args(["aux"]).output().await?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut processes = Vec::new();
        for (i, line) in stdout.lines().enumerate() {
            if i == 0 {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 11 {
                let pid: u32 = parts.get(1).and_then(|v| v.parse().ok()).unwrap_or(0);
                let cpu: f64 = parts.get(2).and_then(|v| v.parse().ok()).unwrap_or(0.0);
                let rss_kb: u64 = parts.get(5).and_then(|v| v.parse().ok()).unwrap_or(0);
                let name = parts.get(10).unwrap_or(&"").to_string();
                let user = parts.get(0).map(|s| s.to_string());
                processes.push(ProcessInfo {
                    pid,
                    parent_pid: None,
                    name,
                    executable_path: None,
                    command_line: None,
                    cpu_percent: cpu,
                    memory_mb: rss_kb / 1024,
                    status: ProcessStatus::Running,
                    start_time: 0,
                    user,
                });
            }
        }
        Ok(processes)
    }

    async fn clipboard(&self) -> Result<ClipboardData> {
        let output = Command::new("pbpaste").output().await?;
        let content = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(ClipboardData {
            content_type: ContentType::Text,
            content,
            timestamp: chrono::Utc::now().timestamp_millis(),
        })
    }

    async fn mouse_position(&self) -> Result<MouseInfo> {
        let script = r#"use framework "CoreGraphics"
set pt to current location of mouse
return (item 1 of pt) & "|" & (item 2 of pt)"#;
        let result = Self::osascript(script).await.unwrap_or_default();
        let parts: Vec<&str> = result.splitn(2, '|').collect();
        let x = parts
            .get(0)
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(0);
        let y = parts
            .get(1)
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(0);
        Ok(MouseInfo {
            x,
            y,
            display_id: None,
            semantic_context: None,
        })
    }

    async fn monitors(&self) -> Result<Vec<MonitorInfo>> {
        let output = Command::new("system_profiler")
            .args(["SPDisplaysDataType", "-json"])
            .output()
            .await?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut monitors = Vec::new();
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&stdout) {
            if let Some(displays) = val.get("SPDisplaysDataType").and_then(|v| v.as_array()) {
                for (i, display) in displays.iter().enumerate() {
                    let name = display
                        .get("sppci_model")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Display")
                        .to_string();
                    let res = display
                        .get("spdisplays_resolution")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let (w, h) = parse_resolution(&res);
                    monitors.push(MonitorInfo {
                        id: i as u64,
                        name,
                        bounds: Rect::new(0, 0, w, h),
                        work_area: Rect::new(0, 0, w, h),
                        scale_factor: 2.0,
                        refresh_rate_hz: None,
                        is_primary: i == 0,
                        rotation: None,
                    });
                }
            }
        }
        Ok(monitors)
    }

    async fn system_resources(&self) -> Result<SystemResources> {
        let cpu = Command::new("ps")
            .args(["-A", "-o", "%cpu="])
            .output()
            .await?;
        let cpu_out = String::from_utf8_lossy(&cpu.stdout);
        let cpu_avg: f64 = cpu_out
            .lines()
            .filter_map(|l| l.trim().parse::<f64>().ok())
            .sum::<f64>();

        let mem = Command::new("vm_stat").output().await?;
        let mem_out = String::from_utf8_lossy(&mem.stdout);
        let page_size: u64 = 16384;
        let mut active_pages: u64 = 0;
        let mut wired_pages: u64 = 0;
        let mut free_pages: u64 = 0;
        for line in mem_out.lines() {
            if line.starts_with("Pages active:") {
                active_pages = line
                    .split(':')
                    .nth(1)
                    .and_then(|v| v.trim().trim_end_matches('.').parse().ok())
                    .unwrap_or(0);
            } else if line.starts_with("Pages wired down:") {
                wired_pages = line
                    .split(':')
                    .nth(1)
                    .and_then(|v| v.trim().trim_end_matches('.').parse().ok())
                    .unwrap_or(0);
            } else if line.starts_with("Pages free:") {
                free_pages = line
                    .split(':')
                    .nth(1)
                    .and_then(|v| v.trim().trim_end_matches('.').parse().ok())
                    .unwrap_or(0);
            }
        }
        let used_mb = ((active_pages + wired_pages) * page_size) / (1024 * 1024);
        let free_mb = (free_pages * page_size) / (1024 * 1024);
        let total_mb = used_mb + free_mb;
        let pct = if total_mb > 0 {
            (used_mb as f64 / total_mb as f64) * 100.0
        } else {
            0.0
        };

        Ok(SystemResources {
            cpu_usage_percent: cpu_avg,
            memory_total_mb: total_mb,
            memory_used_mb: used_mb,
            memory_percent: pct,
            swap_total_mb: 0,
            swap_used_mb: 0,
            disk_read_mb: None,
            disk_write_mb: None,
            load_average: None,
        })
    }

    async fn network_state(&self) -> Result<NetworkState> {
        let output = Command::new("ifconfig").args(["-l"]).output().await?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let interfaces: Vec<NetworkInterface> = stdout
            .split_whitespace()
            .filter(|name| *name != "lo0")
            .map(|name| NetworkInterface {
                name: name.to_string(),
                ip_addresses: vec![],
                mac_address: None,
                is_up: true,
                bytes_sent: 0,
                bytes_received: 0,
            })
            .collect();
        Ok(NetworkState {
            is_connected: !interfaces.is_empty(),
            connectivity_type: ConnectivityType::Ethernet,
            interfaces,
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
        let output = Command::new("pmset").args(["-g", "batt"]).output().await?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut percent = None;
        let mut is_charging = false;
        for line in stdout.lines() {
            if line.contains('%') {
                if let Some(pct_str) = line.split('\t').nth(1) {
                    percent = pct_str.trim().trim_end_matches('%').parse::<f64>().ok();
                    is_charging = line.contains("AC Power") || line.contains("charging");
                }
            }
        }
        let source = if is_charging {
            PowerSource::Ac
        } else {
            PowerSource::Battery
        };
        Ok(PowerState {
            source,
            battery_percent: percent,
            is_charging,
            time_remaining_seconds: None,
        })
    }

    async fn workspace(&self) -> Result<WorkspaceInfo> {
        Ok(WorkspaceInfo {
            current: 1,
            total: 1,
            names: vec!["Desktop".to_string()],
        })
    }

    async fn notifications(&self) -> Result<Vec<NotificationInfo>> {
        Ok(vec![])
    }

    async fn keyboard_focus(&self) -> Result<FocusInfo> {
        let win = self.active_window().await?;
        Ok(FocusInfo {
            element_type: "window".to_string(),
            description: Some(win.title),
            window_id: Some(win.id),
        })
    }

    async fn installed_apps(&self) -> Result<Vec<InstalledApp>> {
        let output = Command::new("ls").args(["/Applications"]).output().await?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let apps: Vec<InstalledApp> = stdout
            .lines()
            .filter(|line| line.ends_with(".app"))
            .map(|line| InstalledApp {
                name: line.trim_end_matches(".app").to_string(),
                executable: None,
                version: None,
                category: None,
            })
            .collect();
        Ok(apps)
    }

    async fn selected_text(&self) -> Result<Option<String>> {
        let clip = self.clipboard().await?;
        Ok(Some(clip.content).filter(|s| !s.is_empty()))
    }
}

/// Parse a resolution string like "1920x1080" into (width, height)
fn parse_resolution(res: &str) -> (u32, u32) {
    let parts: Vec<&str> = res.split('x').collect();
    let w = parts
        .get(0)
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0);
    let h = parts
        .get(1)
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0);
    (w, h)
}
