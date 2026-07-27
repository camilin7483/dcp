//! Configuration file support for dcpd.
//!
//! Loads configuration from `~/.config/dcpd/config.toml` or a path specified
//! via the `--config` CLI argument.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Top-level daemon configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DaemonConfig {
    /// Socket path override.
    pub socket: Option<String>,
    /// Log level.
    pub log_level: String,
    /// Plugin directory override.
    pub plugin_dir: Option<String>,
    /// Audit log directory override.
    pub audit_dir: Option<String>,
    /// Enable remote WebSocket server.
    pub remote: bool,
    /// Remote listen address.
    pub remote_addr: Option<String>,
    /// TLS certificate path.
    pub tls_cert: Option<String>,
    /// TLS key path.
    pub tls_key: Option<String>,
    /// Enable vision module.
    pub vision: bool,
    /// File watcher paths.
    pub watch_paths: Vec<String>,
    /// Permission defaults for local sessions.
    pub default_permissions: Vec<String>,
    /// Plugin-specific configuration.
    pub plugins: std::collections::HashMap<String, toml::Value>,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            socket: None,
            log_level: "info".to_string(),
            plugin_dir: None,
            audit_dir: None,
            remote: false,
            remote_addr: None,
            tls_cert: None,
            tls_key: None,
            vision: false,
            watch_paths: vec![],
            default_permissions: vec![],
            plugins: std::collections::HashMap::new(),
        }
    }
}

impl DaemonConfig {
    /// Load configuration from the default location.
    pub fn load_default() -> Self {
        let config_path = default_config_path();
        Self::load_from(&config_path).unwrap_or_default()
    }

    /// Load configuration from a specific file.
    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path)?;
        let config: DaemonConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save default configuration to a file (for bootstrapping).
    pub fn save_default(path: &Path) -> Result<()> {
        let config = Self::default();
        let content = toml::to_string_pretty(&config)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }
}

/// Get the default config file path.
pub fn default_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("dcpd")
        .join("config.toml")
}

/// Get the default data directory.
pub fn default_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("dcpd")
}
