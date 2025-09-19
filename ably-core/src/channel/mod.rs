// 🟡 YELLOW Phase: Channel management implementation
// Handles channel attach/detach and state management

use crate::error::{AblyError, AblyResult};
use crate::protocol::{ProtocolMessage, Action, Message, ErrorInfo};
use crate::transport::WebSocketTransport;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::{info, warn, error};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::Utc;

/// Channel states as per Ably specification
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChannelState {
    Initialized,
    Attaching,
    Attached,
    Detaching,
    Detached,
    Suspended,
    Failed,
}

/// Channel modes/capabilities
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChannelMode {
    Publish,
    Subscribe,
    PresenceSubscribe,
    Presence,
    History,
}

/// Channel options
#[derive(Debug, Clone, Default)]
pub struct ChannelOptions {
    pub params: Option<Vec<(&'static str, &'static str)>>,
    pub modes: Option<Vec<ChannelMode>>,
    pub cipher: Option<CipherParams>,
}

/// Cipher parameters for encrypted channels
#[derive(Debug, Clone)]
pub struct CipherParams {
    pub key: Vec<u8>,
    pub algorithm: String,
}

/// Channel state information for recovery
#[derive(Debug, Clone)]
pub struct ChannelStateInfo {
    pub state: ChannelState,
    pub resume: Option<String>,
    pub reason: Option<ErrorInfo>,
    pub has_presence: bool,
    pub has_backlog: bool,
}

/// State change event
#[derive(Debug, Clone)]
pub struct StateChangeEvent {
    pub from: ChannelState,
    pub to: ChannelState,
    pub reason: Option<ErrorInfo>,
}

/// Channel implementation
pub struct Channel {
    name: String,
    state: Arc<RwLock<ChannelState>>,
    state_info: Arc<RwLock<ChannelStateInfo>>,
    options: ChannelOptions,
    transport: Arc<WebSocketTransport>,
    message_queue: Arc<RwLock<Vec<Message>>>,
    subscribers: Arc<RwLock<HashMap<String, Vec<mpsc::UnboundedSender<Message>>>>>,
    presence_map: Arc<RwLock<HashMap<String, PresenceMember>>>,
    msg_serial: Arc<RwLock<i64>>,
    attach_serial: Arc<RwLock<Option<String>>>,
    state_listeners: Arc<RwLock<Vec<mpsc::UnboundedSender<StateChangeEvent>>>>,
}

/// Presence member information
#[derive(Debug, Clone)]
pub struct PresenceMember {
    pub client_id: String,
    pub connection_id: String,
    pub data: Option<serde_json::Value>,
    pub action: PresenceAction,
}

/// Presence actions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PresenceAction {
    Enter,
    Leave,
    Update,
    Present,
    Absent,
}

impl Channel {
    /// Create a new channel
    pub fn new(name: String, transport: Arc<WebSocketTransport>) -> Self {
        Self::with_options(name, transport, ChannelOptions::default())
    }

