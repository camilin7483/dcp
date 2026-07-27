//! DCP CLI — command-line interface for Desktop Context Protocol.

use anyhow::{Context as AnyhowContext, Result};
use clap::{Parser, Subcommand};
use dcp_types::*;
use serde_json::Value;
use std::path::PathBuf;

use futures::SinkExt;
use futures::StreamExt;
use tokio::net::UnixStream;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

#[derive(Parser)]
#[command(name = "dcp", version, about = "Desktop Context Protocol CLI")]
struct Cli {
    /// Socket path (default: $XDG_RUNTIME_DIR/dcpd.sock)
    #[arg(long, global = true)]
    socket: Option<PathBuf>,

    /// Output format
    #[arg(long, global = true, default_value = "pretty")]
    format: OutputFormat,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Query desktop context
    Query {
        /// Context selectors (comma-separated: activeWindow,clipboard,processes)
        #[arg(value_delimiter = ',')]
        selectors: Vec<String>,
    },
    /// Subscribe to events in real-time
    Subscribe {
        /// Event types to subscribe to
        #[arg(value_delimiter = ',')]
        events: Vec<String>,
    },
    /// Show daemon status
    Status,
    /// Create a session
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },
    /// Dump full desktop context
    Inspect,
    /// Benchmark context.get latency
    Benchmark {
        /// Number of iterations
        #[arg(default_value = "100")]
        iterations: u32,
    },
}

#[derive(Subcommand)]
enum SessionAction {
    /// Create a new session
    Create {
        /// Client name
        #[arg(long)]
        name: Option<String>,
    },
    /// Close a session
    Close {
        /// Session ID
        session_id: String,
    },
    /// List active sessions
    List,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum OutputFormat {
    Json,
    Pretty,
    Table,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let socket_path = cli.socket.unwrap_or_else(|| {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(format!("{runtime_dir}/dcpd.sock"))
    });

    match cli.command {
        Command::Query { selectors } => cmd_query(&socket_path, selectors, cli.format).await,
        Command::Subscribe { events } => cmd_subscribe(&socket_path, events).await,
        Command::Status => cmd_status(&socket_path, cli.format).await,
        Command::Session { action } => cmd_session(&socket_path, action, cli.format).await,
        Command::Inspect => cmd_inspect(&socket_path, cli.format).await,
        Command::Benchmark { iterations } => cmd_benchmark(&socket_path, iterations).await,
    }
}

/// Client for communicating with dcpd.
struct DcpClient {
    framed: Framed<UnixStream, LengthDelimitedCodec>,
    session_id: Option<String>,
}

impl DcpClient {
    async fn connect(socket_path: &PathBuf) -> Result<Self> {
        let stream = UnixStream::connect(socket_path)
            .await
            .context("Failed to connect to dcpd. Is the daemon running?")?;

        let codec = LengthDelimitedCodec::builder()
            .length_field_length(4)
            .max_frame_length(16 * 1024 * 1024)
            .new_codec();

        let mut client = Self {
            framed: Framed::new(stream, codec),
            session_id: None,
        };

        // Auto-create session with default capabilities
        client.create_default_session().await?;

        Ok(client)
    }

    async fn create_default_session(&mut self) -> Result<()> {
        let request = Request::new(
            1,
            "session.create",
            SessionCreateParams {
                client_name: Some("dcp-cli".to_string()),
                capabilities: Capability::default_local(),
                encoding: None,
            },
        );

        let bytes = serde_json::to_vec(&request)?;
        self.framed.send(bytes.into()).await?;

        let response_bytes = self.framed.next().await.context("Connection closed")??;
        let response: Response = serde_json::from_slice(&response_bytes)?;

        if let Some(error) = response.error {
            anyhow::bail!("Session creation failed: {} (code {})", error.message, error.code);
        }

        if let Some(result) = response.result {
            if let Some(sid) = result.get("sessionId").and_then(|v| v.as_str()) {
                self.session_id = Some(sid.to_string());
            }
        }

        Ok(())
    }

    async fn send(&mut self, request: &Request) -> Result<Option<Response>> {
        let bytes = serde_json::to_vec(request)?;
        self.framed.send(bytes.into()).await?;

        if request.is_notification() {
            return Ok(None);
        }

        let response_bytes = self.framed.next().await.context("Connection closed")??;
        let response: Response = serde_json::from_slice(&response_bytes)?;
        Ok(Some(response))
    }

