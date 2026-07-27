//! Linux automation backend using xdotool, xclip, xdg-open.

use anyhow::{Context, Result};
use async_trait::async_trait;
use dcp_types::{KeyModifier, MouseButton};
use std::process::Command;

use super::AutomationBackend;

pub struct LinuxAutomation;

impl LinuxAutomation {
    pub fn new() -> Self {
        Self
    }

    fn xdotool(args: &[&str]) -> Result<()> {
        let output = Command::new("xdotool")
            .args(args)
            .output()
            .context("xdotool not found — install it with your package manager")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("xdotool error: {stderr}");
        }
        Ok(())
    }

    fn xdotool_output(args: &[&str]) -> Result<String> {
        let output = Command::new("xdotool")
            .args(args)
            .output()
            .context("xdotool not found")?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

#[async_trait]
impl AutomationBackend for LinuxAutomation {
    async fn move_mouse(&self, x: i32, y: i32) -> Result<()> {
        Self::xdotool(&["mousemove", &x.to_string(), &y.to_string()])
    }

    async fn click_mouse(&self, x: i32, y: i32, button: MouseButton) -> Result<()> {
        let btn = match button {
            MouseButton::Left => "1",
            MouseButton::Right => "3",
            MouseButton::Middle => "2",
        };
        Self::xdotool(&["mousemove", &x.to_string(), &y.to_string(), "click", btn])
    }

    async fn double_click(&self, x: i32, y: i32) -> Result<()> {
        Self::xdotool(&[
            "mousemove",
            &x.to_string(),
            &y.to_string(),
            "click",
            "--repeat",
            "2",
            "--delay",
            "50",
            "1",
        ])
    }

    async fn drag_mouse(&self, from_x: i32, from_y: i32, to_x: i32, to_y: i32) -> Result<()> {
        Self::xdotool(&["mousemove", &from_x.to_string(), &from_y.to_string()])?;
        Self::xdotool(&["mousedown", "1"])?;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        Self::xdotool(&["mousemove", "--sync", &to_x.to_string(), &to_y.to_string()])?;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        Self::xdotool(&["mouseup", "1"])?;
        Ok(())
    }

    async fn scroll(&self, x: i32, y: i32, _delta_x: i32, delta_y: i32) -> Result<()> {
        Self::xdotool(&["mousemove", &x.to_string(), &y.to_string()])?;
        if delta_y > 0 {
            Self::xdotool(&["click", "--repeat", &delta_y.abs().to_string(), "4"])?;
        } else if delta_y < 0 {
            Self::xdotool(&["click", "--repeat", &delta_y.abs().to_string(), "5"])?;
        }
        Ok(())
    }

    async fn type_text(&self, text: &str) -> Result<()> {
        Self::xdotool(&["type", "--clearmodifiers", text])
    }

    async fn press_key(&self, key: &str, modifiers: &[KeyModifier]) -> Result<()> {
        let mod_str: Vec<&str> = modifiers
            .iter()
            .map(|m| match m {
                KeyModifier::Shift => "shift",
                KeyModifier::Control => "ctrl",
                KeyModifier::Alt => "alt",
                KeyModifier::Meta => "super",
            })
            .collect();

        let combined = if mod_str.is_empty() {
            key.to_string()
        } else {
            format!("{}+{}", mod_str.join("+"), key)
        };

        Self::xdotool(&["key", "--clearmodifiers", &combined])
    }

    async fn press_hotkey(&self, keys: &[String]) -> Result<()> {
        let combined = keys.join("+");
        Self::xdotool(&["key", "--clearmodifiers", &combined])
    }

    async fn set_clipboard(&self, content: &str, _content_type: Option<&str>) -> Result<()> {
        // Try xclip first, fall back to wl-copy
        let result = Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(ref mut stdin) = child.stdin {
                    let _ = stdin.write_all(content.as_bytes());
                }
                child.wait()
            });

        if result.is_err() {
            // Try wl-copy for Wayland
            Command::new("wl-copy")
                .arg(content)
                .output()
                .context("neither xclip nor wl-copy found")?;
        }

        Ok(())
    }

    async fn launch_app(
        &self,
        executable: &str,
        args: &[String],
        working_dir: Option<&str>,
    ) -> Result<u32> {
        let mut cmd = Command::new(executable);
        cmd.args(args);
        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());

        let child = cmd.spawn().context("failed to launch application")?;
        Ok(child.id())
    }

    async fn focus_window(&self, window_id: u64) -> Result<()> {
        Self::xdotool(&["windowfocus", &window_id.to_string()])
    }

    async fn move_window(&self, window_id: u64, x: i32, y: i32) -> Result<()> {
        Self::xdotool(&[
            "windowmove",
            "--sync",
            &window_id.to_string(),
            &x.to_string(),
            &y.to_string(),
        ])
    }

    async fn resize_window(&self, window_id: u64, width: u32, height: u32) -> Result<()> {
        Self::xdotool(&[
            "windowsize",
            "--sync",
            &window_id.to_string(),
            &width.to_string(),
            &height.to_string(),
        ])
    }

    async fn minimize_window(&self, window_id: u64) -> Result<()> {
        // xdotool doesn't have direct minimize; use wmctrl or xdotool key
        let _ = Command::new("xdotool")
            .args(["windowminimize", &window_id.to_string()])
            .output();
        Ok(())
    }

    async fn maximize_window(&self, window_id: u64) -> Result<()> {
        // Use wmctrl if available
        let _ = Command::new("wmctrl")
            .args([
                "-i",
                "-r",
                &window_id.to_string(),
                "-b",
                "add,maximized_vert,maximized_horz",
            ])
            .output();
        Ok(())
    }

    async fn restore_window(&self, window_id: u64) -> Result<()> {
        let _ = Command::new("wmctrl")
            .args([
                "-i",
                "-r",
                &window_id.to_string(),
                "-b",
                "remove,maximized_vert,maximized_horz",
            ])
            .output();
        Ok(())
    }

    async fn close_window(&self, window_id: u64) -> Result<()> {
        Self::xdotool(&["windowclose", &window_id.to_string()])
    }

    async fn open_file(&self, path: &str) -> Result<()> {
        Command::new("xdg-open")
            .arg(path)
            .spawn()
            .context("xdg-open not found")?;
        Ok(())
    }
}
