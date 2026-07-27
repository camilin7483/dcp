//! Linux platform backend: Wayland + X11 + D-Bus integration.
//!
//! Uses xdotool/xprop for X11, /proc for process info,
//! xclip/wl-paste for clipboard, and inotify for file watching.

use anyhow::{Context, Result};
use async_trait::async_trait;
use dcp_types::*;
use serde::Deserialize;
use tokio::task;
use tracing::warn;

use super::PlatformBackend;

#[derive(Deserialize, Debug)]
struct HyprWindow {
    #[serde(default)]
    address: String,
    #[serde(default)]
    class: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    pid: u32,
    #[serde(default)]
    at: [i32; 2],
    #[serde(default)]
    size: [u32; 2],
    #[serde(default)]
    workspace: HyprWorkspaceRef,
    #[serde(default)]
    monitor: u64,
    #[serde(default)]
    floating: bool,
    #[serde(default)]
    mapped: bool,
    #[serde(default)]
    hidden: bool,
    #[serde(default)]
    visible: bool,
    #[serde(default)]
    xwayland: bool,
}

#[derive(Deserialize, Debug, Default)]
struct HyprWorkspaceRef {
    #[serde(default)]
    id: i64,
    #[serde(default)]
    name: String,
}

#[derive(Deserialize, Debug)]
struct HyprMonitor {
    #[serde(default)]
    id: u64,
    #[serde(default)]
    name: String,
    #[serde(default)]
    width: u32,
    #[serde(default)]
    height: u32,
    #[serde(default)]
    refreshRate: f64,
    #[serde(default)]
    x: i32,
    #[serde(default)]
    y: i32,
    #[serde(default)]
    scale: f64,
    #[serde(default)]
    transform: u32,
    #[serde(default)]
    focused: bool,
}

#[derive(Deserialize, Debug)]
struct HyprWorkspace {
    #[serde(default)]
    id: i64,
    #[serde(default)]
    name: String,
    #[serde(default)]
    monitor: String,
    #[serde(default)]
    windows: u32,
}

#[derive(Deserialize, Debug)]
struct HyprCursorPos {
    x: i32,
    y: i32,
}

pub struct LinuxBackend {
    session_type: SessionType,
}

impl LinuxBackend {
    pub async fn new() -> Result<Self> {
        let session_type = if std::env::var("WAYLAND_DISPLAY").is_ok() {
            SessionType::Wayland
        } else if std::env::var("DISPLAY").is_ok() {
            SessionType::X11
        } else {
            SessionType::Unknown
        };

        Ok(Self { session_type })
    }

    async fn read_proc_file<T, F>(path: &str, parser: F) -> T
    where
        F: FnOnce(&str) -> T + Send + 'static,
        T: Send + Default + 'static,
    {
        let path = path.to_string();
        task::spawn_blocking(move || {
            match std::fs::read_to_string(&path) {
                Ok(content) => parser(&content),
                Err(_) => T::default(),
            }
        })
        .await
        .unwrap_or_default()
    }

