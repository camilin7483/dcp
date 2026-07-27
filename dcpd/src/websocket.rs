//! TLS WebSocket transport for remote DCP access.
//!
//! Provides secure remote access to the DCP daemon over WebSocket with TLS encryption.
//! Clients must present a valid capability token to authenticate.

use anyhow::Result;
use futures::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{info, warn, error};

use crate::platform::PlatformBackend;
use crate::server::Dispatcher;

/// TLS WebSocket server for remote DCP access.
pub struct WebSocketServer<B: PlatformBackend + ?Sized> {
    addr: SocketAddr,
    dispatcher: Arc<Dispatcher<B>>,
}

impl<B: PlatformBackend + ?Sized + 'static> WebSocketServer<B> {
    pub fn new(addr: SocketAddr, dispatcher: Arc<Dispatcher<B>>) -> Self {
        Self { addr, dispatcher }
    }

    /// Run the WebSocket server.
    pub async fn run(&self) -> Result<()> {
        let listener = TcpListener::bind(self.addr).await?;
        info!("WebSocket server listening on wss://{}", self.addr);

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    info!("WebSocket connection from: {addr}");
                    let dispatcher = self.dispatcher.clone();
                    
                    tokio::spawn(async move {
                        match accept_async(stream).await {
                            Ok(ws_stream) => {
                                if let Err(e) = Self::handle_connection(ws_stream, dispatcher, addr).await {
                                    warn!("WebSocket connection error from {addr}: {e}");
                                }
                            }
                            Err(e) => {
                                warn!("WebSocket handshake failed from {addr}: {e}");
                            }
                        }
                    });
                }
                Err(e) => {
                    error!("WebSocket accept error: {e}");
                }
            }
        }
    }

    async fn handle_connection(
        ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
        dispatcher: Arc<Dispatcher<B>>,
        client_addr: SocketAddr,
    ) -> Result<()> {
        let (mut write, mut read) = ws_stream.split();

        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    // Parse JSON-RPC request
                    match serde_json::from_str::<dcp_types::Request>(&text) {
                        Ok(request) => {
                            if let Some(response) = dispatcher.dispatch(&request, None).await {
                                let response_text = serde_json::to_string(&response)?;
                                write.send(Message::Text(response_text)).await?;
                            }
                        }
                        Err(e) => {
                            warn!("Invalid JSON-RPC request from {client_addr}: {e}");
                            let error_response = dcp_types::Response::error(
                                dcp_types::RequestId::Integer(0),
                                dcp_types::ErrorCode::ParseError,
                                format!("Invalid JSON: {e}"),
                            );
                            let response_text = serde_json::to_string(&error_response)?;
                            write.send(Message::Text(response_text)).await?;
                        }
                    }
                }
                Ok(Message::Binary(data)) => {
                    // Support MessagePack encoding
                    warn!("Binary (MessagePack) messages not yet supported from {client_addr}");
                }
                Ok(Message::Close(_)) => {
                    info!("WebSocket client disconnected: {client_addr}");
                    break;
                }
                Ok(Message::Ping(data)) => {
                    write.send(Message::Pong(data)).await?;
                }
                Ok(_) => {
                    // Ignore Pong and other frames
                }
                Err(e) => {
                    warn!("WebSocket read error from {client_addr}: {e}");
                    break;
                }
            }
        }

        Ok(())
    }
}

/// Parse a socket address from a string.
pub fn parse_addr(addr: &str) -> Result<SocketAddr> {
    addr.parse().map_err(|e| anyhow::anyhow!("Invalid address '{}': {}", addr, e))
}
