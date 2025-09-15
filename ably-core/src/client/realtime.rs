// 🟡 YELLOW Phase: Minimal Realtime client implementation
// WebSocket-based real-time client

use crate::auth::AuthMode;
use crate::connection::state_machine::{ConnectionStateMachine, ConnectionState, ConnectionEvent};
use crate::error::{AblyError, AblyResult};
use crate::protocol::messages::{ProtocolMessage, Action, Message, PresenceMessage, ErrorInfo, PresenceAction};
use crate::transport::{WebSocketTransport, TransportConfig};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, oneshot};
use tracing::{debug, info, warn, error};

/// Realtime client for WebSocket connections
pub struct RealtimeClient {
    transport: Arc<WebSocketTransport>,
    state_machine: Arc<ConnectionStateMachine>,
    channels: Arc<RwLock<HashMap<String, RealtimeChannel>>>,
    message_tx: mpsc::Sender<ProtocolMessage>,
    message_rx: Arc<RwLock<mpsc::Receiver<ProtocolMessage>>>,
}

impl RealtimeClient {
    /// Create a new realtime client with API key
    pub async fn new(api_key: impl Into<String>) -> AblyResult<Self> {
        let config = TransportConfig::default();
        let auth = AuthMode::ApiKey(api_key.into());
        let url = "wss://realtime.ably.io";
        let transport = WebSocketTransport::new(url, config, auth);
        
        let state_machine = Arc::new(ConnectionStateMachine::new());
        let channels = Arc::new(RwLock::new(HashMap::new()));
        let (message_tx, message_rx) = mpsc::channel(100);
        
        let client = Self {
            transport: Arc::new(transport),
            state_machine,
            channels,
            message_tx,
            message_rx: Arc::new(RwLock::new(message_rx)),
        };
        
        Ok(client)
    }
    
    /// Connect to Ably realtime
    pub async fn connect(&self) -> AblyResult<()> {
        info!("Connecting to Ably realtime...");
        
        // Update state machine
        self.state_machine.send_event(ConnectionEvent::Connect).await?;
        
        // Connect transport
        self.transport.connect().await?;
        
        // Start message processing loop
        self.start_message_processor();
        
        // Update state
        self.state_machine.send_event(ConnectionEvent::Connected("temp-id".to_string())).await?;
        
        info!("Connected to Ably realtime");
        Ok(())
    }
    
    /// Disconnect from Ably
    pub async fn disconnect(&self) -> AblyResult<()> {
        info!("Disconnecting from Ably realtime...");
        
        self.state_machine.send_event(ConnectionEvent::Disconnect).await?;
        self.transport.disconnect().await?;
        self.state_machine.send_event(ConnectionEvent::Disconnected(None)).await?;
        
        info!("Disconnected from Ably realtime");
        Ok(())
    }
    
    /// Get or create a channel
    pub async fn channel(&self, name: impl Into<String>) -> RealtimeChannel {
        let name = name.into();
        let mut channels = self.channels.write().await;
        
        channels.entry(name.clone())
            .or_insert_with(|| {
                RealtimeChannel::new(
                    name.clone(),
                    self.transport.clone(),
                    self.state_machine.clone(),
                )
            })
            .clone()
    }
    
