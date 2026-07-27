//! TLS WebSocket transport for remote DCP access.
//!
//! Provides secure remote access to the DCP daemon over WebSocket with TLS encryption.
//! Clients must present a valid capability token to authenticate.

use anyhow::Result;
use futures::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{info, warn, error};

use crate::platform::PlatformBackend;
use crate::server::{Dispatcher, Session};

/// TLS WebSocket server for remote DCP access.
pub struct WebSocketServer<B: PlatformBackend + ?Sized> {
    addr: SocketAddr,
    dispatcher: Arc<Dispatcher<B>>,
    tls_config: Option<Arc<rustls::ServerConfig>>,
}

impl<B: PlatformBackend + ?Sized + 'static> WebSocketServer<B> {
    pub fn new(
        addr: SocketAddr,
        dispatcher: Arc<Dispatcher<B>>,
        tls_config: Option<Arc<rustls::ServerConfig>>,
    ) -> Self {
        Self { addr, dispatcher, tls_config }
    }

    /// Run the WebSocket server.
    pub async fn run(&self) -> Result<()> {
        let listener = TcpListener::bind(self.addr).await?;
        let scheme = if self.tls_config.is_some() { "wss" } else { "ws" };
        info!("WebSocket server listening on {scheme}://{}", self.addr);

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    info!("WebSocket connection from: {addr}");
                    let dispatcher = self.dispatcher.clone();
                    let tls_config = self.tls_config.clone();

                    tokio::spawn(async move {
                        if let Some(tls_config) = tls_config {
                            let acceptor = TlsAcceptor::from(tls_config);
                            match acceptor.accept(stream).await {
                                Ok(tls_stream) => {
                                    if let Err(e) = Self::handle_ws_handshake(tls_stream, dispatcher, addr).await {
                                        warn!("WebSocket connection error from {addr}: {e}");
                                    }
                                }
                                Err(e) => {
                                    warn!("TLS handshake failed from {addr}: {e}");
                                }
                            }
                        } else if let Err(e) = Self::handle_ws_handshake(stream, dispatcher, addr).await {
                            warn!("WebSocket connection error from {addr}: {e}");
                        }
                    });
                }
                Err(e) => {
                    error!("WebSocket accept error: {e}");
                }
            }
        }
    }

    async fn handle_ws_handshake<S>(
        stream: S,
        dispatcher: Arc<Dispatcher<B>>,
        client_addr: SocketAddr,
    ) -> Result<()>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        match accept_async(stream).await {
            Ok(ws_stream) => Self::handle_connection(ws_stream, dispatcher, client_addr).await,
            Err(e) => {
                warn!("WebSocket handshake failed from {client_addr}: {e}");
                Ok(())
            }
        }
    }

    async fn handle_connection<S>(
        ws_stream: tokio_tungstenite::WebSocketStream<S>,
        dispatcher: Arc<Dispatcher<B>>,
        client_addr: SocketAddr,
    ) -> Result<()>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        let (mut write, mut read) = ws_stream.split();
        let mut session: Option<Session> = None;

        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    match serde_json::from_str::<dcp_types::Request>(&text) {
                        Ok(request) => {
                            let is_close = request.method == "session.close";
                            let is_create = request.method == "session.create";

                            let (response, created_session) =
                                dispatcher.dispatch(&request, session.as_ref()).await;

                            if let Some(s) = created_session {
                                session = Some(s);
                            } else if is_create {
                                let session_id = response
                                    .as_ref()
                                    .and_then(|r| r.result.as_ref())
                                    .and_then(|v| v.get("sessionId"))
                                    .and_then(|v| v.as_str());
                                if let Some(sid) = session_id {
                                    if let Some(s) = dispatcher.session_manager.get_session(sid).await
                                    {
                                        session = Some(s);
                                    }
                                }
                            }

                            if is_close {
                                session = None;
                            }

                            if let Some(response) = response {
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
                Ok(Message::Binary(_)) => {
                    warn!("Binary (MessagePack) messages not yet supported from {client_addr}");
                }
                Ok(Message::Close(_)) => {
                    info!("WebSocket client disconnected: {client_addr}");
                    break;
                }
                Ok(Message::Ping(data)) => {
                    write.send(Message::Pong(data)).await?;
                }
                Ok(_) => {}
                Err(e) => {
                    warn!("WebSocket read error from {client_addr}: {e}");
                    break;
                }
            }
        }

        Ok(())
    }
}

/// Load TLS configuration from PEM certificate and key files.
pub fn load_tls_config(cert_path: &Path, key_path: &Path) -> Result<Arc<rustls::ServerConfig>> {
    use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs1KeyDer, PrivatePkcs8KeyDer};

    let certs = {
        let file = std::fs::File::open(cert_path)
            .map_err(|e| anyhow::anyhow!("Failed to open certificate '{}': {}", cert_path.display(), e))?;
        let mut reader = std::io::BufReader::new(file);
        rustls_pemfile::certs(&mut reader)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| anyhow::anyhow!("Failed to parse certificates from '{}': {}", cert_path.display(), e))?
    };

    let key_data = std::fs::read(key_path)
        .map_err(|e| anyhow::anyhow!("Failed to read key file '{}': {}", key_path.display(), e))?;

    let key_der = {
        let mut reader = std::io::BufReader::new(&*key_data);
        if let Some(key) = rustls_pemfile::pkcs8_private_keys(&mut reader)
            .next()
            .transpose()
            .map_err(|e| anyhow::anyhow!("Failed to parse PKCS#8 key from '{}': {}", key_path.display(), e))?
        {
            PrivateKeyDer::from(PrivatePkcs8KeyDer::from(key))
        } else {
            let mut reader = std::io::BufReader::new(&*key_data);
            if let Some(key) = rustls_pemfile::rsa_private_keys(&mut reader)
                .next()
                .transpose()
                .map_err(|e| anyhow::anyhow!("Failed to parse RSA key from '{}': {}", key_path.display(), e))?
            {
                PrivateKeyDer::from(PrivatePkcs1KeyDer::from(key))
            } else {
                anyhow::bail!("No private key found in '{}'", key_path.display())
            }
        }
    };

    let cert_chain: Vec<CertificateDer<'static>> = certs
        .into_iter()
        .map(CertificateDer::from)
        .collect();

    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key_der)?;

    Ok(Arc::new(config))
}

/// Parse a socket address from a string.
pub fn parse_addr(addr: &str) -> Result<SocketAddr> {
    addr.parse().map_err(|e| anyhow::anyhow!("Invalid address '{}': {}", addr, e))
}
