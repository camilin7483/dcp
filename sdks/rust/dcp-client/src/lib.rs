//! DCP Client — Rust client library for the Desktop Context Protocol daemon.
//!
//! # Example
//!
//! ```no_run
//! use dcp_client::Client;
//! use dcp_types::ContextSelector;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let mut client = Client::connect("/tmp/dcpd.sock").await?;
//!     let session = client.create_session(&["dcp:context:windows:read"]).await?;
//!
//!     let ctx = client.query(&[ContextSelector::ActiveWindow]).await?;
//!     println!("Active window: {:?}", ctx.active_window);
//!
//!     Ok(())
//! }
//! ```

use anyhow::{Context, Result};
use dcp_types::*;
use futures::{SinkExt, StreamExt};
use std::path::Path;
use tokio::net::UnixStream;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

/// Error returned by the DCP client.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("not connected to daemon")]
    NotConnected,
    #[error("RPC error: {0}")]
    Rpc(String),
    #[error("connection closed")]
    ConnectionClosed,
}

/// A client for communicating with the DCP daemon over Unix socket.
pub struct Client {
    framed: Option<Framed<UnixStream, LengthDelimitedCodec>>,
    request_id: u64,
    session_id: Option<String>,
    token: Option<String>,
}

impl Client {
    /// Connect to the DCP daemon at the given socket path.
    pub async fn connect(path: impl AsRef<Path>) -> Result<Self> {
        let stream = UnixStream::connect(path.as_ref())
            .await
            .context("Failed to connect to dcpd. Is the daemon running?")?;

        let codec = LengthDelimitedCodec::builder()
            .length_field_length(4)
            .max_frame_length(16 * 1024 * 1024)
            .new_codec();

        Ok(Self {
            framed: Some(Framed::new(stream, codec)),
            request_id: 0,
            session_id: None,
            token: None,
        })
    }

    /// Check if the client is connected.
    pub fn is_connected(&self) -> bool {
        self.framed.is_some()
    }

    /// Create a session with requested capabilities.
    pub async fn create_session(&mut self, capabilities: &[&str]) -> Result<SessionCreateResult> {
        let caps: Vec<Capability> = capabilities
            .iter()
            .filter_map(|s| Capability::from_str(s))
            .collect();

        let params = SessionCreateParams {
            client_name: Some("dcp-client-rs".to_string()),
            capabilities: caps,
            encoding: None,
        };

        let result: SessionCreateResult = self.call("session.create", params).await?;
        self.session_id = Some(result.session_id.clone());
        self.token = Some(result.token.clone());
        Ok(result)
    }

    /// Query desktop context with the given selectors.
    pub async fn query(&mut self, selectors: &[ContextSelector]) -> Result<ContextSnapshot> {
        let params = ContextGetParams {
            selectors: selectors.to_vec(),
        };
        self.call("context.get", params).await
    }

    /// Execute an automation command.
    pub async fn execute(
        &mut self,
        command: AutomationCommand,
        dry_run: bool,
    ) -> Result<AutomationResult> {
        let params = AutomationExecuteParams { command, dry_run };
        self.call("automation.execute", params).await
    }

    /// Capture screen/window/region.
    pub async fn capture(&mut self, params: VisionCaptureParams) -> Result<VisionCaptureResult> {
        self.call("vision.capture", params).await
    }

    /// Perform OCR on a base64-encoded image.
    pub async fn ocr(&mut self, params: VisionOcrParams) -> Result<VisionOcrResult> {
        self.call("vision.ocr", params).await
    }

    /// Get daemon status.
    pub async fn status(&mut self) -> Result<serde_json::Value> {
        self.call("daemon.status", serde_json::json!({})).await
    }

    /// Subscribe to events.
    pub async fn subscribe(
        &mut self,
        events: Vec<EventType>,
        batch: bool,
        batch_interval_ms: Option<u64>,
    ) -> Result<String> {
        let params = EventsSubscribeParams {
            events,
            batch,
            batch_interval_ms,
        };
        let result: serde_json::Value = self.call("events.subscribe", params).await?;
        Ok(result["subscriptionId"].as_str().unwrap_or("").to_string())
    }

    /// Close the current session and connection.
    pub async fn close(&mut self) -> Result<()> {
        if let Some(session_id) = self.session_id.take() {
            let _ = self
                .call::<serde_json::Value, _>(
                    "session.close",
                    serde_json::json!({"sessionId": session_id}),
                )
                .await;
        }
        self.framed = None;
        Ok(())
    }

    /// Send a JSON-RPC request and wait for the response.
    async fn call<T: serde::de::DeserializeOwned, P: serde::Serialize>(
        &mut self,
        method: &str,
        params: P,
    ) -> Result<T> {
        let framed = self.framed.as_mut().ok_or(ClientError::NotConnected)?;

        self.request_id += 1;
        let request = Request::new(self.request_id as i64, method, params);
        let bytes = serde_json::to_vec(&request)?;

        framed
            .send(bytes.into())
            .await
            .context("Failed to send request")?;

        let response_bytes = framed
            .next()
            .await
            .context("Connection closed waiting for response")??;

        let response: Response = serde_json::from_slice(&response_bytes)?;

        if let Some(error) = response.error {
            anyhow::bail!("RPC error: {} (code {})", error.message, error.code);
        }

        let result = response
            .result
            .ok_or_else(|| anyhow::anyhow!("No result in response"))?;

        Ok(serde_json::from_value(result)?)
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        if self.framed.is_some() {
            let _ = self.framed.take();
        }
    }
}
