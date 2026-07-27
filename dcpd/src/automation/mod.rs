//! Automation module: mouse, keyboard, clipboard control.
//!
//! Automation is separated from observation via permissions.
//! Each platform implements the `AutomationBackend` trait.

pub mod executor;

pub use executor::LinuxAutomation;

use anyhow::Result;
use async_trait::async_trait;
use dcp_types::{AutomationCommand, AutomationResult, MouseButton, KeyModifier};

/// Platform-agnostic automation backend trait.
#[async_trait]
pub trait AutomationBackend: Send + Sync {
    async fn move_mouse(&self, x: i32, y: i32) -> Result<()>;
    async fn click_mouse(&self, x: i32, y: i32, button: MouseButton) -> Result<()>;
    async fn double_click(&self, x: i32, y: i32) -> Result<()>;
    async fn drag_mouse(&self, from_x: i32, from_y: i32, to_x: i32, to_y: i32) -> Result<()>;
    async fn scroll(&self, x: i32, y: i32, delta_x: i32, delta_y: i32) -> Result<()>;
    async fn type_text(&self, text: &str) -> Result<()>;
    async fn press_key(&self, key: &str, modifiers: &[KeyModifier]) -> Result<()>;
    async fn press_hotkey(&self, keys: &[String]) -> Result<()>;
    async fn set_clipboard(&self, content: &str, content_type: Option<&str>) -> Result<()>;
    async fn launch_app(&self, executable: &str, args: &[String], working_dir: Option<&str>) -> Result<u32>;
    async fn focus_window(&self, window_id: u64) -> Result<()>;
    async fn move_window(&self, window_id: u64, x: i32, y: i32) -> Result<()>;
    async fn resize_window(&self, window_id: u64, width: u32, height: u32) -> Result<()>;
    async fn minimize_window(&self, window_id: u64) -> Result<()>;
    async fn maximize_window(&self, window_id: u64) -> Result<()>;
    async fn restore_window(&self, window_id: u64) -> Result<()>;
    async fn close_window(&self, window_id: u64) -> Result<()>;
    async fn open_file(&self, path: &str) -> Result<()>;

    /// Execute any automation command.
    async fn execute(&self, command: &AutomationCommand, dry_run: bool) -> Result<AutomationResult> {
        if dry_run {
            return Ok(AutomationResult {
                success: true,
                message: Some(format!("dry run: {:?}", command)),
            });
        }

        let result = match command {
            AutomationCommand::MouseMove { x, y } => {
                self.move_mouse(*x, *y).await.map(|_| "mouse moved")
            }
            AutomationCommand::MouseClick { x, y, button } => {
                self.click_mouse(*x, *y, *button).await.map(|_| "clicked")
            }
            AutomationCommand::MouseDoubleClick { x, y } => {
                self.double_click(*x, *y).await.map(|_| "double-clicked")
            }
            AutomationCommand::MouseDrag { from_x, from_y, to_x, to_y } => {
                self.drag_mouse(*from_x, *from_y, *to_x, *to_y).await.map(|_| "dragged")
            }
            AutomationCommand::MouseScroll { x, y, delta_x, delta_y } => {
                self.scroll(*x, *y, *delta_x, *delta_y).await.map(|_| "scrolled")
            }
            AutomationCommand::KeyboardType { text } => {
                self.type_text(text).await.map(|_| "typed")
            }
            AutomationCommand::KeyboardKey { key, modifiers } => {
                self.press_key(key, modifiers).await.map(|_| "key pressed")
            }
            AutomationCommand::KeyboardHotkey { keys } => {
                self.press_hotkey(keys).await.map(|_| "hotkey pressed")
            }
            AutomationCommand::ClipboardSet { content, content_type } => {
                self.set_clipboard(content, content_type.as_deref()).await.map(|_| "clipboard set")
            }
            AutomationCommand::AppLaunch { executable, args, working_dir } => {
                self.launch_app(executable, args, working_dir.as_deref()).await.map(|_| "app launched")
            }
            AutomationCommand::WindowFocus { window_id } => {
                self.focus_window(*window_id).await.map(|_| "window focused")
            }
            AutomationCommand::WindowMove { window_id, x, y } => {
                self.move_window(*window_id, *x, *y).await.map(|_| "window moved")
            }
            AutomationCommand::WindowResize { window_id, width, height } => {
                self.resize_window(*window_id, *width, *height).await.map(|_| "window resized")
            }
            AutomationCommand::WindowMinimize { window_id } => {
                self.minimize_window(*window_id).await.map(|_| "window minimized")
            }
            AutomationCommand::WindowMaximize { window_id } => {
                self.maximize_window(*window_id).await.map(|_| "window maximized")
            }
            AutomationCommand::WindowRestore { window_id } => {
                self.restore_window(*window_id).await.map(|_| "window restored")
            }
            AutomationCommand::WindowClose { window_id } => {
                self.close_window(*window_id).await.map(|_| "window closed")
            }
            AutomationCommand::FileOpen { path } => {
                self.open_file(path).await.map(|_| "file opened")
            }
        };

        match result {
            Ok(msg) => Ok(AutomationResult {
                success: true,
                message: Some(msg.to_string()),
            }),
            Err(e) => Ok(AutomationResult {
                success: false,
                message: Some(e.to_string()),
            }),
        }
    }
}
