//! Plugin host: external process supervisor, lifecycle management, auto-discovery.
//!
//! The plugin host discovers plugins in the configured directory, spawns them as
//! independent processes, monitors their health, and restarts crashed plugins.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Child;
use tracing::{info, warn, error};

use crate::events::EventBus;
use dcp_types::{EventType, EventData, PluginEventData, SystemEvent};

/// Plugin host — discovers, spawns, and supervises plugin processes.
pub struct PluginHost {
    plugin_dir: PathBuf,
    plugins: Arc<tokio::sync::RwLock<HashMap<String, PluginInstance>>>,
    event_bus: EventBus,
    max_restarts: u32,
    restart_cooldown: Duration,
}

impl PluginHost {
    pub fn new(plugin_dir: PathBuf, event_bus: EventBus) -> Self {
        std::fs::create_dir_all(&plugin_dir).ok();
        Self {
            plugin_dir,
            plugins: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            event_bus,
            max_restarts: 5,
            restart_cooldown: Duration::from_secs(10),
        }
    }

    /// Discover all plugins in the plugin directory.
    pub async fn discover_plugins(&self) -> Vec<PluginManifest> {
        let mut manifests = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&self.plugin_dir) {
            for entry in entries.flatten() {
                let path = entry.path();

                // Look for dcp-plugin.json manifest
                let manifest_path = path.join("dcp-plugin.json");
                if manifest_path.exists() {
                    match std::fs::read_to_string(&manifest_path) {
                        Ok(content) => {
                            match serde_json::from_str::<PluginManifest>(&content) {
                                Ok(manifest) => {
                                    info!("Discovered plugin: {} v{}", manifest.plugin_id, manifest.version);
                                    manifests.push(manifest);
                                }
                                Err(e) => {
                                    warn!("Invalid manifest at {}: {e}", manifest_path.display());
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to read manifest at {}: {e}", manifest_path.display());
                        }
                    }
                }
            }
        }

        info!("Discovered {} plugin(s)", manifests.len());
        manifests
    }

    /// Auto-start all discovered plugins.
    pub async fn auto_start(&self) -> Vec<String> {
        let manifests = self.discover_plugins().await;
        let mut started = Vec::new();

        for manifest in manifests {
            match self.start_plugin(manifest).await {
                Ok(id) => started.push(id),
                Err(e) => error!("Failed to auto-start plugin: {e}"),
            }
        }

        started
    }

    /// Start a plugin process.
    pub async fn start_plugin(&self, manifest: PluginManifest) -> anyhow::Result<String> {
        let plugin_id = manifest.plugin_id.clone();
        info!("Starting plugin: {} v{}", plugin_id, manifest.version);

        // Check if already running
        if self.plugins.read().await.contains_key(&plugin_id) {
            warn!("Plugin {plugin_id} already running");
            return Ok(plugin_id);
        }

        let plugin_dir = self.plugin_dir.join(&plugin_id);
        let executable = plugin_dir.join(&manifest.executable);

        if !executable.exists() {
            anyhow::bail!("Plugin executable not found: {}", executable.display());
        }

        // Create plugin socket directory
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .unwrap_or_else(|_| "/tmp".to_string());
        let plugin_socket_dir = PathBuf::from(format!("{runtime_dir}/dcpd/plugins"));
        std::fs::create_dir_all(&plugin_socket_dir)?;

        let socket_path = format!("{}/{}.sock", plugin_socket_dir.display(), plugin_id);

        // Remove stale socket
        let _ = std::fs::remove_file(&socket_path);

        // Spawn plugin process
        let child = tokio::process::Command::new(&executable)
            .arg("--socket")
            .arg(&socket_path)
            .current_dir(&plugin_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        let pid = child.id();

        let instance = PluginInstance {
            manifest: manifest.clone(),
            child,
            socket_path: socket_path.clone(),
            pid,
            restart_count: 0,
            started_at: std::time::Instant::now(),
            status: PluginStatus::Running,
        };

        self.plugins.write().await.insert(plugin_id.clone(), instance);

        // Emit plugin registered event
        let event = SystemEvent::new(
            EventType::PluginRegistered,
            EventData::Plugin(PluginEventData {
                plugin_id: plugin_id.clone(),
                version: Some(manifest.version),
            }),
        );
        self.event_bus.publish(event).await;

        info!("Plugin {plugin_id} started (PID: {pid:?})");
        Ok(plugin_id)
    }

    /// Stop a plugin process.
    pub async fn stop_plugin(&self, plugin_id: &str) -> bool {
        let mut plugins = self.plugins.write().await;
        if let Some(mut instance) = plugins.remove(plugin_id) {
            instance.status = PluginStatus::Stopping;
            let _ = instance.child.kill().await;

            // Emit plugin unregistered event
            let event = SystemEvent::new(
                EventType::PluginUnregistered,
                EventData::Plugin(PluginEventData {
                    plugin_id: plugin_id.to_string(),
                    version: Some(instance.manifest.version.clone()),
                }),
            );
            drop(plugins);
            self.event_bus.publish(event).await;

            info!("Plugin {plugin_id} stopped");
            true
        } else {
            false
        }
    }

    /// Stop all plugins (for graceful shutdown).
    pub async fn stop_all(&self) {
        let plugin_ids: Vec<String> = self.plugins.read().await.keys().cloned().collect();
        for id in plugin_ids {
            self.stop_plugin(&id).await;
        }
    }

    /// Check health of all plugins and restart crashed ones.
    pub async fn health_check(&self) {
        let mut plugins = self.plugins.write().await;
        let mut to_restart: Vec<(String, PluginManifest)> = Vec::new();

        for (id, instance) in plugins.iter_mut() {
            if instance.status != PluginStatus::Running {
                continue;
            }
            match instance.child.try_wait() {
                Ok(Some(status)) => {
                    warn!("Plugin {id} exited with status: {status}");
                    instance.status = PluginStatus::Crashed;
                    if instance.restart_count < self.max_restarts {
                        to_restart.push((id.clone(), instance.manifest.clone()));
                    } else {
                        instance.status = PluginStatus::Failed;
                        error!("Plugin {id} exceeded max restarts ({})", self.max_restarts);
                    }
                }
                Ok(None) => {
                    // still alive, nothing to do
                }
                Err(e) => {
                    error!("Plugin {id} health check error: {e}");
                }
            }
        }
        drop(plugins);

        for (id, manifest) in to_restart {
            let mut plugins = self.plugins.write().await;
            if let Some(instance) = plugins.get_mut(&id) {
                instance.restart_count += 1;
                warn!("Restarting plugin {id} (attempt {}/{})",
                    instance.restart_count, self.max_restarts);

                let plugin_dir = self.plugin_dir.join(&id);
                let executable = plugin_dir.join(&manifest.executable);
                let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
                    .unwrap_or_else(|_| "/tmp".to_string());
                let socket_path = format!("{runtime_dir}/dcpd/plugins/{id}.sock");

                match tokio::process::Command::new(&executable)
                    .arg("--socket").arg(&socket_path)
                    .current_dir(&plugin_dir)
                    .spawn()
                {
                    Ok(child) => {
                        instance.child = child;
                        instance.pid = instance.child.id();
                        instance.status = PluginStatus::Running;
                        instance.started_at = std::time::Instant::now();
                        info!("Plugin {id} restarted successfully");
                    }
                    Err(e) => {
                        error!("Failed to restart plugin {id}: {e}");
                        instance.status = PluginStatus::Crashed;
                    }
                }
            }
        }
    }

    /// Run the health check loop.
    pub async fn run_health_loop(self: Arc<Self>) {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            self.health_check().await;
        }
    }

    /// List all plugins with their status.
    pub async fn list_plugins(&self) -> Vec<PluginInfo> {
        self.plugins.read().await.iter().map(|(id, instance)| {
            PluginInfo {
                id: id.clone(),
                version: instance.manifest.version.clone(),
                pid: instance.pid,
                status: instance.status,
                restart_count: instance.restart_count,
                socket_path: instance.socket_path.clone(),
                provides_context: instance.manifest.capabilities.provides_context.clone(),
                emits_events: instance.manifest.capabilities.emits_events.clone(),
            }
        }).collect()
    }
}

/// Information about a running plugin.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInfo {
    pub id: String,
    pub version: String,
    pub pid: Option<u32>,
    pub status: PluginStatus,
    pub restart_count: u32,
    pub socket_path: String,
    pub provides_context: Vec<String>,
    pub emits_events: Vec<String>,
}

/// Plugin manifest (dcp-plugin.json).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub plugin_id: String,
    pub version: String,
    pub executable: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default)]
    pub config: serde_json::Value,
    pub capabilities: PluginCapabilities,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCapabilities {
    pub provides_context: Vec<String>,
    pub emits_events: Vec<String>,
    #[serde(default)]
    pub handles_automation: Vec<String>,
}

/// Plugin lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginStatus {
    Starting,
    Running,
    Stopping,
    Crashed,
    Failed,
}

/// A running plugin instance.
pub struct PluginInstance {
    pub manifest: PluginManifest,
    pub child: Child,
    pub socket_path: String,
    pub pid: Option<u32>,
    pub restart_count: u32,
    pub started_at: std::time::Instant,
    pub status: PluginStatus,
}
