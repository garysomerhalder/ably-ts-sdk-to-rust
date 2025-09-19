// 🟡 YELLOW Phase: Minimal Realtime client implementation
// WebSocket-based real-time client

use crate::auth::AuthMode;
use crate::channel::{Channel, ChannelManager, ChannelOptions};
use crate::connection::state_machine::{ConnectionStateMachine, ConnectionState, ConnectionEvent};
use crate::error::{AblyError, AblyResult};
use crate::protocol::messages::{ProtocolMessage, Action, Message, PresenceMessage, ErrorInfo, PresenceAction};
use crate::transport::{WebSocketTransport, TransportConfig};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn, error};

/// Realtime client for WebSocket connections
pub struct RealtimeClient {
    transport: Arc<WebSocketTransport>,
    state_machine: Arc<ConnectionStateMachine>,
    channel_manager: Arc<ChannelManager>,
    message_tx: mpsc::Sender<ProtocolMessage>,
    message_rx: Arc<RwLock<mpsc::Receiver<ProtocolMessage>>>,
    msg_serial: Arc<RwLock<i64>>,
}

impl RealtimeClient {
    /// Create a new realtime client with API key
    pub async fn new(api_key: impl Into<String>) -> AblyResult<Self> {
        Self::new_internal(api_key.into()).await
    }

    /// Create a new realtime client with API key (convenience method)
    pub async fn with_api_key(api_key: impl Into<String>) -> Self {
        Self::new_internal(api_key.into()).await.expect("Failed to create RealtimeClient")
    }