    /// Create a channel with options
    pub fn with_options(name: String, transport: Arc<WebSocketTransport>, options: ChannelOptions) -> Self {
        let state_info = ChannelStateInfo {
            state: ChannelState::Initialized,
            resume: None,
            reason: None,
            has_presence: false,
            has_backlog: false,
        };

        Self {
            name,
            state: Arc::new(RwLock::new(ChannelState::Initialized)),
            state_info: Arc::new(RwLock::new(state_info)),
            options,
            transport,
            message_queue: Arc::new(RwLock::new(Vec::new())),
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            presence_map: Arc::new(RwLock::new(HashMap::new())),
            msg_serial: Arc::new(RwLock::new(0)),
            attach_serial: Arc::new(RwLock::new(None)),
            state_listeners: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get channel name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get current channel state
    pub async fn state(&self) -> ChannelState {
        *self.state.read().await
    }

    /// Get detailed state information
    pub async fn state_info(&self) -> ChannelStateInfo {
        self.state_info.read().await.clone()
    }

    /// Update channel state with info
    async fn update_state(&self, new_state: ChannelState, reason: Option<ErrorInfo>) {
        let mut state = self.state.write().await;
        let mut state_info = self.state_info.write().await;

        let old_state = *state;
        *state = new_state;
        state_info.state = new_state;
        state_info.reason = reason.clone();

        drop(state);
        drop(state_info);

        // Notify state change listeners
        let listeners = self.state_listeners.read().await;
        let event = StateChangeEvent {
            from: old_state,
            to: new_state,
            reason,
        };

        for listener in listeners.iter() {
            let _ = listener.send(event.clone());
        }

        info!("Channel {} state transition: {:?} -> {:?}", self.name, old_state, new_state);
    }

    /// Subscribe to state changes
    pub async fn on_state_change(&self) -> mpsc::UnboundedReceiver<StateChangeEvent> {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut listeners = self.state_listeners.write().await;
        listeners.push(tx);
        rx
    }

    /// Get message serial
    pub async fn get_msg_serial(&self) -> i64 {
        *self.msg_serial.read().await
    }

    /// Get attach serial
    pub async fn get_attach_serial(&self) -> Option<String> {
        self.attach_serial.read().await.clone()
    }

    /// Get queue size
    pub async fn get_queue_size(&self) -> usize {
        self.message_queue.read().await.len()
    }

    /// Handle error
    pub async fn handle_error(&self, error: ErrorInfo) {
        self.update_state(ChannelState::Failed, Some(error)).await;
    }

    /// Recover from failed state
    pub async fn recover(&self) -> AblyResult<()> {
        if self.state().await != ChannelState::Failed {
            return Err(AblyError::invalid_request("Channel is not in failed state"));
        }

        info!("Recovering channel {} from failed state", self.name);
        self.attach().await
    }

    /// Suspend channel
    pub async fn suspend(&self) -> AblyResult<()> {
        self.update_state(ChannelState::Suspended, None).await;
        Ok(())
    }

    /// Resume channel with serial
    pub async fn resume(&self, serial: Option<String>) -> AblyResult<()> {
        if let Some(s) = serial {
            let mut attach_serial = self.attach_serial.write().await;
            *attach_serial = Some(s);
        }
        self.attach().await
    }

    /// Enter presence
    pub async fn presence_enter(&self, data: Option<serde_json::Value>) -> AblyResult<()> {
        // Auto-attach if not attached
        if self.state().await != ChannelState::Attached {
            self.attach().await?;
        }

        // Update state info
        let mut state_info = self.state_info.write().await;
        state_info.has_presence = true;
        drop(state_info);

        // Send presence enter message
        let presence_msg = crate::protocol::PresenceMessage {
            action: Some(crate::protocol::PresenceAction::Enter),
            client_id: Some("rust-client".to_string()),
            data,
            ..Default::default()
        };

        let msg = ProtocolMessage {
            action: Action::Presence,
            channel: Some(self.name.clone()),
            presence: Some(vec![presence_msg]),
            ..Default::default()
        };

        self.transport.send_message(msg).await?;
        Ok(())
    }

    /// Leave presence
    pub async fn presence_leave(&self) -> AblyResult<()> {
        // Send presence leave message
        let presence_msg = crate::protocol::PresenceMessage {
            action: Some(crate::protocol::PresenceAction::Leave),
            client_id: Some("rust-client".to_string()),
            ..Default::default()
        };

        let msg = ProtocolMessage {
            action: Action::Presence,
            channel: Some(self.name.clone()),
            presence: Some(vec![presence_msg]),
            ..Default::default()
        };

        self.transport.send_message(msg).await?;

        // Update state info
        let mut state_info = self.state_info.write().await;
        state_info.has_presence = false;

        Ok(())
    }

    /// Attach to the channel with retry logic
    pub async fn attach(&self) -> AblyResult<()> {
        // Validate channel name
        if self.name.is_empty() {
            self.update_state(ChannelState::Failed, Some(ErrorInfo {
                code: 40003,
                message: Some("Channel name cannot be empty".to_string()),
                ..Default::default()
            })).await;
            return Err(AblyError::invalid_request("Channel name cannot be empty"));
        }

        let current_state = self.state().await;

        match current_state {
            ChannelState::Attached => return Ok(()),
            ChannelState::Attaching => {
                // Wait for attach to complete
                self.wait_for_state(ChannelState::Attached).await?;
                return Ok(());
            }
            ChannelState::Failed => {
                // Attempt recovery from failed state
                info!("Attempting to recover channel {} from failed state", self.name);
            }
            _ => {}
        }

        // Retry logic for attach operation
        let max_retries = 3;
        let mut attempt = 0;
        let mut last_error = None;

        while attempt < max_retries {
            attempt += 1;

            // Update state to Attaching
            self.update_state(ChannelState::Attaching, None).await;

            info!("Attaching to channel: {} (attempt {}/{})", self.name, attempt, max_retries);

            // Send ATTACH protocol message
            let attach_msg = ProtocolMessage::attach(self.name.clone(), self.build_attach_flags());

            match self.transport.send_message(attach_msg).await {
                Ok(_) => {
                    // Wait for ATTACHED response
                    match self.wait_for_state(ChannelState::Attached).await {
                        Ok(_) => {
                            info!("Successfully attached to channel: {}", self.name);
                            return Ok(());
                        }
                        Err(e) => {
                            last_error = Some(e);
                            if attempt < max_retries {
                                warn!("Channel {} attach attempt {} failed, retrying...", self.name, attempt);
                                tokio::time::sleep(tokio::time::Duration::from_millis(500 * attempt as u64)).await;
                            }
                        }
                    }
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < max_retries {
                        warn!("Failed to send attach message for channel {}, retrying...", self.name);
                        tokio::time::sleep(tokio::time::Duration::from_millis(500 * attempt as u64)).await;
                    }
                }
            }
        }

        // All retries exhausted, set to failed state
        self.update_state(ChannelState::Failed, last_error.as_ref().and_then(|e| {
            if let AblyError::Api { code, message } = e {
                Some(ErrorInfo {
                    code: *code,
                    message: Some(message.clone()),
                    ..Default::default()
                })
            } else {
                None
            }
        })).await;

        error!("Failed to attach to channel {} after {} attempts", self.name, max_retries);
        Err(last_error.unwrap_or_else(|| AblyError::channel_failed(format!(
            "Failed to attach to channel {} after {} attempts",
            self.name, max_retries
        ))))
    }

    /// Detach from the channel
    pub async fn detach(&self) -> AblyResult<()> {
        let current_state = self.state().await;

        match current_state {
            ChannelState::Detached | ChannelState::Initialized => return Ok(()),
            ChannelState::Detaching => {
                self.wait_for_state(ChannelState::Detached).await?;
                return Ok(());
            }
            ChannelState::Failed => {
                self.update_state(ChannelState::Detached, None).await;
                return Ok(());
            }
            _ => {}
        }

        // Update state to Detaching
        self.update_state(ChannelState::Detaching, None).await;

        info!("Detaching from channel: {}", self.name);

        // Send DETACH protocol message
        let detach_msg = ProtocolMessage::detach(self.name.clone());

        self.transport.send_message(detach_msg).await?;

        // Wait for DETACHED response
        self.wait_for_state(ChannelState::Detached).await?;

        info!("Successfully detached from channel: {}", self.name);
        Ok(())
    }

    /// Publish a message to the channel with retry logic
    pub async fn publish(&self, name: &str, data: impl Into<String>) -> AblyResult<()> {
        // Auto-attach if not attached
        if self.state().await != ChannelState::Attached {
            self.attach().await?;
        }

        // Increment message serial
        let mut serial = self.msg_serial.write().await;
        *serial += 1;
        let msg_serial = *serial;
        drop(serial);

        let message = Message {
            name: Some(name.to_string()),
            data: Some(serde_json::json!(data.into())),
            id: Some(format!("msg:{}:{}", self.name, uuid::Uuid::new_v4())),
            timestamp: Some(chrono::Utc::now().timestamp_millis()),
            ..Default::default()
        };

        let publish_msg = ProtocolMessage::message(self.name.clone(), vec![message.clone()]);

        // Retry logic for publish
        let max_retries = 3;
        let mut attempt = 0;
        let mut last_error = None;

        while attempt < max_retries {
            attempt += 1;

            match self.transport.send_message(publish_msg.clone()).await {
                Ok(_) => {
                    info!("Published message to channel {}", self.name);
                    return Ok(());
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < max_retries {
                        warn!("Failed to publish to channel {} (attempt {}/{}), retrying...",
                              self.name, attempt, max_retries);
                        tokio::time::sleep(tokio::time::Duration::from_millis(200 * attempt as u64)).await;
                    }
                }
            }
        }

        error!("Failed to publish to channel {} after {} attempts", self.name, max_retries);
        Err(last_error.unwrap_or_else(|| AblyError::network(format!(
            "Failed to publish to channel {} after {} attempts",
            self.name, max_retries
        ))))
    }

    /// Subscribe to messages on this channel
    pub async fn subscribe(&self, event: Option<String>) -> mpsc::UnboundedReceiver<Message> {
        let (tx, rx) = mpsc::unbounded_channel();

        let event_name = event.unwrap_or_else(|| "*".to_string());

        let mut subscribers = self.subscribers.write().await;
        subscribers
            .entry(event_name)
            .or_insert_with(Vec::new)
            .push(tx);

        rx
    }

    /// Handle incoming protocol message for this channel
    pub async fn handle_protocol_message(&self, msg: ProtocolMessage) -> AblyResult<()> {
        match msg.action {
            Action::Attached => {
                self.update_state(ChannelState::Attached, None).await;

                // Store attach serial if present
                if let Some(serial) = msg.channel_serial.clone() {
                    let mut attach_serial = self.attach_serial.write().await;
                    *attach_serial = Some(serial);
                }

                // Update state info flags if present
                if let Some(flags) = msg.flags {
                    let mut state_info = self.state_info.write().await;
                    state_info.has_presence = flags & crate::protocol::flags::HAS_PRESENCE != 0;
                    state_info.has_backlog = flags & crate::protocol::flags::HAS_BACKLOG != 0;
                    state_info.resume = msg.channel_serial;
                }

                info!("Channel {} attached", self.name);
            }
            Action::Detached => {
                self.update_state(ChannelState::Detached, msg.error).await;

                // Clear state on detach
                let mut state_info = self.state_info.write().await;
                state_info.has_presence = false;
                state_info.has_backlog = false;
                state_info.resume = None;

                info!("Channel {} detached", self.name);
            }
            Action::Message => {
                if let Some(messages) = msg.messages {
                    self.handle_messages(messages).await?;
                }
            }
            Action::Presence => {
                if let Some(presence) = msg.presence {
                    self.handle_presence(presence).await?;
                }
            }
            Action::Error => {
                self.update_state(ChannelState::Failed, msg.error).await;
                error!("Channel {} error", self.name);
            }
            _ => {}
        }

        Ok(())
    }

    /// Handle incoming messages
    async fn handle_messages(&self, messages: Vec<Message>) -> AblyResult<()> {
        let subscribers = self.subscribers.read().await;

        for message in messages {
            let event_name = message.name.clone().unwrap_or_else(|| "*".to_string());

            // Send to exact event subscribers
            if let Some(subs) = subscribers.get(&event_name) {
                for sub in subs {
                    let _ = sub.send(message.clone());
                }
            }

            // Send to wildcard subscribers
            if event_name != "*" {
                if let Some(subs) = subscribers.get("*") {
                    for sub in subs {
                        let _ = sub.send(message.clone());
                    }
                }
            }

            // Add to message queue
            let mut queue = self.message_queue.write().await;
            queue.push(message);
            if queue.len() > 1000 {
                queue.drain(0..500);
            }

            // Update backlog flag if queue has messages
            if queue.len() > 0 {
                let mut state_info = self.state_info.write().await;
                state_info.has_backlog = true;
            }
        }

        Ok(())
    }

    /// Handle presence messages
    async fn handle_presence(&self, presence: Vec<crate::protocol::PresenceMessage>) -> AblyResult<()> {
        let mut presence_map = self.presence_map.write().await;

        for msg in presence {
            if let (Some(client_id), Some(action)) = (&msg.client_id, &msg.action) {
                match action {
                    crate::protocol::PresenceAction::Enter | crate::protocol::PresenceAction::Present => {
                        presence_map.insert(client_id.clone(), PresenceMember {
                            client_id: client_id.clone(),
                            connection_id: msg.connection_id.unwrap_or_default(),
                            data: msg.data,
                            action: PresenceAction::Present,
                        });
                    }
                    crate::protocol::PresenceAction::Leave | crate::protocol::PresenceAction::Absent => {
                        presence_map.remove(client_id);
                    }
                    crate::protocol::PresenceAction::Update => {
                        if let Some(member) = presence_map.get_mut(client_id) {
                            member.data = msg.data;
                            member.action = PresenceAction::Update;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Wait for a specific channel state with exponential backoff
    async fn wait_for_state(&self, target: ChannelState) -> AblyResult<()> {
        let timeout_duration = tokio::time::Duration::from_secs(10);
        let start = tokio::time::Instant::now();
        let mut delay = tokio::time::Duration::from_millis(50);

        loop {
            if self.state().await == target {
                return Ok(());
            }

            // Check for failure state
            if self.state().await == ChannelState::Failed {
                return Err(AblyError::channel_failed(format!(
                    "Channel {} entered failed state while waiting for {:?}",
                    self.name, target
                )));
            }

            if start.elapsed() > timeout_duration {
                return Err(AblyError::timeout(format!(
                    "Timeout waiting for channel {} to reach state {:?}",
                    self.name, target
                )));
            }

            tokio::time::sleep(delay).await;

            // Exponential backoff with max delay of 1 second
            delay = std::cmp::min(delay * 2, tokio::time::Duration::from_secs(1));
        }
    }

    /// Build attach flags based on channel options
    fn build_attach_flags(&self) -> Option<u32> {
        let mut flags = 0u32;

        if let Some(modes) = &self.options.modes {
            for mode in modes {
                match mode {
                    ChannelMode::Publish => flags |= crate::protocol::flags::PUBLISH,
                    ChannelMode::Subscribe => flags |= crate::protocol::flags::SUBSCRIBE,
                    ChannelMode::PresenceSubscribe => flags |= crate::protocol::flags::PRESENCE_SUBSCRIBE,
                    ChannelMode::Presence => flags |= crate::protocol::flags::PRESENCE,
                    _ => {}
                }
            }
        }

        if flags > 0 {
            Some(flags)
        } else {
            None
        }
    }
}

/// Channel manager for handling multiple channels
pub struct ChannelManager {
    channels: Arc<RwLock<HashMap<String, Arc<Channel>>>>,
    transport: Arc<WebSocketTransport>,
}

impl ChannelManager {
    /// Create new channel manager
    pub fn new(transport: Arc<WebSocketTransport>) -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            transport,
        }
    }

    /// Get or create a channel
    pub async fn get_or_create(&self, name: &str) -> Arc<Channel> {
        let mut channels = self.channels.write().await;

        if let Some(channel) = channels.get(name) {
            return Arc::clone(channel);
        }

        let channel = Arc::new(Channel::new(name.to_string(), Arc::clone(&self.transport)));
        channels.insert(name.to_string(), Arc::clone(&channel));
        channel
    }

    /// Get a channel with options
    pub async fn get_or_create_with_options(&self, name: &str, options: ChannelOptions) -> Arc<Channel> {
        let mut channels = self.channels.write().await;

        if let Some(channel) = channels.get(name) {
            return Arc::clone(channel);
        }

        let channel = Arc::new(Channel::with_options(
            name.to_string(),
            Arc::clone(&self.transport),
            options,
        ));
        channels.insert(name.to_string(), Arc::clone(&channel));
        channel
    }

    /// Handle incoming protocol messages
    pub async fn handle_protocol_message(&self, msg: ProtocolMessage) -> AblyResult<()> {
        if let Some(channel_name) = &msg.channel {
            if let Some(channel) = self.channels.read().await.get(channel_name) {
                channel.handle_protocol_message(msg).await?;
            }
        }

        Ok(())
    }
}