    async fn xdotool_output(args: &[&str]) -> Result<String> {
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let output = tokio::process::Command::new("xdotool")
            .args(&args)
            .output().await?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    async fn xprop_output(args: &[&str]) -> Result<String> {
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let output = tokio::process::Command::new("xprop")
            .args(&args)
            .output().await?;
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn get_window_info_x11(window_id: &str) -> Result<WindowInfo> {
        let wid = window_id.trim();

        let title = Self::xdotool_output(&["getwindowname", wid])
            .await
            .unwrap_or_default();

        let pid_str = Self::xdotool_output(&["getwindowpid", wid])
            .await
            .unwrap_or_else(|_| "0".to_string());
        let pid: u32 = pid_str.parse().unwrap_or(0);

        let app = Self::read_proc_file(&format!("/proc/{pid}/comm"), |c| {
            c.trim().to_string()
        }).await;

        let geo = Self::xdotool_output(&["getwindowgeometry", "--shell", wid])
            .await
            .unwrap_or_default();

        let mut x = 0i32;
        let mut y = 0i32;
        let mut width = 0u32;
        let mut height = 0u32;

        for line in geo.lines() {
            if let Some(val) = line.strip_prefix("X=") {
                x = val.parse().unwrap_or(0);
            } else if let Some(val) = line.strip_prefix("Y=") {
                y = val.parse().unwrap_or(0);
            } else if let Some(val) = line.strip_prefix("WIDTH=") {
                width = val.parse().unwrap_or(0);
            } else if let Some(val) = line.strip_prefix("HEIGHT=") {
                height = val.parse().unwrap_or(0);
            }
        }

        let focused_id = Self::xdotool_output(&["getactivewindow"]).await.unwrap_or_default();
        let is_focused = focused_id == wid;

        Ok(WindowInfo {
            id: wid.parse().unwrap_or(0),
            title,
            application: app,
            pid,
            bounds: Rect::new(x, y, width, height),
            is_focused,
            is_minimized: false,
            is_maximized: false,
            is_visible: true,
            monitor_id: None,
            workspace_id: None,
            parent_id: None,
            children: vec![],
        })
    }

    async fn read_meminfo() -> (u64, u64) {
        Self::read_proc_file("/proc/meminfo", |content| {
            let mut total_kb = 0u64;
            let mut available_kb = 0u64;

            for line in content.lines() {
                if line.starts_with("MemTotal:") {
                    total_kb = line.split_whitespace()
                        .nth(1)
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0);
                } else if line.starts_with("MemAvailable:") {
                    available_kb = line.split_whitespace()
                        .nth(1)
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0);
                }
            }

            (total_kb / 1024, available_kb / 1024)
        }).await
    }

    async fn read_load_average() -> Option<[f64; 3]> {
        Self::read_proc_file("/proc/loadavg", |content| {
            let parts: Vec<f64> = content
                .split_whitespace()
                .take(3)
                .filter_map(|p| p.parse().ok())
                .collect();
            if parts.len() == 3 {
                Some([parts[0], parts[1], parts[2]])
            } else {
                None
            }
        }).await
    }

    fn is_hyprland(&self) -> bool {
        self.session_type == SessionType::Wayland
            && std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok()
    }

