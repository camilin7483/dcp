//! DCP Plugin SDK — Rust bindings for building DCP plugins.
//!
//! Plugins communicate with dcpd via a dedicated Unix socket.
//! Each plugin runs as an independent process for crash isolation.
//!
//! # Example
//! ```rust,no_run
//! use dcp_plugin_sdk::{Plugin, PluginContext, PluginRegistration};
//!
//! struct MyPlugin;
//!
//! #[async_trait::async_trait]
//! impl Plugin for MyPlugin {
//!     fn registration(&self) -> PluginRegistration {
//!         PluginRegistration {
//!             plugin_id: "my-plugin".into(),
//!             version: "1.0.0".into(),
//!             provides_context: vec!["myPlugin.data".into()],
//!             emits_events: vec!["myPlugin.changed".into()],
//!             handles_automation: vec![],
//!         }
//!     }
//!
//!     async fn on_context_request(&self, ctx: &PluginContext, key: &str) -> Option<serde_json::Value> {
//!         Some(serde_json::json!({"hello": "world"}))
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let plugin = MyPlugin;
//!     dcp_plugin_sdk::run_plugin(plugin).await
//! }
//! ```

pub use dcp_types;
pub use serde_json;
pub use tokio;
pub use tracing;

use anyhow::Result;
use dcp_types::{EventType, Request, RequestId, Response};
use futures::{SinkExt, StreamExt};
use std::path::PathBuf;
use tokio::net::UnixStream;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tracing::{error, info};

/// Plugin registration info sent to dcpd on connect.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRegistration {
    pub plugin_id: String,
    pub version: String,
    pub provides_context: Vec<String>,
    pub emits_events: Vec<String>,
    pub handles_automation: Vec<String>,
}

/// Runtime context passed to plugin callbacks.
pub struct PluginContext {
    pub plugin_id: String,
}

/// Trait that all DCP plugins must implement.
#[async_trait::async_trait]
pub trait Plugin: Send + Sync + 'static {
    /// Returns the plugin's registration info.
    fn registration(&self) -> PluginRegistration;

    /// Called when dcpd requests context from this plugin.
    async fn on_context_request(
        &self,
        ctx: &PluginContext,
        key: &str,
    ) -> Option<serde_json::Value> {
        let _ = (ctx, key);
        None
    }

    /// Called when dcpd sends an automation command to this plugin.
    async fn on_automation(
        &self,
        ctx: &PluginContext,
        command: &str,
        args: serde_json::Value,
    ) -> Option<serde_json::Value> {
        let _ = (ctx, command, args);
        None
    }

    /// Called when the plugin starts.
    async fn on_start(&self, _ctx: &PluginContext) -> Result<()> {
        Ok(())
    }

    /// Called when the plugin is shutting down.
    async fn on_stop(&self, _ctx: &PluginContext) {}
}

/// Run a plugin — connects to dcpd, registers, and processes requests.
pub async fn run_plugin<P: Plugin>(plugin: P) -> Result<()> {
    let reg = plugin.registration();
    let socket_path = resolve_plugin_socket(&reg.plugin_id)?;

    info!("Plugin {} v{} starting", reg.plugin_id, reg.version);
    info!("Connecting to dcpd at {}", socket_path.display());

    let plugin_id = reg.plugin_id.clone();
    let ctx = PluginContext {
        plugin_id: plugin_id.clone(),
    };

    plugin.on_start(&ctx).await?;

    let stream = UnixStream::connect(&socket_path).await?;
    let codec = LengthDelimitedCodec::builder()
        .length_field_length(4)
        .max_frame_length(16 * 1024 * 1024)
        .new_codec();
    let mut framed = Framed::new(stream, codec);

    let register_req = Request::new(
        1,
        "plugin.register",
        serde_json::json!({
            "pluginId": reg.plugin_id,
            "version": reg.version,
            "capabilities": {
                "providesContext": reg.provides_context,
                "emitsEvents": reg.emits_events,
                "handlesAutomation": reg.handles_automation,
            }
        }),
    );

    let bytes = serde_json::to_vec(&register_req)?;
    framed.send(bytes.into()).await?;
    info!("Registered with dcpd");

    while let Some(frame) = framed.next().await {
        match frame {
            Ok(bytes) => {
                if let Ok(request) = serde_json::from_slice::<Request>(&bytes) {
                    let response = handle_request(&plugin, &ctx, &request).await;
                    if let Some(resp) = response {
                        let resp_bytes = serde_json::to_vec(&resp)?;
                        framed.send(resp_bytes.into()).await?;
                    }
                }
            }
            Err(e) => {
                error!("Plugin connection error: {e}");
                break;
            }
        }
    }

    plugin.on_stop(&ctx).await;
    Ok(())
}

async fn handle_request<P: Plugin>(
    plugin: &P,
    ctx: &PluginContext,
    request: &Request,
) -> Option<Response> {
    let id = request.id.clone().unwrap_or(RequestId::Integer(0));

    match request.method.as_str() {
        "context.request" => {
            let key = request
                .params
                .as_ref()
                .and_then(|p| p.get("key"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let result = plugin.on_context_request(ctx, key).await;
            Some(Response::success(
                id,
                result.unwrap_or(serde_json::Value::Null),
            ))
        }
        "automation.execute" => {
            let command = request
                .params
                .as_ref()
                .and_then(|p| p.get("command"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let args = request
                .params
                .as_ref()
                .and_then(|p| p.get("args"))
                .cloned()
                .unwrap_or(serde_json::Value::Null);

            let result = plugin.on_automation(ctx, command, args).await;
            Some(Response::success(
                id,
                result.unwrap_or(serde_json::Value::Null),
            ))
        }
        "ping" => Some(Response::success(id, serde_json::json!({"pong": true}))),
        _ => Some(Response::error(
            id,
            dcp_types::ErrorCode::MethodNotFound,
            format!("Unknown method: {}", request.method),
        )),
    }
}

fn resolve_plugin_socket(plugin_id: &str) -> Result<PathBuf> {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    Ok(PathBuf::from(format!(
        "{runtime_dir}/dcpd/plugins/{plugin_id}.sock"
    )))
}

/// Helper to emit an event from within a plugin.
pub async fn emit_event(
    framed: &mut Framed<UnixStream, LengthDelimitedCodec>,
    event_type: EventType,
    data: serde_json::Value,
) -> Result<()> {
    let notification = Request::notification(
        "event",
        serde_json::json!({
            "eventType": event_type,
            "data": data,
            "timestamp": chrono::Utc::now().timestamp_millis(),
        }),
    );
    let bytes = serde_json::to_vec(&notification)?;
    framed.send(bytes.into()).await?;
    Ok(())
}
