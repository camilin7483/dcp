//! Browser integration plugin for DCP.
//!
//! This plugin monitors browser windows and extracts tab information
//! from window titles. It demonstrates how to build a DCP plugin that
//! provides browser context to AI agents.
//!
//! Supported browsers: Firefox, Chrome, Chromium, Edge, Brave
//!
//! Note: For full browser integration, a browser extension would be needed
//! to access the complete tab list and URLs. This plugin uses window title
//! parsing as a simpler demonstration.

use anyhow::Result;
use async_trait::async_trait;
use dcp_plugin_sdk::{Plugin, PluginContext, PluginRegistration};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::process::Command;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrowserTab {
    title: String,
    url: Option<String>,
    browser: String,
}

struct BrowserPlugin {
    browser_names: Vec<String>,
}

impl BrowserPlugin {
    fn new() -> Self {
        Self {
            browser_names: vec![
                "Firefox".to_string(),
                "Google Chrome".to_string(),
                "Chromium".to_string(),
                "Microsoft Edge".to_string(),
                "Brave".to_string(),
            ],
        }
    }

    /// Get current browser window title using xdotool.
    fn get_browser_title(&self) -> Option<String> {
        // Find the active window
        let output = Command::new("xdotool")
            .args(["getactivewindow", "getwindowname"])
            .output()
            .ok()?;

        let title = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // Check if it's a browser window
        if self.browser_names.iter().any(|b| title.contains(b)) {
            Some(title)
        } else {
            None
        }
    }

    /// Parse browser title to extract page title and URL.
    fn parse_browser_title(&self, title: &str) -> Option<BrowserTab> {
        // Browser title format: "Page Title — Browser Name" or "Page Title - Browser Name"
        let re =
            Regex::new(r"(.+?)\s*[—\-]\s*(Firefox|Google Chrome|Chromium|Microsoft Edge|Brave)")
                .ok()?;

        if let Some(captures) = re.captures(title) {
            let page_title = captures.get(1)?.as_str().trim().to_string();
            let browser = captures.get(2)?.as_str().to_string();

            Some(BrowserTab {
                title: page_title,
                url: None, // URL not available from window title alone
                browser,
            })
        } else {
            None
        }
    }

    /// Get all browser windows.
    fn get_browser_windows(&self) -> Vec<BrowserTab> {
        let output = Command::new("xdotool")
            .args(["search", "--name", ""])
            .output()
            .ok();

        let mut tabs = Vec::new();

        if let Some(output) = output {
            let window_ids: Vec<String> = String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(String::from)
                .collect();

            for wid in window_ids {
                if let Ok(title_output) = Command::new("xdotool")
                    .args(["getwindowname", &wid])
                    .output()
                {
                    let title = String::from_utf8_lossy(&title_output.stdout)
                        .trim()
                        .to_string();

                    if let Some(tab) = self.parse_browser_title(&title) {
                        tabs.push(tab);
                    }
                }
            }
        }

        tabs
    }
}

#[async_trait]
impl Plugin for BrowserPlugin {
    fn registration(&self) -> PluginRegistration {
        PluginRegistration {
            plugin_id: "browser".to_string(),
            version: "0.1.0".to_string(),
            provides_context: vec!["browser.tabs".to_string(), "browser.active".to_string()],
            emits_events: vec!["browser.tab.changed".to_string()],
            handles_automation: vec![],
        }
    }

    async fn on_start(&self, ctx: &PluginContext) -> Result<()> {
        info!("Browser plugin started: {}", ctx.plugin_id);
        Ok(())
    }

    async fn on_context_request(
        &self,
        _ctx: &PluginContext,
        key: &str,
    ) -> Option<serde_json::Value> {
        match key {
            "browser.tabs" => {
                let tabs = self.get_browser_windows();
                Some(serde_json::to_value(tabs).unwrap_or(serde_json::Value::Null))
            }
            "browser.active" => {
                if let Some(title) = self.get_browser_title() {
                    if let Some(tab) = self.parse_browser_title(&title) {
                        return Some(serde_json::to_value(tab).unwrap_or(serde_json::Value::Null));
                    }
                }
                Some(serde_json::Value::Null)
            }
            _ => None,
        }
    }

    async fn on_stop(&self, ctx: &PluginContext) {
        info!("Browser plugin stopped: {}", ctx.plugin_id);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    info!("Starting browser plugin");

    let plugin = BrowserPlugin::new();
    dcp_plugin_sdk::run_plugin(plugin).await
}