    async fn hyprctl(args: &[&str]) -> Result<String> {
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let output = tokio::process::Command::new("hyprctl")
            .args(&args)
            .output().await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("hyprctl failed: {stderr}");
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn hyprctl_json<T: serde::de::DeserializeOwned>(args: &[&str]) -> Result<T> {
        let json = Self::hyprctl(args).await?;
        Ok(serde_json::from_str(&json)?)
    }

    fn semantic_context_for(title: &str, class: &str) -> Option<String> {
        if title.is_empty() {
            return None;
        }
        if title.ends_with(".rs") || title.ends_with(".py") || title.ends_with(".ts") || title.ends_with(".js") {
            return Some(format!("Editing source file: {title}"));
        }
        let lower_class = class.to_lowercase();
        if lower_class.contains("terminal") || lower_class.contains("kitty") || lower_class.contains("alacritty") || lower_class.contains("wezterm") {
            return Some(format!("Using terminal: {title}"));
        }
        if lower_class.contains("firefox") || lower_class.contains("chrome") || lower_class.contains("chromium") || lower_class.contains("brave") {
            return Some(format!("Browsing: {title}"));
        }
        if lower_class.contains("spotify") {
            return Some(format!("Playing music: {title}"));
        }
        Some(format!("Working in: {title}"))
    }

    async fn read_cpu_usage() -> f64 {
        Self::read_proc_file("/proc/stat", |content| {
            if let Some(first_line) = content.lines().next() {
                let parts: Vec<u64> = first_line
                    .split_whitespace()
                    .skip(1)
                    .filter_map(|p| p.parse().ok())
                    .collect();
                if parts.len() >= 4 {
                    let total: u64 = parts.iter().copied().map(|v| v).sum();
                    let idle = parts.get(3).copied().unwrap_or(0);
                    if total > 0 {
                        return ((total.saturating_sub(idle)) as f64 / total as f64) * 100.0;
                    }
                }
            }
            0.0
        }).await
    }
}

#[async_trait]
impl PlatformBackend for LinuxBackend {
    async fn active_window(&self) -> Result<ActiveWindowInfo> {
        match self.session_type {
            SessionType::X11 => {
                let wid = Self::xdotool_output(&["getactivewindow"])
                    .await
                    .unwrap_or_default();
                if wid.is_empty() {
                    return Ok(ActiveWindowInfo {
                        id: 0,
                        title: String::new(),
                        application: String::new(),
                        pid: 0,
                        bounds: Rect::new(0, 0, 0, 0),
                        is_focused: true,
                        semantic_context: None,
                    });
                }

                let title = Self::xdotool_output(&["getwindowname", &wid])
                    .await
                    .unwrap_or_default();

                let pid_str = Self::xdotool_output(&["getwindowpid", &wid])
                    .await
                    .unwrap_or_else(|_| "0".to_string());
                let pid: u32 = pid_str.parse().unwrap_or(0);

                let app = task::spawn_blocking(move || {
                    std::fs::read_to_string(format!("/proc/{pid}/comm"))
                        .unwrap_or_default()
                        .trim()
                        .to_string()
                }).await.unwrap_or_default();

                let geo = Self::xdotool_output(&["getwindowgeometry", "--shell", &wid])
                    .await
                    .unwrap_or_default();

                let mut x = 0i32;
                let mut y = 0i32;
                let mut width = 0u32;
                let mut height = 0u32;

                for line in geo.lines() {
                    if let Some(val) = line.strip_prefix("X=") {
                        x = val.parse().unwrap_or(0);
                    } else if let Some(val) = line.strip_prefix("Y=") {
                        y = val.parse().unwrap_or(0);
                    } else if let Some(val) = line.strip_prefix("WIDTH=") {
                        width = val.parse().unwrap_or(0);
                    } else if let Some(val) = line.strip_prefix("HEIGHT=") {
                        height = val.parse().unwrap_or(0);
                    }
                }

                // Semantic context heuristic
                let semantic = if title.ends_with(".rs") || title.ends_with(".py") || title.ends_with(".ts") {
                    Some(format!("Editing source file: {title}"))
                } else if title.contains("Terminal") || title.contains("alacritty") || title.contains("kitty") {
                    Some(format!("Using terminal: {title}"))
                } else if title.contains("Firefox") || title.contains("Chrome") || title.contains("Browser") {
                    Some(format!("Browsing: {title}"))
                } else if !title.is_empty() {
                    Some(format!("Working in: {title}"))
                } else {
                    None
                };

                Ok(ActiveWindowInfo {
                    id: wid.parse().unwrap_or(0),
                    title,
                    application: app,
                    pid,
                    bounds: Rect::new(x, y, width, height),
                    is_focused: true,
                    semantic_context: semantic,
                })
            }
            SessionType::Wayland => {
                if self.is_hyprland() {
                    match Self::hyprctl_json::<HyprWindow>(&["activewindow", "-j"]).await {
                        Ok(w) => {
                            let semantic = Self::semantic_context_for(&w.title, &w.class);
                            let wid = u64::from_str_radix(
                                w.address.trim_start_matches("0x"),
                                16,
                            ).unwrap_or(0);
                            return Ok(ActiveWindowInfo {
                                id: wid,
                                title: w.title,
                                application: w.class,
                                pid: w.pid,
                                bounds: Rect::new(w.at[0], w.at[1], w.size[0], w.size[1]),
                                is_focused: true,
                                semantic_context: semantic,
                            });
                        }
                        Err(e) => {
                            warn!("hyprctl activewindow failed: {e}");
                        }
                    }
                }
                Ok(ActiveWindowInfo {
                    id: 0,
                    title: "Wayland (unsupported compositor)".to_string(),
                    application: "unknown".to_string(),
                    pid: 0,
                    bounds: Rect::new(0, 0, 0, 0),
                    is_focused: true,
                    semantic_context: None,
                })
            }
            _ => Ok(ActiveWindowInfo {
                id: 0,
                title: "Unknown session".to_string(),
                application: "unknown".to_string(),
                pid: 0,
                bounds: Rect::new(0, 0, 0, 0),
                is_focused: true,
                semantic_context: None,
            }),
        }
    }

