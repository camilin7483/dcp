//! Example DCP plugin — exposes system hostname, uptime, and load average.

use anyhow::Result;
use dcp_plugin_sdk::{Plugin, PluginContext, PluginRegistration, run_plugin};

struct SystemInfoPlugin;

#[async_trait::async_trait]
impl Plugin for SystemInfoPlugin {
    fn registration(&self) -> PluginRegistration {
        PluginRegistration {
            plugin_id: "system-info".into(),
            version: "0.1.0".into(),
            provides_context: vec!["systemInfo".into()],
            emits_events: vec![],
            handles_automation: vec![],
        }
    }

    async fn on_start(&self, _ctx: &PluginContext) -> Result<()> {
        tracing::info!("System info plugin started");
        Ok(())
    }

    async fn on_context_request(
        &self,
        _ctx: &PluginContext,
        key: &str,
    ) -> Option<serde_json::Value> {
        match key {
            "systemInfo" => {
                let hostname = hostname().unwrap_or_else(|| "unknown".into());
                let uptime = uptime_seconds();
                let load = load_average();

                Some(serde_json::json!({
                    "hostname": hostname,
                    "uptimeSeconds": uptime,
                    "loadAverage": load,
                }))
            }
            _ => None,
        }
    }
}

fn hostname() -> Option<String> {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .ok()
        .map(|s| s.trim().to_string())
}

fn uptime_seconds() -> u64 {
    std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| s.split_whitespace().next().map(String::from))
        .and_then(|s| s.parse::<f64>().ok())
        .map(|f| f as u64)
        .unwrap_or(0)
}

fn load_average() -> [f64; 3] {
    std::fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|s| {
            let parts: Vec<f64> = s
                .split_whitespace()
                .take(3)
                .filter_map(|p| p.parse().ok())
                .collect();
            if parts.len() == 3 {
                Some([parts[0], parts[1], parts[2]])
            } else {
                None
            }
        })
        .unwrap_or([0.0, 0.0, 0.0])
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    let plugin = SystemInfoPlugin;
    run_plugin(plugin).await
}