    async fn receive_event(&mut self) -> Result<Response> {
        let bytes = self.framed.next().await.context("Connection closed")??;
        let response: Response = serde_json::from_slice(&bytes)?;
        Ok(response)
    }
}

async fn cmd_query(
    socket_path: &PathBuf,
    selectors: Vec<String>,
    format: OutputFormat,
) -> Result<()> {
    let mut client = DcpClient::connect(socket_path).await?;

    let context_selectors: Vec<ContextSelector> =
        selectors.iter().filter_map(|s| parse_selector(s)).collect();

    let selectors_to_use = if context_selectors.is_empty() {
        vec![ContextSelector::ActiveWindow]
    } else {
        context_selectors
    };

    let request = Request::new(
        1,
        "context.get",
        ContextGetParams {
            selectors: selectors_to_use,
        },
    );

    if let Some(response) = client.send(&request).await? {
        if let Some(err) = response.error {
            eprintln!("Error: {}", err.message);
            std::process::exit(1);
        }
        print_value(response.result.as_ref(), format);
    }

    Ok(())
}

async fn cmd_subscribe(socket_path: &PathBuf, events: Vec<String>) -> Result<()> {
    let mut client = DcpClient::connect(socket_path).await?;

    let event_types: Vec<EventType> = events.iter().filter_map(|s| parse_event_type(s)).collect();

    let request = Request::new(
        1,
        "events.subscribe",
        EventsSubscribeParams {
            events: event_types,
            batch: false,
            batch_interval_ms: None,
        },
    );

    if let Some(response) = client.send(&request).await? {
        if let Some(err) = response.error {
            eprintln!("Error: {}", err.message);
            std::process::exit(1);
        }
        let sub_id = response
            .result
            .and_then(|r| {
                r.get("subscriptionId")
                    .and_then(|v| v.as_str().map(String::from))
            })
            .unwrap_or_default();
        println!("Subscribed: {sub_id}");
        println!("Listening for events... (Ctrl+C to exit)");
    }

    loop {
        match client.framed.next().await {
            Some(Ok(bytes)) => {
                if let Ok(value) = serde_json::from_slice::<Value>(&bytes) {
                    println!("{}", serde_json::to_string_pretty(&value)?);
                }
            }
            Some(Err(e)) => {
                eprintln!("Connection error: {e}");
                break;
            }
            None => {
                println!("Connection closed by server");
                break;
            }
        }
    }

    Ok(())
}

async fn cmd_status(socket_path: &PathBuf, format: OutputFormat) -> Result<()> {
    let mut client = DcpClient::connect(socket_path).await?;

    let request = Request::new(1, "daemon.status", serde_json::json!({}));

    if let Some(response) = client.send(&request).await? {
        if let Some(err) = response.error {
            eprintln!("Error: {}", err.message);
            std::process::exit(1);
        }
        print_value(response.result.as_ref(), format);
    }

    Ok(())
}

async fn cmd_session(
    socket_path: &PathBuf,
    action: SessionAction,
    format: OutputFormat,
) -> Result<()> {
    let mut client = DcpClient::connect(socket_path).await?;

    match action {
        SessionAction::Create { name } => {
            let request = Request::new(
                1,
                "session.create",
                SessionCreateParams {
                    client_name: name,
                    capabilities: Capability::default_local(),
                    encoding: None,
                },
            );

            if let Some(response) = client.send(&request).await? {
                print_value(response.result.as_ref(), format);
            }
        }
        SessionAction::Close { session_id } => {
            let request = Request::new(
                1,
                "session.close",
                serde_json::json!({"sessionId": session_id}),
            );
            if let Some(response) = client.send(&request).await? {
                print_value(response.result.as_ref(), format);
            }
        }
        SessionAction::List => {
            let request = Request::new(1, "daemon.status", serde_json::json!({}));
            if let Some(response) = client.send(&request).await? {
                print_value(response.result.as_ref(), format);
            }
        }
    }

    Ok(())
}

async fn cmd_inspect(socket_path: &PathBuf, format: OutputFormat) -> Result<()> {
    let mut client = DcpClient::connect(socket_path).await?;

    let all_selectors = vec![
        ContextSelector::ActiveWindow,
        ContextSelector::WindowTree,
        ContextSelector::RunningProcesses,
        ContextSelector::Clipboard,
        ContextSelector::Mouse,
        ContextSelector::Monitors,
        ContextSelector::SystemResources,
        ContextSelector::Network,
        ContextSelector::AudioDevices,
        ContextSelector::Power,
        ContextSelector::Workspace,
        ContextSelector::Notifications,
    ];

    let request = Request::new(
        1,
        "context.get",
        ContextGetParams {
            selectors: all_selectors,
        },
    );

    if let Some(response) = client.send(&request).await? {
        if let Some(err) = response.error {
            eprintln!("Error: {}", err.message);
            std::process::exit(1);
        }
        print_value(response.result.as_ref(), format);
    }

    Ok(())
}

async fn cmd_benchmark(socket_path: &PathBuf, iterations: u32) -> Result<()> {
    let mut client = DcpClient::connect(socket_path).await?;

    let request = Request::new(
        1,
        "context.get",
        ContextGetParams {
            selectors: vec![ContextSelector::ActiveWindow],
        },
    );

    let mut latencies = Vec::with_capacity(iterations as usize);

    for _ in 0..iterations {
        let start = std::time::Instant::now();
        client.send(&request).await?;
        let elapsed = start.elapsed();
        latencies.push(elapsed.as_micros() as f64);
    }

    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let avg = latencies.iter().sum::<f64>() / latencies.len() as f64;
    let p50 = latencies[latencies.len() / 2];
    let p95 = latencies[(latencies.len() as f64 * 0.95) as usize];
    let p99 = latencies[(latencies.len() as f64 * 0.99) as usize];
    let min = latencies[0];
    let max = *latencies.last().unwrap();

    println!("Benchmark results ({iterations} iterations):");
    println!("  avg:  {avg:.1} µs");
    println!("  p50:  {p50:.1} µs");
    println!("  p95:  {p95:.1} µs");
    println!("  p99:  {p99:.1} µs");
    println!("  min:  {min:.1} µs");
    println!("  max:  {max:.1} µs");

    Ok(())
}

fn print_value(value: Option<&Value>, format: OutputFormat) {
    let value = value.unwrap_or(&Value::Null);
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string(value).unwrap_or_default());
        }
        OutputFormat::Pretty => {
            println!(
                "{}",
                serde_json::to_string_pretty(value).unwrap_or_default()
            );
        }
        OutputFormat::Table => {
            if let Some(obj) = value.as_object() {
                for (key, val) in obj {
                    match val {
                        Value::String(s) => println!("{key}: {s}"),
                        Value::Number(n) => println!("{key}: {n}"),
                        Value::Bool(b) => println!("{key}: {b}"),
                        Value::Null => println!("{key}: null"),
                        _ => println!("{key}: {}", serde_json::to_string(val).unwrap_or_default()),
                    }
                }
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(value).unwrap_or_default()
                );
            }
        }
    }
}