    async fn window_tree(&self) -> Result<Vec<WindowInfo>> {
        match self.session_type {
            SessionType::X11 => {
                let output = Self::xdotool_output(&["search", "--onlyvisible", "--name", ""])
                    .await
                    .unwrap_or_default();

                let mut windows = Vec::new();
                for wid in output.lines() {
                    let wid = wid.trim();
                    if wid.is_empty() {
                        continue;
                    }
                    if let Ok(info) = Self::get_window_info_x11(wid).await {
                        windows.push(info);
                    }
                }
                Ok(windows)
            }
            SessionType::Wayland if self.is_hyprland() => {
                let clients: Vec<HyprWindow> = Self::hyprctl_json(&["clients", "-j"])
                    .await
                    .unwrap_or_default();

                let active: Option<HyprWindow> = Self::hyprctl_json(&["activewindow", "-j"]).await.ok();
                let active_addr = active.as_ref().map(|w| w.address.as_str()).unwrap_or("");

                Ok(clients
                    .into_iter()
                    .map(|w| {
                        let wid = u64::from_str_radix(
                            w.address.trim_start_matches("0x"),
                            16,
                        ).unwrap_or(0);
                        WindowInfo {
                            id: wid,
                            title: w.title,
                            application: w.class,
                            pid: w.pid,
                            bounds: Rect::new(w.at[0], w.at[1], w.size[0], w.size[1]),
                            is_focused: w.address == active_addr,
                            is_minimized: w.hidden,
                            is_maximized: false,
                            is_visible: w.visible,
                            monitor_id: Some(w.monitor),
                            workspace_id: if w.workspace.id > 0 {
                                Some(w.workspace.id as u32)
                            } else {
                                None
                            },
                            parent_id: None,
                            children: vec![],
                        }
                    })
                    .collect())
            }
            _ => Ok(vec![]),
        }
    }

    async fn running_processes(&self) -> Result<Vec<ProcessInfo>> {
        task::spawn_blocking(|| -> Result<Vec<ProcessInfo>> {
            let mut processes = Vec::new();

            let dir = match std::fs::read_dir("/proc") {
                Ok(d) => d,
                Err(e) => return Err(anyhow::anyhow!("Failed to read /proc: {e}")),
            };

            for entry in dir {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                let name = entry.file_name();
                let name_str = name.to_string_lossy();

                if let Ok(pid) = name_str.parse::<u32>() {
                    let comm = match std::fs::read_to_string(format!("/proc/{pid}/comm")) {
                        Ok(c) => c.trim().to_string(),
                        Err(_) => continue,
                    };

                    let exe = std::fs::read_link(format!("/proc/{pid}/exe"))
                        .ok()
                        .map(|p| p.to_string_lossy().to_string());

                    let cmdline = std::fs::read_to_string(format!("/proc/{pid}/cmdline"))
                        .ok()
                        .map(|s| s.replace('\0', " ").trim().to_string());

                    let stat_content = std::fs::read_to_string(format!("/proc/{pid}/stat"))
                        .unwrap_or_default();
                    let stat_parts: Vec<&str> = stat_content.rsplitn(2, ')').collect();
                    let mut memory_mb = 0u64;
                    let mut status = ProcessStatus::Unknown;

                    if stat_parts.len() >= 2 {
                        let fields: Vec<&str> = stat_parts[0].split_whitespace().collect();
                        if fields.len() >= 24 {
                            let rss_pages: u64 = fields.get(21)
                                .and_then(|v| v.parse().ok())
                                .unwrap_or(0);
                            memory_mb = (rss_pages * 4096) / (1024 * 1024);

                            let state = fields.get(0).copied().unwrap_or("");
                            status = match state {
                                "R" => ProcessStatus::Running,
                                "S" => ProcessStatus::Sleeping,
                                "D" => ProcessStatus::Sleeping,
                                "T" => ProcessStatus::Stopped,
                                "Z" => ProcessStatus::Zombie,
                                "I" => ProcessStatus::Idle,
                                _ => ProcessStatus::Unknown,
                            };
                        }
                    }

                    let ppid = std::fs::read_to_string(format!("/proc/{pid}/status"))
                        .ok()
                        .and_then(|s| {
                            s.lines()
                                .find(|l| l.starts_with("PPid:"))
                                .and_then(|l| l.split_whitespace().nth(1))
                                .and_then(|v| v.parse::<u32>().ok())
                        });

                    processes.push(ProcessInfo {
                        pid,
                        parent_pid: ppid,
                        name: comm,
                        executable_path: exe,
                        command_line: cmdline,
                        cpu_percent: 0.0,
                        memory_mb,
                        status,
                        start_time: 0,
                        user: None,
                    });
                }
            }

            Ok(processes)
        }).await
          .context("blocking task failed")?
    }

