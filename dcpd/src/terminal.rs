//! Terminal output capture for Linux.
//!
//! Captures output from terminal emulators by reading from pseudo-terminals
//! or using terminal-specific protocols.

use crate::events::EventBus;
use anyhow::Result;
use dcp_types::{EventData, EventType, SystemEvent, TerminalEventData};
use std::collections::HashMap;
use tokio::io::AsyncReadExt;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Terminal session tracker.
pub struct TerminalCapture {
    event_bus: EventBus,
    sessions: RwLock<HashMap<String, TerminalSession>>,
}

struct TerminalSession {
    id: String,
    pid: u32,
    cwd: String,
    shell: String,
    last_command: Option<String>,
    last_output: String,
}

impl TerminalCapture {
    pub fn new(event_bus: EventBus) -> Self {
        Self {
            event_bus,
            sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Track a terminal session by PID.
    pub async fn track_session(&self, pid: u32) -> Result<String> {
        let id = format!("term_{pid}");

        // Read /proc/{pid}/cmdline to get shell
        let cmdline = std::fs::read_to_string(format!("/proc/{pid}/cmdline"))
            .unwrap_or_default()
            .replace('\0', " ")
            .trim()
            .to_string();

        let shell = cmdline
            .split_whitespace()
            .last()
            .unwrap_or("unknown")
            .to_string();

        // Read /proc/{pid}/cwd
        let cwd = std::fs::read_link(format!("/proc/{pid}/cwd"))
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "/".to_string());

        let session = TerminalSession {
            id: id.clone(),
            pid,
            cwd,
            shell: shell.clone(),
            last_command: None,
            last_output: String::new(),
        };

        self.sessions.write().await.insert(id.clone(), session);
        info!("Tracking terminal session: {id} (PID {pid}, shell: {shell})");

        Ok(id)
    }

    /// Update terminal output (called when new output is detected).
    pub async fn update_output(&self, session_id: &str, output: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            // Keep last N lines
            let lines: Vec<&str> = output.lines().collect();
            let max_lines = 50;
            let start = lines.len().saturating_sub(max_lines);
            session.last_output = lines[start..].join("\n");

            // Try to detect last command
            if let Some(last_line) = lines.last() {
                if last_line.contains("$ ") || last_line.contains("# ") || last_line.contains("> ")
                {
                    session.last_command = Some(last_line.to_string());
                }
            }

            let data = EventData::Terminal(TerminalEventData {
                terminal_id: session_id.to_string(),
                pid: session.pid,
                content: Some(session.last_output.clone()),
                cwd: Some(session.cwd.clone()),
            });

            let event = SystemEvent::new(EventType::TerminalOutputReceived, data);
            self.event_bus.publish(event).await;
        }
    }

    /// Get current terminal info.
    pub async fn get_info(&self, session_id: &str) -> Option<dcp_types::TerminalInfo> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).map(|s| dcp_types::TerminalInfo {
            id: s.id.clone(),
            pid: s.pid,
            cwd: s.cwd.clone(),
            shell: s.shell.clone(),
            last_command: s.last_command.clone(),
            last_output: Some(s.last_output.clone()),
            title: None,
        })
    }

    /// List all tracked terminals.
    pub async fn list_terminals(&self) -> Vec<dcp_types::TerminalInfo> {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .map(|s| dcp_types::TerminalInfo {
                id: s.id.clone(),
                pid: s.pid,
                cwd: s.cwd.clone(),
                shell: s.shell.clone(),
                last_command: s.last_command.clone(),
                last_output: Some(s.last_output.clone()),
                title: None,
            })
            .collect()
    }
}

/// Auto-detect and track running terminal sessions.
pub async fn auto_detect_terminals(capture: &TerminalCapture) -> Result<Vec<String>> {
    let mut terminal_pids = Vec::new();

    // Look for common terminal processes
    let terminal_names = [
        "bash",
        "zsh",
        "fish",
        "sh",
        "ksh",
        "csh",
        "tcsh",
        "alacritty",
        "kitty",
        "gnome-terminal",
        "konsole",
        "xterm",
        "wezterm",
        "terminator",
        "tilix",
    ];

    for entry in std::fs::read_dir("/proc")? {
        let entry = entry?;
        let name = entry.file_name();
        if let Ok(pid) = name.to_string_lossy().parse::<u32>() {
            if let Ok(comm) = std::fs::read_to_string(format!("/proc/{pid}/comm")) {
                let comm = comm.trim();
                if terminal_names.iter().any(|t| comm.contains(t)) {
                    terminal_pids.push(pid);
                }
            }
        }
    }

    let mut tracked = Vec::new();
    for pid in terminal_pids {
        match capture.track_session(pid).await {
            Ok(id) => tracked.push(id),
            Err(e) => warn!("Failed to track terminal PID {pid}: {e}"),
        }
    }

    info!("Auto-detected {} terminal sessions", tracked.len());
    Ok(tracked)
}