fn parse_selector(s: &str) -> Option<ContextSelector> {
    match s.to_lowercase().as_str() {
        "activewindow" | "active-window" => Some(ContextSelector::ActiveWindow),
        "windowtree" | "window-tree" | "windows" => Some(ContextSelector::WindowTree),
        "activeapplication" | "active-app" | "app" => Some(ContextSelector::ActiveApplication),
        "runningprocesses" | "processes" | "procs" => Some(ContextSelector::RunningProcesses),
        "clipboard" | "clip" => Some(ContextSelector::Clipboard),
        "mouse" | "cursor" => Some(ContextSelector::Mouse),
        "keyboardfocus" | "focus" => Some(ContextSelector::KeyboardFocus),
        "monitors" | "displays" => Some(ContextSelector::Monitors),
        "systemresources" | "resources" | "sys" => Some(ContextSelector::SystemResources),
        "network" | "net" => Some(ContextSelector::Network),
        "audiodevices" | "audio" => Some(ContextSelector::AudioDevices),
        "notifications" | "notifs" => Some(ContextSelector::Notifications),
        "power" | "battery" => Some(ContextSelector::Power),
        "workspace" | "desktop" => Some(ContextSelector::Workspace),
        "installedapps" | "apps" => Some(ContextSelector::InstalledApps),
        "terminals" | "terminal" => Some(ContextSelector::Terminals),
        "browser" | "tabs" => Some(ContextSelector::Browser),
        "openfiles" | "files" => Some(ContextSelector::OpenFiles),
        "selectedtext" | "selection" => Some(ContextSelector::SelectedText),
        _ => None,
    }
}

fn parse_event_type(s: &str) -> Option<EventType> {
    match s.to_lowercase().as_str() {
        "window.focus" | "windowfocus" => Some(EventType::WindowFocusChanged),
        "window.opened" | "windowopened" => Some(EventType::WindowOpened),
        "window.closed" | "windowclosed" => Some(EventType::WindowClosed),
        "window.title" | "windowtitle" => Some(EventType::WindowTitleChanged),
        "app.launched" | "applaunch" => Some(EventType::ApplicationLaunched),
        "app.terminated" | "appterminated" => Some(EventType::ApplicationTerminated),
        "clipboard" | "clip" => Some(EventType::ClipboardChanged),
        "selection" => Some(EventType::SelectionChanged),
        "file.changed" | "filechanged" => Some(EventType::FileChanged),
        "terminal.exec" | "terminalexec" => Some(EventType::TerminalCommandExecuted),
        "terminal.output" | "terminaloutput" => Some(EventType::TerminalOutputReceived),
        "browser.tab" | "browsertab" => Some(EventType::BrowserTabActivated),
        "browser.url" | "browserurl" => Some(EventType::BrowserUrlChanged),
        "notification" => Some(EventType::NotificationReceived),
        "monitor.connected" | "monitorconnected" => Some(EventType::MonitorConnected),
        "audio" => Some(EventType::AudioDeviceAdded),
        "network" => Some(EventType::NetworkConnectivityChanged),
        "power" => Some(EventType::PowerStateChanged),
        "screen.locked" => Some(EventType::ScreenLocked),
        _ => None,
    }
}
