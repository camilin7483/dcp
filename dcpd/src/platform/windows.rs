//! Windows platform backend using the Windows API.
//!
//! Uses `windows-rs` crate for Win32 API access.

use anyhow::Result;
use async_trait::async_trait;
use dcp_types::*;
#[cfg(windows)]
use tokio::process::Command;
use tracing::warn;

use super::PlatformBackend;

pub struct WindowsBackend;

impl WindowsBackend {
    pub async fn new() -> Result<Self> {
        #[cfg(windows)]
        tracing::info!("Windows backend initialized");
        #[cfg(not(windows))]
        tracing::warn!("Windows backend running on non-Windows platform (stub mode)");
        Ok(Self)
    }
}

#[async_trait]
impl PlatformBackend for WindowsBackend {
    async fn active_window(&self) -> Result<ActiveWindowInfo> {
        #[cfg(windows)] {
            // Use PowerShell to get active window info
            let script = r#"
Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
using System.Diagnostics;
public class WinAPI {
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int count);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);
}
"@
$hwnd = [WinAPI]::GetForegroundWindow()
$sb = New-Object System.Text.StringBuilder 256
[WinAPI]::GetWindowText($hwnd, $sb, 256) | Out-Null
$title = $sb.ToString()
$pid = 0
[WinAPI]::GetWindowThreadProcessId($hwnd, [ref]$pid) | Out-Null
$proc = Get-Process -Id $pid -ErrorAction SilentlyContinue
Write-Output "$pid|$($proc.ProcessName)|$title"
"#;
            let output = Command::new("powershell")
                .args(["-NoProfile", "-Command", script])
                .output().await?;
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let parts: Vec<&str> = stdout.splitn(3, '|').collect();
            let pid: u32 = parts.get(0).and_then(|p| p.parse().ok()).unwrap_or(0);
            let app = parts.get(1).unwrap_or(&"").to_string();
            let title = parts.get(2).unwrap_or(&"").to_string();
            return Ok(ActiveWindowInfo {
                id: pid as u64,
                title,
                application: app,
                pid,
                bounds: Rect::new(0, 0, 0, 0),
                is_focused: true,
                semantic_context: None,
            });
        }
        #[cfg(not(windows))]
        Ok(ActiveWindowInfo {
            id: 0,
            title: String::new(),
            application: String::new(),
            pid: 0,
            bounds: Rect::new(0, 0, 0, 0),
            is_focused: false,
            semantic_context: None,
        })
    }

    async fn window_tree(&self) -> Result<Vec<WindowInfo>> {
        Ok(vec![])
    }

    async fn running_processes(&self) -> Result<Vec<ProcessInfo>> {
        #[cfg(windows)] {
            let output = Command::new("powershell")
                .args(["-NoProfile", "-Command", r#"
Get-Process | Select-Object Id, ProcessName, CPU, WorkingSet64, StartTime, @{N='ParentId';E={(Get-WmiObject Win32_Process -Filter ('ProcessId='+$_.Id)).ParentProcessId}} | ConvertTo-Json -Compress
                "#])
                .output().await?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut processes = Vec::new();
            if let Ok(list) = serde_json::from_str::<Vec<serde_json::Value>>(&stdout) {
                for item in list {
                    let pid: u32 = item.get("Id").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    let name = item.get("ProcessName").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let mem_mb = item.get("WorkingSet64").and_then(|v| v.as_u64()).unwrap_or(0) / (1024*1024);
                    let ppid: Option<u32> = item.get("ParentId").and_then(|v| v.as_u64()).map(|v| v as u32);
                    processes.push(ProcessInfo {
                        pid,
                        parent_pid: ppid,
                        name,
                        executable_path: None,
                        command_line: None,
                        cpu_percent: item.get("CPU").and_then(|v| v.as_f64()).unwrap_or(0.0),
                        memory_mb: mem_mb as u64,
                        status: ProcessStatus::Running,
                        start_time: 0,
                        user: None,
                    });
                }
            }
            return Ok(processes);
        }
        #[cfg(not(windows))]
        Ok(vec![])
    }

    async fn clipboard(&self) -> Result<ClipboardData> {
        #[cfg(windows)] {
            let output = Command::new("powershell")
                .args(["-NoProfile", "-Command", r#"Get-Clipboard"#])
                .output().await?;
            let content = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return Ok(ClipboardData {
                content_type: ContentType::Text,
                content,
                timestamp: chrono::Utc::now().timestamp_millis(),
            });
        }
        #[cfg(not(windows))]
        Ok(ClipboardData {
            content_type: ContentType::Text,
            content: String::new(),
            timestamp: chrono::Utc::now().timestamp_millis(),
        })
    }

    async fn mouse_position(&self) -> Result<MouseInfo> {
        #[cfg(windows)] {
            let script = r#"
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class MouseAPI {
    [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT lpPoint);
    public struct POINT { public int X; public int Y; }
}
"@
$pt = New-Object MouseAPI+POINT
[MouseAPI]::GetCursorPos([ref]$pt) | Out-Null
Write-Output "$($pt.X)|$($pt.Y)"
"#;
            let output = Command::new("powershell")
                .args(["-NoProfile", "-Command", script])
                .output().await?;
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let parts: Vec<&str> = stdout.splitn(2, '|').collect();
            let x = parts.get(0).and_then(|v| v.parse().ok()).unwrap_or(0);
            let y = parts.get(1).and_then(|v| v.parse().ok()).unwrap_or(0);
            return Ok(MouseInfo { x, y, display_id: None, semantic_context: None });
        }
        #[cfg(not(windows))]
        Ok(MouseInfo { x: 0, y: 0, display_id: None, semantic_context: None })
    }

    async fn monitors(&self) -> Result<Vec<MonitorInfo>> {
        #[cfg(windows)] {
            let output = Command::new("powershell")
                .args(["-NoProfile", "-Command", r#"
Add-Type -AssemblyName System.Windows.Forms
[System.Windows.Forms.Screen]::AllScreens | Select-Object DeviceName, Bounds, WorkingArea, Primary | ConvertTo-Json -Compress
                "#])
                .output().await?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut monitors = Vec::new();
            if let Ok(list) = serde_json::from_str::<Vec<serde_json::Value>>(&stdout) {
                for (i, item) in list.iter().enumerate() {
                    let bounds = item.get("Bounds").and_then(|b| {
                        Some(Rect::new(
                            b.get("X").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                            b.get("Y").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                            b.get("Width").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                            b.get("Height").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                        ))
                    }).unwrap_or_default();
                    let is_primary = item.get("Primary").and_then(|v| v.as_bool()).unwrap_or(false);
                    monitors.push(MonitorInfo {
                        id: i as u64,
                        name: item.get("DeviceName").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        bounds,
                        work_area: bounds,
                        scale_factor: 1.0,
                        refresh_rate_hz: None,
                        is_primary,
                        rotation: None,
                    });
                }
            }
            return Ok(monitors);
        }
        #[cfg(not(windows))]
        Ok(vec![])
    }

    async fn system_resources(&self) -> Result<SystemResources> {
        #[cfg(windows)] {
            let output = Command::new("powershell")
                .args(["-NoProfile", "-Command", r#"
$cpu = (Get-CimInstance Win32_Processor | Measure-Object -Property LoadPercentage -Average).Average
$os = Get-CimInstance Win32_OperatingSystem
$total = [math]::Round($os.TotalVisibleMemorySize / 1024)
$free = [math]::Round($os.FreePhysicalMemory / 1024)
$used = $total - $free
$pct = if ($total -gt 0) { [math]::Round(($used / $total) * 100, 1) } else { 0 }
Write-Output "$cpu|$total|$used|$pct"
                "#])
                .output().await?;
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let parts: Vec<&str> = stdout.splitn(4, '|').collect();
            let cpu = parts.get(0).and_then(|v| v.parse().ok()).unwrap_or(0.0);
            let total = parts.get(1).and_then(|v| v.parse().ok()).unwrap_or(0);
            let used = parts.get(2).and_then(|v| v.parse().ok()).unwrap_or(0);
            let pct = parts.get(3).and_then(|v| v.parse().ok()).unwrap_or(0.0);
            return Ok(SystemResources {
                cpu_usage_percent: cpu,
                memory_total_mb: total,
                memory_used_mb: used,
                memory_percent: pct,
                swap_total_mb: 0,
                swap_used_mb: 0,
                disk_read_mb: None,
                disk_write_mb: None,
                load_average: None,
            });
        }
        #[cfg(not(windows))]
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
        #[cfg(windows)] {
            let output = Command::new("powershell")
                .args(["-NoProfile", "-Command", r#"
Get-NetAdapter | Where-Object Status -eq 'Up' | Select-Object Name, MacAddress, LinkSpeed | ConvertTo-Json -Compress
                "#])
                .output().await?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut interfaces = Vec::new();
            if let Ok(list) = serde_json::from_str::<Vec<serde_json::Value>>(&stdout) {
                for item in list {
                    interfaces.push(NetworkInterface {
                        name: item.get("Name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        ip_addresses: vec![],
                        mac_address: item.get("MacAddress").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        is_up: true,
                        bytes_sent: 0,
                        bytes_received: 0,
                    });
                }
            }
            return Ok(NetworkState {
                is_connected: !interfaces.is_empty(),
                connectivity_type: if interfaces.is_empty() { ConnectivityType::None } else { ConnectivityType::Ethernet },
                interfaces,
            });
        }
        #[cfg(not(windows))]
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
        #[cfg(windows)] {
            let output = Command::new("powershell")
                .args(["-NoProfile", "-Command", r#"
$status = (Get-CimInstance Win32_Battery).BatteryStatus
$pct = (Get-CimInstance Win32_Battery).EstimatedChargeRemaining
$charging = $status -eq 2 -or $status -eq 6
Write-Output "$pct|$charging"
                "#])
                .output().await?;
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let parts: Vec<&str> = stdout.splitn(2, '|').collect();
            let pct = parts.get(0).and_then(|v| v.parse().ok());
            let charging = parts.get(1).and_then(|v| v.trim().parse::<bool>().ok()).unwrap_or(false);
            let source = if charging { PowerSource::Ac } else { PowerSource::Battery };
            return Ok(PowerState {
                source,
                battery_percent: pct,
                is_charging: charging,
                time_remaining_seconds: None,
            });
        }
        #[cfg(not(windows))]
        Ok(PowerState {
            source: PowerSource::Ac,
            battery_percent: None,
            is_charging: false,
            time_remaining_seconds: None,
        })
    }

    async fn workspace(&self) -> Result<WorkspaceInfo> {
        Ok(WorkspaceInfo { current: 0, total: 1, names: vec!["Desktop".to_string()] })
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
        #[cfg(windows)] {
            let output = Command::new("powershell")
                .args(["-NoProfile", "-Command", r#"
Get-StartApps | Select-Object Name, AppId | ConvertTo-Json -Compress
                "#])
                .output().await?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut apps = Vec::new();
            if let Ok(list) = serde_json::from_str::<Vec<serde_json::Value>>(&stdout) {
                for item in list {
                    apps.push(InstalledApp {
                        name: item.get("Name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        executable: None,
                        version: None,
                        category: None,
                    });
                }
            }
            return Ok(apps);
        }
        #[cfg(not(windows))]
        Ok(vec![])
    }

    async fn selected_text(&self) -> Result<Option<String>> {
        let clip = self.clipboard().await?;
        Ok(Some(clip.content).filter(|s| !s.is_empty()))
    }
}
