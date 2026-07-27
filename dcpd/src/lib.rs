//! DCP Daemon — Desktop Context Protocol core server.

pub mod automation;
pub mod audit;
pub mod cache;
pub mod config;
pub mod dbus;
pub mod events;
pub mod permissions;
pub mod platform;
pub mod plugins;
pub mod server;
pub mod terminal;
pub mod vision;
pub mod watcher;
pub mod wayland;
pub mod websocket;

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, Level};

/// DCP Daemon — Desktop Context Protocol server.
#[derive(Parser, Debug)]
#[command(name = "dcpd", version, about)]
pub struct DaemonArgs {
    /// Socket path (default: $XDG_RUNTIME_DIR/dcpd.sock)
    #[arg(long)]
    pub socket: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    pub log_level: String,

    /// Allow remote connections (TLS required)
    #[arg(long)]
    pub remote: bool,

    /// Remote listen address (e.g., 127.0.0.1:9527)
    #[arg(long)]
    pub remote_addr: Option<String>,

    /// TLS certificate path (for remote mode)
    #[arg(long)]
    pub tls_cert: Option<PathBuf>,

    /// TLS private key path (for remote mode)
    #[arg(long)]
    pub tls_key: Option<PathBuf>,

    /// Plugin directory
    #[arg(long)]
    pub plugin_dir: Option<PathBuf>,

    /// Enable vision module
    #[arg(long)]
    pub vision: bool,

    /// Audit log path
    #[arg(long)]
    pub audit_dir: Option<PathBuf>,

    /// Config file path
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Run as foreground process
    #[arg(long)]
    pub foreground: bool,
}

impl DaemonArgs {
    pub fn socket_path(&self) -> PathBuf {
        self.socket.clone().unwrap_or_else(|| {
            let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
                .unwrap_or_else(|_| "/tmp".to_string());
            PathBuf::from(format!("{runtime_dir}/dcpd.sock"))
        })
    }

    pub fn plugin_directory(&self) -> PathBuf {
        self.plugin_dir.clone().unwrap_or_else(|| {
            let data_dir = std::env::var("XDG_DATA_HOME")
                .unwrap_or_else(|_| {
                    let home = std::env::var("HOME").unwrap_or_else(|_| "/home".to_string());
                    format!("{home}/.local/share")
                });
            PathBuf::from(format!("{data_dir}/dcpd/plugins"))
        })
    }

    pub fn audit_directory(&self) -> PathBuf {
        self.audit_dir.clone().unwrap_or_else(|| {
            let data_dir = std::env::var("XDG_DATA_HOME")
                .unwrap_or_else(|_| {
                    let home = std::env::var("HOME").unwrap_or_else(|_| "/home".to_string());
                    format!("{home}/.local/share")
                });
            PathBuf::from(format!("{data_dir}/dcpd/audit"))
        })
    }
}

pub async fn run(args: DaemonArgs) -> Result<()> {
    let level = match args.log_level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .init();

    info!("DCP Daemon v{}", dcp_types::PROTOCOL_VERSION);
    info!("Socket: {}", args.socket_path().display());
    info!("Platform: {:?}", platform::current_platform());

    // Load configuration
    let daemon_config = if let Some(config_path) = &args.config {
        config::DaemonConfig::load_from(config_path)?
    } else {
        config::DaemonConfig::load_default()
    };
    info!("Configuration loaded");

    let socket_path = args.socket_path();
    if socket_path.exists() {
        std::fs::remove_file(&socket_path)?;
    }

    // Set up graceful shutdown
    let shutdown = Arc::new(tokio::sync::Notify::new());
    let shutdown_clone = shutdown.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        info!("Shutdown signal received");
        shutdown_clone.notify_waiters();
    });

    let backend = platform::create_backend().await?;
    let automation = platform::create_automation();
    let event_bus = events::EventBus::new();
    let cache = cache::ContextCache::new();
    let audit_logger = audit::AuditLogger::new(args.audit_directory());
    let perm_manager = permissions::PermissionManager::new();
    let plugin_host = Arc::new(plugins::PluginHost::new(args.plugin_directory(), event_bus.clone()));
    let session_manager = server::SessionManager::new(perm_manager.clone());
    let dispatcher = server::Dispatcher::new(
        backend,
        event_bus.clone(),
        cache,
        perm_manager,
        audit_logger,
        session_manager,
        automation,
    );

    // Auto-discover and start plugins
    let plugin_host_clone = plugin_host.clone();
    tokio::spawn(async move {
        let started = plugin_host_clone.auto_start().await;
        if !started.is_empty() {
            info!("Auto-started plugins: {}", started.join(", "));
        }
    });

    // Start plugin health check loop
    let plugin_health = plugin_host.clone();
    tokio::spawn(async move {
        plugin_health.run_health_loop().await;
    });

    // Start Linux-specific services
    if cfg!(target_os = "linux") {
        let dbus_bus = event_bus.clone();
        tokio::spawn(async move {
            if let Err(e) = dbus::run_notification_listener(dbus_bus).await {
                tracing::warn!("D-Bus notification listener error: {e}");
            }
        });

        let watcher_bus = event_bus.clone();
        let watch_paths = if daemon_config.watch_paths.is_empty() {
            vec![
                std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("/home")),
            ]
        } else {
            daemon_config.watch_paths.iter().map(PathBuf::from).collect()
        };
        tokio::spawn(async move {
            if let Err(e) = watcher::run_file_watcher(watcher_bus, watch_paths).await {
                tracing::warn!("File watcher error: {e}");
            }
        });

        let term_bus = event_bus.clone();
        tokio::spawn(async move {
            let capture = terminal::TerminalCapture::new(term_bus);
            if let Err(e) = terminal::auto_detect_terminals(&capture).await {
                tracing::warn!("Terminal detection error: {e}");
            }
        });
    }

    // Start WebSocket server if configured
    if args.remote {
        if let Some(addr_str) = &args.remote_addr {
            if let Ok(addr) = websocket::parse_addr(addr_str) {
                let ws_dispatcher = dispatcher.clone();
                tokio::spawn(async move {
                    let ws_server = websocket::WebSocketServer::new(addr, ws_dispatcher);
                    if let Err(e) = ws_server.run().await {
                        tracing::error!("WebSocket server error: {e}");
                    }
                });
            }
        }
    }

    let server = server::UnixSocketServer::new(socket_path, dispatcher);

    // Run server with graceful shutdown
    tokio::select! {
        result = server.run() => {
            if let Err(e) = result {
                tracing::error!("Server error: {e}");
            }
        }
        _ = shutdown.notified() => {
            info!("Graceful shutdown initiated");
            plugin_host.stop_all().await;
        }
    }

    Ok(())
}