    async fn clipboard(&self) -> Result<ClipboardData> {
        let content = match self.session_type {
            SessionType::X11 => {
                let output = tokio::process::Command::new("xclip")
                    .args(["-selection", "clipboard", "-o"])
                    .output().await?;
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            SessionType::Wayland => {
                let output = tokio::process::Command::new("wl-paste")
                    .output().await?;
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            _ => String::new(),
        };

        let content_type = if content.trim().starts_with("<!DOCTYPE html")
            || content.trim().starts_with("<html")
            || content.trim().starts_with("<!--")
        {
            ContentType::Html
        } else {
            ContentType::Text
        };

        Ok(ClipboardData {
            content_type,
            content,
            timestamp: chrono::Utc::now().timestamp_millis(),
        })
    }

    async fn mouse_position(&self) -> Result<MouseInfo> {
        if self.is_hyprland() {
            if let Ok(pos) = Self::hyprctl_json::<HyprCursorPos>(&["cursorpos", "-j"]).await {
                let semantic = Some(format!("Cursor at ({}, {})", pos.x, pos.y));
                return Ok(MouseInfo {
                    x: pos.x,
                    y: pos.y,
                    display_id: None,
                    semantic_context: semantic,
                });
            }
        }

        let output = Self::xdotool_output(&["getmouselocation"])
            .await
            .unwrap_or_default();

        let mut x = 0i32;
        let mut y = 0i32;
        let mut screen = 0u64;

        for part in output.split_whitespace() {
            if let Some(val) = part.strip_prefix("x:") {
                x = val.parse().unwrap_or(0);
            } else if let Some(val) = part.strip_prefix("y:") {
                y = val.parse().unwrap_or(0);
            } else if let Some(val) = part.strip_prefix("screen:") {
                screen = val.parse().unwrap_or(0);
            }
        }

        let semantic = if x > 0 || y > 0 {
            Some(format!("Cursor at ({x}, {y}) on screen {screen}"))
        } else {
            None
        };

        Ok(MouseInfo {
            x,
            y,
            display_id: Some(screen),
            semantic_context: semantic,
        })
    }

    async fn monitors(&self) -> Result<Vec<MonitorInfo>> {
        if self.is_hyprland() {
            let mons: Vec<HyprMonitor> = Self::hyprctl_json(&["monitors", "-j"])
                .await
                .unwrap_or_default();
            return Ok(mons
                .into_iter()
                .map(|m| {
                    let bounds = Rect::new(m.x, m.y, m.width, m.height);
                    MonitorInfo {
                        id: m.id,
                        name: m.name,
                        bounds,
                        work_area: bounds,
                        scale_factor: m.scale,
                        refresh_rate_hz: Some(m.refreshRate as u32),
                        is_primary: m.focused,
                        rotation: Some(m.transform),
                    }
                })
                .collect());
        }

        if self.session_type == SessionType::X11 {
            let output = tokio::process::Command::new("xrandr")
                .args(["--query"])
                .output().await?;
            let stdout = String::from_utf8_lossy(&output.stdout);

            let mut monitors = Vec::new();
            let mut current_name = String::new();
            let mut is_primary = false;
            let mut bounds = Rect::new(0, 0, 0, 0);
            let mut refresh = None;

            for line in stdout.lines() {
                if line.contains(" connected") {
                    current_name = line.split_whitespace().next().unwrap_or("").to_string();
                    is_primary = line.contains("primary");

                    for part in line.split_whitespace() {
                        if part.contains('x') && part.contains('+') {
                            let geom: Vec<&str> = part.split('+').collect();
                            if geom.len() >= 3 {
                                let size: Vec<&str> = geom[0].split('x').collect();
                                if size.len() == 2 {
                                    let w: u32 = size[0].parse().unwrap_or(0);
                                    let h: u32 = size[1].parse().unwrap_or(0);
                                    let x: i32 = geom[1].parse().unwrap_or(0);
                                    let y: i32 = geom[2].parse().unwrap_or(0);
                                    bounds = Rect::new(x, y, w, h);
                                }
                            }
                        }
                        if part.contains("Hz") || part.ends_with('*') {
                            refresh = part.trim_end_matches('*')
                                .parse::<f64>().ok()
                                .map(|f| f as u32);
                        }
                    }

                    if !current_name.is_empty() && bounds.width > 0 {
                        monitors.push(MonitorInfo {
                            id: monitors.len() as u64,
                            name: current_name.clone(),
                            bounds,
                            work_area: bounds,
                            scale_factor: 1.0,
                            refresh_rate_hz: refresh,
                            is_primary,
                            rotation: None,
                        });
                    }
                }
            }

            return Ok(monitors);
        }

        Ok(vec![])
    }

    async fn system_resources(&self) -> Result<SystemResources> {
        let (total_mb, available_mb) = Self::read_meminfo().await;
        let used_mb = total_mb.saturating_sub(available_mb);
        let percent = if total_mb > 0 {
            (used_mb as f64 / total_mb as f64) * 100.0
        } else {
            0.0
        };

        let load = Self::read_load_average().await;
        let cpu = Self::read_cpu_usage().await;

        Ok(SystemResources {
            cpu_usage_percent: cpu,
            memory_total_mb: total_mb,
            memory_used_mb: used_mb,
            memory_percent: percent,
            swap_total_mb: 0,
            swap_used_mb: 0,
            disk_read_mb: None,
            disk_write_mb: None,
            load_average: load,
        })
    }

    async fn network_state(&self) -> Result<NetworkState> {
        task::spawn_blocking(|| -> Result<NetworkState> {
            let mut interfaces = Vec::new();

            if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name == "lo" {
                        continue;
                    }

                    let is_up = std::fs::read_to_string(format!("/sys/class/net/{name}/operstate"))
                        .map(|s| s.trim() == "up")
                        .unwrap_or(false);

                    let mac = std::fs::read_to_string(format!("/sys/class/net/{name}/address"))
                        .ok()
                        .map(|s| s.trim().to_string());

                    let stats_rx = std::fs::read_to_string(format!("/sys/class/net/{name}/statistics/rx_bytes"))
                        .ok()
                        .and_then(|s| s.trim().parse::<u64>().ok())
                        .unwrap_or(0);

                    let stats_tx = std::fs::read_to_string(format!("/sys/class/net/{name}/statistics/tx_bytes"))
                        .ok()
                        .and_then(|s| s.trim().parse::<u64>().ok())
                        .unwrap_or(0);

                    interfaces.push(NetworkInterface {
                        name,
                        ip_addresses: vec![],
                        mac_address: mac,
                        is_up,
                        bytes_sent: stats_tx,
                        bytes_received: stats_rx,
                    });
                }
            }

            let is_connected = interfaces.iter().any(|i| i.is_up);
            let connectivity = if is_connected {
                ConnectivityType::Ethernet
            } else {
                ConnectivityType::None
            };

            Ok(NetworkState {
                interfaces,
                is_connected,
                connectivity_type: connectivity,
            })
        }).await
          .context("blocking task failed")?
    }

