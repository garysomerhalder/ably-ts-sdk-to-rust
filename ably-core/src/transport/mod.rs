// 🟡 YELLOW Phase: Minimal WebSocket transport implementation
// Integration-First - connects to real Ably WebSocket endpoints!

use crate::auth::AuthMode;
use crate::error::{AblyError, AblyResult};
use crate::protocol::messages::{ProtocolMessage, Action};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message, WebSocketStream, MaybeTlsStream};
use tokio::net::TcpStream;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use serde::{Serialize, Deserialize};
use serde_repr::{Serialize_repr, Deserialize_repr};
use tracing::{debug, info, warn, error};

pub use self::config::TransportConfig;
pub use self::resilience::{ReconnectManager, HeartbeatManager, MessageQueue, ConnectionStats};

mod config;
mod resilience;

/// Transport state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransportState {
    Initialized,
    Connecting,
    Connected,
    Disconnected,
    Closing,
    Closed,
    Failed,
}

/// WebSocket transport for Ably realtime connection
pub struct WebSocketTransport {
    url: String,
    config: TransportConfig,
    auth_mode: AuthMode,
    state: Arc<RwLock<TransportState>>,
    connection_id: Arc<RwLock<Option<String>>>,
    ws_stream: Arc<RwLock<Option<WebSocketStream<MaybeTlsStream<TcpStream>>>>>,
    message_tx: mpsc::UnboundedSender<ProtocolMessage>,
    message_rx: Arc<RwLock<mpsc::UnboundedReceiver<ProtocolMessage>>>,
}

impl WebSocketTransport {
    /// Create new WebSocket transport
    pub fn new(url: &str, config: TransportConfig, auth_mode: AuthMode) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        
        Self {
            url: url.to_string(),
            config,
            auth_mode,
            state: Arc::new(RwLock::new(TransportState::Initialized)),
            connection_id: Arc::new(RwLock::new(None)),
            ws_stream: Arc::new(RwLock::new(None)),
            message_tx: tx,
            message_rx: Arc::new(RwLock::new(rx)),
        }
    }

    /// Connect to Ably WebSocket endpoint
    pub async fn connect(&self) -> AblyResult<()> {
        let mut state = self.state.write().await;
        *state = TransportState::Connecting;
        drop(state);

        // Build WebSocket URL with auth parameters
        let ws_url = self.build_ws_url()?;
        info!("Connecting to WebSocket: {}", ws_url);

        // Connect to WebSocket
        match connect_async(&ws_url).await {
            Ok((ws_stream, _response)) => {
                info!("WebSocket connected successfully");
                
                let mut stream_guard = self.ws_stream.write().await;
                *stream_guard = Some(ws_stream);
                drop(stream_guard);

                let mut state = self.state.write().await;
                *state = TransportState::Connected;
                drop(state);

                // Start message handling loop
                self.start_message_loop().await;

                Ok(())
            }
            Err(e) => {
                error!("Failed to connect WebSocket: {}", e);
                let mut state = self.state.write().await;
                *state = TransportState::Failed;
                Err(AblyError::connection_failed(format!("WebSocket connection failed: {}", e)))
            }
        }
    }

    /// Disconnect from WebSocket
    pub async fn disconnect(&self) -> AblyResult<()> {
        let mut state = self.state.write().await;
        *state = TransportState::Disconnected;
        drop(state);

        let mut ws_guard = self.ws_stream.write().await;
        if let Some(mut ws) = ws_guard.take() {
            let _ = ws.close(None).await;
        }

        Ok(())
    }

    /// Get current transport state
    pub async fn state(&self) -> TransportState {
        *self.state.read().await
    }

    /// Send a protocol message
    pub async fn send_message(&self, message: ProtocolMessage) -> AblyResult<()> {
        let mut ws_guard = self.ws_stream.write().await;
        
        if let Some(ws) = ws_guard.as_mut() {
            let json = serde_json::to_string(&message)
                .map_err(|e| AblyError::parse(format!("Failed to serialize message: {}", e)))?;
            
            ws.send(Message::Text(json)).await
                .map_err(|e| AblyError::network(format!("Failed to send message: {}", e)))?;
            
            Ok(())
        } else {
            Err(AblyError::connection_failed("WebSocket not connected"))
        }
    }

    /// Receive next protocol message
    pub async fn receive_message(&self) -> AblyResult<ProtocolMessage> {
        let mut rx = self.message_rx.write().await;
        
        if let Some(msg) = rx.recv().await {
            Ok(msg)
        } else {
            Err(AblyError::connection_failed("Message channel closed"))
        }
    }

    /// Check if using binary protocol
    pub fn is_binary_protocol(&self) -> bool {
        self.config.use_binary_protocol
    }

    /// Build WebSocket URL with authentication
    fn build_ws_url(&self) -> AblyResult<String> {
        let mut url = self.url.clone();
        
        // Add protocol version
        url.push_str("?v=3");
        
        // Add authentication
        match &self.auth_mode {
            AuthMode::ApiKey(key) => {
                url.push_str("&key=");
                url.push_str(key);
            }
            AuthMode::Token(token) => {
                url.push_str("&access_token=");
                url.push_str(token);
            }
        }
        
        // Add format
        if self.config.use_binary_protocol {
            url.push_str("&format=msgpack");
        } else {
            url.push_str("&format=json");
        }
        
        Ok(url)
    }

    /// Start message handling loop
    async fn start_message_loop(&self) {
        let ws_stream = Arc::clone(&self.ws_stream);
        let state = Arc::clone(&self.state);
        let connection_id = Arc::clone(&self.connection_id);
        let tx = self.message_tx.clone();

        tokio::spawn(async move {
            loop {
                let mut ws_guard = ws_stream.write().await;
                
                if let Some(ws) = ws_guard.as_mut() {
                    match ws.next().await {
                        Some(Ok(Message::Text(text))) => {
                            debug!("Received text message: {}", text);
                            
                            if let Ok(msg) = serde_json::from_str::<ProtocolMessage>(&text) {
                                // Handle CONNECTED message
                                if msg.action == Action::Connected {
                                    if let Some(id) = &msg.connection_id {
                                        let mut conn_id = connection_id.write().await;
                                        *conn_id = Some(id.clone());
                                    }
                                }
                                
                                let _ = tx.send(msg);
                            }
                        }
                        Some(Ok(Message::Binary(data))) => {
                            debug!("Received binary message ({} bytes)", data.len());
                            // TODO: Implement MessagePack decoding
                        }
                        Some(Ok(Message::Close(_))) => {
                            info!("WebSocket closed by server");
                            let mut state_guard = state.write().await;
                            *state_guard = TransportState::Closed;
                            break;
                        }
                        Some(Err(e)) => {
                            error!("WebSocket error: {}", e);
                            let mut state_guard = state.write().await;
                            *state_guard = TransportState::Failed;
                            break;
                        }
                        None => {
                            warn!("WebSocket stream ended");
                            break;
                        }
                        _ => {}
                    }
                } else {
                    break;
                }
            }
        });
    }
}