    /// Internal creation method
    async fn new_internal(api_key: String) -> AblyResult<Self> {
        let config = TransportConfig::default();
        let auth = AuthMode::ApiKey(api_key);
        let url = "wss://realtime.ably.io/"; // Trailing slash is REQUIRED!
        let transport = WebSocketTransport::new(url, config, auth);
        
        let state_machine = Arc::new(ConnectionStateMachine::new());
        
        // Start state machine event processor
        let sm_clone = state_machine.clone();
        tokio::spawn(async move {
            sm_clone.process_events().await;
        });
        
        let transport_arc = Arc::new(transport);
        let channel_manager = Arc::new(ChannelManager::new(Arc::clone(&transport_arc)));
        let (message_tx, message_rx) = mpsc::channel(100);

        let client = Self {
            transport: transport_arc,
            state_machine,
            channel_manager,
            message_tx,
            message_rx: Arc::new(RwLock::new(message_rx)),
            msg_serial: Arc::new(RwLock::new(0)),
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
        
        // Wait for CONNECTED message from server
        let timeout = std::time::Duration::from_secs(10);
        let start = std::time::Instant::now();
        
        while !self.is_connected().await {
            if start.elapsed() > timeout {
                return Err(AblyError::connection_failed("Timeout waiting for CONNECTED message"));
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        
        info!("Connected to Ably realtime");
        Ok(())
    }
    
    /// Disconnect from Ably
    pub async fn disconnect(&self) -> AblyResult<()> {
        info!("Disconnecting from Ably realtime...");
        
        self.state_machine.send_event(ConnectionEvent::Disconnect).await?;
        self.transport.disconnect().await?;
        
        // Wait for disconnection to complete
        let timeout = std::time::Duration::from_secs(5);
        let start = std::time::Instant::now();
        
        while self.is_connected().await {
            if start.elapsed() > timeout {
                // Force disconnection state if timeout
                self.state_machine.send_event(ConnectionEvent::Disconnected(None)).await?;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        
        info!("Disconnected from Ably realtime");
        Ok(())
    }
    
    /// Get or create a channel
    pub async fn channel(&self, name: impl Into<String>) -> Arc<Channel> {
        let name = name.into();
        self.channel_manager.get_or_create(&name).await
    }

    /// Get a channel with options
    pub async fn channel_with_options(&self, name: impl Into<String>, options: ChannelOptions) -> Arc<Channel> {
        let name = name.into();
        self.channel_manager.get_or_create_with_options(&name, options).await
    }
    
    /// Get connection state
    pub async fn state(&self) -> ConnectionState {
        self.state_machine.state().await
    }
    
    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        matches!(self.state().await, ConnectionState::Connected)
    }
    
    /// Get connection ID if connected
    pub async fn connection_id(&self) -> Option<String> {
        self.state_machine.connection_id().await
    }
    
    /// Start background message processor
    fn start_message_processor(&self) {
        let transport = self.transport.clone();
        let state_machine = self.state_machine.clone();
        let channel_manager = self.channel_manager.clone();
        
        tokio::spawn(async move {
            loop {
                // Receive messages from transport
                match transport.receive_message().await {
                    Ok(message) => {
                        debug!("Received message: {:?}", message.action);
                        
                        // Process message based on action
                        match message.action {
                            Action::Connected => {
                                // Extract connection ID from message
                                let connection_id = message.connection_id.clone()
                                    .or_else(|| message.connection_details.as_ref()
                                        .and_then(|d| d.connection_key.clone()))
                                    .unwrap_or_else(|| "unknown".to_string());
                                    
                                let _ = state_machine.send_event(ConnectionEvent::Connected(connection_id)).await;
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
                            Action::Message | Action::Attached | Action::Detached | Action::Presence => {
                                // Route to channel manager
                                if let Some(_channel_name) = &message.channel {
                                    let _ = channel_manager.handle_protocol_message(message.clone()).await;
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
    msg_serial: Arc<RwLock<i64>>,
}

type MessageHandler = Arc<dyn Fn(Message) + Send + Sync>;
type PresenceHandler = Arc<dyn Fn(PresenceMessage) + Send + Sync>;

impl RealtimeChannel {
    fn new(
        name: String,
        transport: Arc<WebSocketTransport>,
        state_machine: Arc<ConnectionStateMachine>,
        msg_serial: Arc<RwLock<i64>>,
    ) -> Self {
        Self {
            name,
            transport,
            state_machine,
            message_handlers: Arc::new(RwLock::new(Vec::new())),
            presence_handlers: Arc::new(RwLock::new(Vec::new())),
            msg_serial,
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
        // Increment and get message serial
        let mut serial = self.msg_serial.write().await;
        *serial += 1;
        let current_serial = *serial;
        drop(serial);
        
        let protocol_message = ProtocolMessage {
            action: Action::Message,
            channel: Some(self.name.clone()),
            messages: Some(vec![message]),
            msg_serial: Some(current_serial),
            ..Default::default()
        };
        
        self.transport.send_message(protocol_message).await?;
        Ok(())
    }
    
    /// Subscribe to messages (returns a receiver for the messages)
    pub async fn subscribe(&self) -> mpsc::Receiver<Message> {
        let (tx, rx) = mpsc::channel(100);
        let tx = Arc::new(tx);
        
        let handler: MessageHandler = Arc::new(move |msg: Message| {
            let tx = tx.clone();
            tokio::spawn(async move {
                let _ = tx.send(msg).await;
            });
        });
        
        let mut handlers = self.message_handlers.write().await;
        handlers.push(handler);
        
        rx
    }
    
    /// Subscribe to messages with handler function
    pub async fn subscribe_with_handler<F>(&self, handler: F) 
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
        // Increment and get message serial
        let mut serial = self.msg_serial.write().await;
        *serial += 1;
        let current_serial = *serial;
        drop(serial);
        
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
            msg_serial: Some(current_serial),
            ..Default::default()
        };
        
        self.transport.send_message(protocol_message).await?;
        Ok(())
    }
    
    /// Leave presence
    pub async fn presence_leave(&self) -> AblyResult<()> {
        // Increment and get message serial
        let mut serial = self.msg_serial.write().await;
        *serial += 1;
        let current_serial = *serial;
        drop(serial);
        
        let presence_message = PresenceMessage {
            action: Some(PresenceAction::Leave),
            client_id: Some("rust-client".to_string()),
            ..Default::default()
        };
        
        let protocol_message = ProtocolMessage {
            action: Action::Presence,
            channel: Some(self.name.clone()),
            presence: Some(vec![presence_message]),
            msg_serial: Some(current_serial),
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