    async fn audio_devices(&self) -> Result<AudioDevicesInfo> {
        let output = tokio::process::Command::new("pactl")
            .args(["list", "sinks", "short"])
            .output().await;

        let mut outputs = Vec::new();
        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 2 {
                    outputs.push(AudioDevice {
                        id: parts[0].to_string(),
                        name: parts.get(1).unwrap_or(&"").to_string(),
                        is_default: false,
                        volume: None,
                        is_muted: false,
                    });
                }
            }
        }

        Ok(AudioDevicesInfo {
            inputs: vec![],
            outputs,
            default_input: None,
            default_output: None,
        })
    }

    async fn power_state(&self) -> Result<PowerState> {
        task::spawn_blocking(|| -> Result<PowerState> {
            let mut source = PowerSource::Ac;
            let mut percent = None;
            let mut is_charging = false;

            if let Ok(entries) = std::fs::read_dir("/sys/class/power_supply") {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let base = format!("/sys/class/power_supply/{name}");

                    let ptype = std::fs::read_to_string(format!("{base}/type"))
                        .unwrap_or_default()
                        .trim()
                        .to_string();

                    if ptype == "Battery" {
                        let cap = std::fs::read_to_string(format!("{base}/capacity"))
                            .ok()
                            .and_then(|s| s.trim().parse::<f64>().ok());
                        percent = cap;

                        let status = std::fs::read_to_string(format!("{base}/status"))
                            .unwrap_or_default()
                            .trim()
                            .to_string();
                        is_charging = status == "Charging";
                        source = PowerSource::Battery;
                    }
                }
            }

            Ok(PowerState {
                source,
                battery_percent: percent,
                is_charging,
                time_remaining_seconds: None,
            })
        }).await
          .context("blocking task failed")?
    }

    async fn workspace(&self) -> Result<WorkspaceInfo> {
        if self.is_hyprland() {
            let workspaces: Vec<HyprWorkspace> = Self::hyprctl_json(&["workspaces", "-j"])
                .await
                .unwrap_or_default();
            let active: HyprWorkspace = Self::hyprctl_json(&["activeworkspace", "-j"])
                .await
                .unwrap_or(HyprWorkspace {
                    id: 1,
                    name: "1".to_string(),
                    monitor: String::new(),
                    windows: 0,
                });

            let regular: Vec<&HyprWorkspace> = workspaces
                .iter()
                .filter(|w| w.id > 0)
                .collect();
            let total = regular.len() as u32;
            let names: Vec<String> = regular.iter().map(|w| w.name.clone()).collect();

            return Ok(WorkspaceInfo {
                current: active.id as u32,
                total,
                names,
            });
        }

        let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();

        if desktop.to_lowercase().contains("sway") {
            return Ok(WorkspaceInfo {
                current: 1,
                total: 1,
                names: vec!["sway".to_string()],
            });
        }

        if self.session_type == SessionType::X11 {
            if let Ok(output) = tokio::process::Command::new("xprop")
                .args(["-root", "_NET_CURRENT_DESKTOP"])
                .output().await
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let current = stdout.split('=')
                    .nth(1)
                    .and_then(|s| s.trim().parse::<u32>().ok())
                    .unwrap_or(0);

                if let Ok(output) = tokio::process::Command::new("xprop")
                    .args(["-root", "_NET_NUMBER_OF_DESKTOPS"])
                    .output().await
                {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let total = stdout.split('=')
                        .nth(1)
                        .and_then(|s| s.trim().parse::<u32>().ok())
                        .unwrap_or(1);

                    let names = (0..total).map(|i| format!("Desktop {i}")).collect();
                    return Ok(WorkspaceInfo { current, total, names });
                }
            }
        }

        Ok(WorkspaceInfo {
            current: 0,
            total: 1,
            names: vec!["default".to_string()],
        })
    }

    async fn notifications(&self) -> Result<Vec<NotificationInfo>> {
        Ok(vec![])
    }

    async fn keyboard_focus(&self) -> Result<FocusInfo> {
        let output = tokio::process::Command::new("xdotool")
            .args(["getactivewindow", "getwindowname"])
            .output().await?;
        let title = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(FocusInfo {
            element_type: "window".to_string(),
            description: if title.is_empty() { None } else { Some(title) },
            window_id: None,
        })
    }

    async fn installed_apps(&self) -> Result<Vec<InstalledApp>> {
        let apps = tokio::task::spawn_blocking(|| {
            let mut apps = Vec::new();
            if let Ok(entries) = std::fs::read_dir("/usr/share/applications") {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                        continue;
                    }
                    let content = std::fs::read_to_string(&path).unwrap_or_default();
                    let name = content.lines()
                        .find(|l| l.starts_with("Name="))
                        .map(|l| l.trim_start_matches("Name=").to_string())
                        .unwrap_or_default();
                    let exec = content.lines()
                        .find(|l| l.starts_with("Exec="))
                        .map(|l| l.trim_start_matches("Exec=").to_string());
                    let cat = content.lines()
                        .find(|l| l.starts_with("Categories="))
                        .map(|l| l.trim_start_matches("Categories=").to_string());
                    if !name.is_empty() {
                        apps.push(InstalledApp {
                            name,
                            executable: exec,
                            version: None,
                            category: cat,
                        });
                    }
                }
            }
            apps
        }).await.unwrap_or_default();
        Ok(apps)
    }

    async fn selected_text(&self) -> Result<Option<String>> {
        if self.session_type == SessionType::X11 {
            if let Ok(output) = tokio::process::Command::new("xclip")
                .args(["-selection", "primary", "-o"])
                .output().await
            {
                let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !text.is_empty() {
                    return Ok(Some(text));
                }
            }
        }
        Ok(None)
    }
}