    /// Get connection state
    pub async fn state(&self) -> ConnectionState {
        self.state_machine.state().await
    }
    
    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        matches!(self.state().await, ConnectionState::Connected)
    }
    
    /// Start background message processor
    fn start_message_processor(&self) {
        let transport = self.transport.clone();
        let state_machine = self.state_machine.clone();
        let channels = self.channels.clone();
        
        tokio::spawn(async move {
            loop {
                // Receive messages from transport
                match transport.receive_message().await {
                    Ok(message) => {
                        debug!("Received message: {:?}", message.action);
                        
                        // Process message based on action
                        match message.action {
                            Action::Connected => {
                                let _ = state_machine.send_event(ConnectionEvent::Connected("temp".to_string())).await;
                            }
                            Action::Disconnected => {
                                let _ = state_machine.send_event(ConnectionEvent::Disconnected(None)).await;
                            }
                            Action::Error => {
                                if let Some(error) = message.error {
                                    error!("Protocol error: {:?}", error);
                                    let _ = state_machine.send_event(ConnectionEvent::Error(error.clone())).await;
                                }
                            }
                            Action::Message => {
                                // Route to appropriate channel
                                if let Some(channel_name) = &message.channel {
                                    let channels = channels.read().await;
                                    if let Some(channel) = channels.get(channel_name) {
                                        channel.handle_message(message).await;
                                    }
                                }
                            }
                            Action::Attached => {
                                if let Some(channel_name) = &message.channel {
                                    let channels = channels.read().await;
                                    if let Some(channel) = channels.get(channel_name) {
                                        channel.handle_attached().await;
                                    }
                                }
                            }
                            Action::Detached => {
                                if let Some(channel_name) = &message.channel {
                                    let channels = channels.read().await;
                                    if let Some(channel) = channels.get(channel_name) {
                                        channel.handle_detached().await;
                                    }
                                }
                            }
                            _ => {
                                debug!("Unhandled message action: {:?}", message.action);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error receiving message: {}", e);
                        let _ = state_machine.send_event(ConnectionEvent::Error(ErrorInfo::default())).await;
                    }
                }
                
                // Small delay to prevent busy loop
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        });
    }
}

/// Realtime channel for pub/sub
#[derive(Clone)]
pub struct RealtimeChannel {
    name: String,
    transport: Arc<WebSocketTransport>,
    state_machine: Arc<ConnectionStateMachine>,
    message_handlers: Arc<RwLock<Vec<MessageHandler>>>,
    presence_handlers: Arc<RwLock<Vec<PresenceHandler>>>,
}

type MessageHandler = Arc<dyn Fn(Message) + Send + Sync>;
type PresenceHandler = Arc<dyn Fn(PresenceMessage) + Send + Sync>;

impl RealtimeChannel {
    fn new(
        name: String,
        transport: Arc<WebSocketTransport>,
        state_machine: Arc<ConnectionStateMachine>,
    ) -> Self {
        Self {
            name,
            transport,
            state_machine,
            message_handlers: Arc::new(RwLock::new(Vec::new())),
            presence_handlers: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Attach to the channel
    pub async fn attach(&self) -> AblyResult<()> {
        info!("Attaching to channel: {}", self.name);
        
        let attach_message = ProtocolMessage {
            action: Action::Attach,
            channel: Some(self.name.clone()),
            ..Default::default()
        };
        
        self.transport.send_message(attach_message).await?;
        
        // TODO: Wait for ATTACHED confirmation
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        info!("Attached to channel: {}", self.name);
        Ok(())
    }
    
    /// Detach from the channel
    pub async fn detach(&self) -> AblyResult<()> {
        info!("Detaching from channel: {}", self.name);
        
        let detach_message = ProtocolMessage {
            action: Action::Detach,
            channel: Some(self.name.clone()),
            ..Default::default()
        };
        
        self.transport.send_message(detach_message).await?;
        
        info!("Detached from channel: {}", self.name);
        Ok(())
    }
    
    /// Publish a message to the channel
    pub async fn publish(&self, message: Message) -> AblyResult<()> {
        let protocol_message = ProtocolMessage {
            action: Action::Message,
            channel: Some(self.name.clone()),
            messages: Some(vec![message]),
            ..Default::default()
        };
        
        self.transport.send_message(protocol_message).await?;
        Ok(())
    }
    
    /// Subscribe to messages
    pub async fn subscribe<F>(&self, handler: F) 
    where
        F: Fn(Message) + Send + Sync + 'static,
    {
        let mut handlers = self.message_handlers.write().await;
        handlers.push(Arc::new(handler));
    }
    
    /// Subscribe to presence messages
    pub async fn subscribe_presence<F>(&self, handler: F)
    where
        F: Fn(PresenceMessage) + Send + Sync + 'static,
    {
        let mut handlers = self.presence_handlers.write().await;
        handlers.push(Arc::new(handler));
    }
    
    /// Get presence members
    pub async fn presence_get(&self) -> AblyResult<Vec<PresenceMessage>> {
        // TODO: Implement presence get
        Ok(Vec::new())
    }
    
    /// Enter presence
    pub async fn presence_enter(&self, data: Option<serde_json::Value>) -> AblyResult<()> {
        let presence_message = PresenceMessage {
            action: Some(PresenceAction::Enter),
            client_id: Some("rust-client".to_string()),
            data,
            ..Default::default()
        };
        
        let protocol_message = ProtocolMessage {
            action: Action::Presence,
            channel: Some(self.name.clone()),
            presence: Some(vec![presence_message]),
            ..Default::default()
        };
        
        self.transport.send_message(protocol_message).await?;
        Ok(())
    }
    
    /// Leave presence
    pub async fn presence_leave(&self) -> AblyResult<()> {
        let presence_message = PresenceMessage {
            action: Some(PresenceAction::Leave),
            client_id: Some("rust-client".to_string()),
            ..Default::default()
        };
        
        let protocol_message = ProtocolMessage {
            action: Action::Presence,
            channel: Some(self.name.clone()),
            presence: Some(vec![presence_message]),
            ..Default::default()
        };
        
        self.transport.send_message(protocol_message).await?;
        Ok(())
    }
    
    /// Handle incoming message
    async fn handle_message(&self, message: ProtocolMessage) {
        if let Some(messages) = message.messages {
            let handlers = self.message_handlers.read().await;
            for msg in messages {
                for handler in handlers.iter() {
                    handler(msg.clone());
                }
            }
        }
        
        if let Some(presence) = message.presence {
            let handlers = self.presence_handlers.read().await;
            for msg in presence {
                for handler in handlers.iter() {
                    handler(msg.clone());
                }
            }
        }
    }
    
    /// Handle channel attached
    async fn handle_attached(&self) {
        info!("Channel attached: {}", self.name);
    }
    
    /// Handle channel detached
    async fn handle_detached(&self) {
        info!("Channel detached: {}", self.name);
    }
}

/// Builder for realtime client
pub struct RealtimeClientBuilder {
    api_key: Option<String>,
    token: Option<String>,
    client_id: Option<String>,
    recover: Option<String>,
    auto_connect: bool,
}

impl Default for RealtimeClientBuilder {
    fn default() -> Self {
        Self {
            api_key: None,
            token: None,
            client_id: None,
            recover: None,
            auto_connect: true,
        }
    }
}

impl RealtimeClientBuilder {
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }
    
    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }
    
    pub fn client_id(mut self, id: impl Into<String>) -> Self {
        self.client_id = Some(id.into());
        self
    }
    
    pub fn recover(mut self, recover: impl Into<String>) -> Self {
        self.recover = Some(recover.into());
        self
    }
    
    pub fn auto_connect(mut self, auto: bool) -> Self {
        self.auto_connect = auto;
        self
    }
    
    pub async fn build(self) -> AblyResult<RealtimeClient> {
        let api_key = self.api_key
            .ok_or_else(|| AblyError::unexpected("API key required"))?;
        
        let client = RealtimeClient::new(api_key).await?;
        
        if self.auto_connect {
            client.connect().await?;
        }
        
        Ok(client)
    }
}