// Connection state machine implementation
// Handles all state transitions for Ably realtime connections

use crate::error::{AblyError, AblyResult};
use crate::protocol::{ProtocolMessage, Action, ErrorInfo};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn, error};

/// Connection states as per Ably specification
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionState {
    Initialized,
    Connecting,
    Connected,
    Disconnected,
    Suspended,
    Closing,
    Closed,
    Failed,
}

/// Connection events that trigger state transitions
#[derive(Debug, Clone)]
pub enum ConnectionEvent {
    Connect,
    Connected(String), // connection_id
    Disconnect,
    Disconnected(Option<ErrorInfo>),
    Close,
    Closed,
    Error(ErrorInfo),
    Suspend,
    Retry,
}

/// Connection state machine
pub struct ConnectionStateMachine {
    state: Arc<RwLock<ConnectionState>>,
    connection_id: Arc<RwLock<Option<String>>>,
    error_info: Arc<RwLock<Option<ErrorInfo>>>,
    state_history: Arc<RwLock<Vec<StateTransition>>>,
    event_tx: mpsc::UnboundedSender<ConnectionEvent>,
    event_rx: Arc<RwLock<mpsc::UnboundedReceiver<ConnectionEvent>>>,
    listeners: Arc<RwLock<Vec<StateChangeListener>>>,
    retry_count: Arc<RwLock<u32>>,
    last_activity: Arc<RwLock<Instant>>,
}

#[derive(Debug, Clone)]
struct StateTransition {
    from: ConnectionState,
    to: ConnectionState,
    event: String,
    timestamp: Instant,
    error: Option<ErrorInfo>,
}

type StateChangeListener = Arc<dyn Fn(ConnectionState, ConnectionState) + Send + Sync>;

impl ConnectionStateMachine {
    /// Create a new connection state machine
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        
        Self {
            state: Arc::new(RwLock::new(ConnectionState::Initialized)),
            connection_id: Arc::new(RwLock::new(None)),
            error_info: Arc::new(RwLock::new(None)),
            state_history: Arc::new(RwLock::new(Vec::new())),
            event_tx: tx,
            event_rx: Arc::new(RwLock::new(rx)),
            listeners: Arc::new(RwLock::new(Vec::new())),
            retry_count: Arc::new(RwLock::new(0)),
            last_activity: Arc::new(RwLock::new(Instant::now())),
        }
    }

    /// Get current state
    pub async fn state(&self) -> ConnectionState {
        *self.state.read().await
    }

    /// Get connection ID
    pub async fn connection_id(&self) -> Option<String> {
        self.connection_id.read().await.clone()
    }

    /// Get last error
    pub async fn error(&self) -> Option<ErrorInfo> {
        self.error_info.read().await.clone()
    }

    /// Send an event to the state machine
    pub async fn send_event(&self, event: ConnectionEvent) -> AblyResult<()> {
        self.event_tx.send(event)
            .map_err(|_| AblyError::Internal {
                message: "Failed to send event to state machine".to_string(),
            })
    }

    /// Process events and handle state transitions
    pub async fn process_events(&self) {
        let mut rx = self.event_rx.write().await;
        
        while let Some(event) = rx.recv().await {
            let current_state = *self.state.read().await;
            let new_state = self.handle_transition(current_state, &event).await;
            
            if new_state != current_state {
                self.transition_to(new_state, event).await;
            }
        }
    }

    /// Handle state transition logic
    async fn handle_transition(&self, current: ConnectionState, event: &ConnectionEvent) -> ConnectionState {
        match (current, event) {
            // From Initialized
            (ConnectionState::Initialized, ConnectionEvent::Connect) => ConnectionState::Connecting,
            
            // From Connecting
            (ConnectionState::Connecting, ConnectionEvent::Connected(_)) => ConnectionState::Connected,
            (ConnectionState::Connecting, ConnectionEvent::Disconnected(_)) => ConnectionState::Disconnected,
            (ConnectionState::Connecting, ConnectionEvent::Error(_)) => ConnectionState::Failed,
            (ConnectionState::Connecting, ConnectionEvent::Close) => ConnectionState::Closing,
            
            // From Connected
            (ConnectionState::Connected, ConnectionEvent::Disconnect) => ConnectionState::Disconnected,
            (ConnectionState::Connected, ConnectionEvent::Disconnected(_)) => ConnectionState::Disconnected,
            (ConnectionState::Connected, ConnectionEvent::Close) => ConnectionState::Closing,
            (ConnectionState::Connected, ConnectionEvent::Error(e)) if self.is_fatal_error(e) => ConnectionState::Failed,
            (ConnectionState::Connected, ConnectionEvent::Error(_)) => ConnectionState::Disconnected,
            
            // From Disconnected
            (ConnectionState::Disconnected, ConnectionEvent::Connect) => ConnectionState::Connecting,
            (ConnectionState::Disconnected, ConnectionEvent::Retry) => ConnectionState::Connecting,
            (ConnectionState::Disconnected, ConnectionEvent::Suspend) => ConnectionState::Suspended,
            (ConnectionState::Disconnected, ConnectionEvent::Close) => ConnectionState::Closing,
            
            // From Suspended
            (ConnectionState::Suspended, ConnectionEvent::Connect) => ConnectionState::Connecting,
            (ConnectionState::Suspended, ConnectionEvent::Close) => ConnectionState::Closing,
            
            // From Closing
            (ConnectionState::Closing, ConnectionEvent::Closed) => ConnectionState::Closed,
            (ConnectionState::Closing, ConnectionEvent::Error(_)) => ConnectionState::Failed,
            
            // Terminal states
            (ConnectionState::Closed, _) => ConnectionState::Closed,
            (ConnectionState::Failed, ConnectionEvent::Connect) => ConnectionState::Connecting,
            (ConnectionState::Failed, _) => ConnectionState::Failed,
            
            // Default: no transition
            _ => current,
        }
    }

    /// Perform state transition
    async fn transition_to(&self, new_state: ConnectionState, event: ConnectionEvent) {
        let current_state = *self.state.read().await;
        
        info!("State transition: {:?} -> {:?} (event: {:?})", current_state, new_state, event);
        
        // Update state
        let mut state = self.state.write().await;
        *state = new_state;
        drop(state);
        
        // Handle event-specific updates
        match event {
            ConnectionEvent::Connected(ref id) => {
                let mut conn_id = self.connection_id.write().await;
                *conn_id = Some(id.clone());
                
                let mut retry = self.retry_count.write().await;
                *retry = 0;
            }
            ConnectionEvent::Error(ref e) | ConnectionEvent::Disconnected(Some(ref e)) => {
                let mut error = self.error_info.write().await;
                *error = Some(e.clone());
            }
            ConnectionEvent::Retry => {
                let mut retry = self.retry_count.write().await;
                *retry += 1;
            }
            _ => {}
        }
        
        // Update activity timestamp
        let mut activity = self.last_activity.write().await;
        *activity = Instant::now();
        
        // Record transition
        let mut history = self.state_history.write().await;
        history.push(StateTransition {
            from: current_state,
            to: new_state,
            event: format!("{:?}", event),
            timestamp: Instant::now(),
            error: match event {
                ConnectionEvent::Error(ref e) | ConnectionEvent::Disconnected(Some(ref e)) => Some(e.clone()),
                _ => None,
            },
        });
        
        // Limit history size
        if history.len() > 100 {
            history.drain(0..50);
        }
        
        // Notify listeners
        self.notify_listeners(current_state, new_state).await;
    }

    /// Check if error is fatal
    fn is_fatal_error(&self, error: &ErrorInfo) -> bool {
        // Fatal error codes that should transition to Failed state
        matches!(error.code, 40000..=40099 | 40100..=40199)
    }

    /// Add state change listener
    pub async fn add_listener<F>(&self, listener: F)
    where
        F: Fn(ConnectionState, ConnectionState) + Send + Sync + 'static,
    {
        let mut listeners = self.listeners.write().await;
        listeners.push(Arc::new(listener));
    }

    /// Notify all listeners of state change
    async fn notify_listeners(&self, from: ConnectionState, to: ConnectionState) {
        let listeners = self.listeners.read().await;
        for listener in listeners.iter() {
            listener(from, to);
        }
    }

    /// Get retry count
    pub async fn retry_count(&self) -> u32 {
        *self.retry_count.read().await
    }

    /// Get time since last activity
    pub async fn idle_time(&self) -> Duration {
        self.last_activity.read().await.elapsed()
    }

    /// Check if connection should be suspended
    pub async fn should_suspend(&self) -> bool {
        let retry_count = *self.retry_count.read().await;
        let idle_time = self.idle_time().await;
        
        // Suspend after 10 retries or 2 minutes of inactivity
        retry_count > 10 || idle_time > Duration::from_secs(120)
    }

    /// Get state history
    pub async fn history(&self) -> Vec<StateTransition> {
        self.state_history.read().await.clone()
    }

    /// Reset state machine
    pub async fn reset(&self) {
        let mut state = self.state.write().await;
        *state = ConnectionState::Initialized;
        
        let mut conn_id = self.connection_id.write().await;
        *conn_id = None;
        
        let mut error = self.error_info.write().await;
        *error = None;
        
        let mut retry = self.retry_count.write().await;
        *retry = 0;
        
        let mut history = self.state_history.write().await;
        history.clear();
    }
}

/// Channel state machine
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

/// Channel events
#[derive(Debug, Clone)]
pub enum ChannelEvent {
    Attach,
    Attached,
    Detach,
    Detached,
    Suspend,
    Error(ErrorInfo),
}

/// Channel state machine
pub struct ChannelStateMachine {
    state: Arc<RwLock<ChannelState>>,
    channel_name: String,
    error_info: Arc<RwLock<Option<ErrorInfo>>>,
}

impl ChannelStateMachine {
    /// Create a new channel state machine
    pub fn new(channel_name: String) -> Self {
        Self {
            state: Arc::new(RwLock::new(ChannelState::Initialized)),
            channel_name,
            error_info: Arc::new(RwLock::new(None)),
        }
    }

    /// Get current state
    pub async fn state(&self) -> ChannelState {
        *self.state.read().await
    }

    /// Process channel event
    pub async fn process_event(&self, event: ChannelEvent) {
        let current = *self.state.read().await;
        let new_state = self.handle_transition(current, &event);
        
        if new_state != current {
            info!("Channel {} state: {:?} -> {:?}", self.channel_name, current, new_state);
            
            let mut state = self.state.write().await;
            *state = new_state;
            
            if let ChannelEvent::Error(e) = event {
                let mut error = self.error_info.write().await;
                *error = Some(e);
            }
        }
    }

    /// Handle channel state transitions
    fn handle_transition(&self, current: ChannelState, event: &ChannelEvent) -> ChannelState {
        match (current, event) {
            (ChannelState::Initialized, ChannelEvent::Attach) => ChannelState::Attaching,
            (ChannelState::Attaching, ChannelEvent::Attached) => ChannelState::Attached,
            (ChannelState::Attaching, ChannelEvent::Error(_)) => ChannelState::Failed,
            (ChannelState::Attached, ChannelEvent::Detach) => ChannelState::Detaching,
            (ChannelState::Attached, ChannelEvent::Suspend) => ChannelState::Suspended,
            (ChannelState::Detaching, ChannelEvent::Detached) => ChannelState::Detached,
            (ChannelState::Detached, ChannelEvent::Attach) => ChannelState::Attaching,
            (ChannelState::Suspended, ChannelEvent::Attach) => ChannelState::Attaching,
            _ => current,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_state_transitions() {
        let sm = ConnectionStateMachine::new();
        
        assert_eq!(sm.state().await, ConnectionState::Initialized);
        
        sm.send_event(ConnectionEvent::Connect).await.unwrap();
        // Would need to process events in real implementation
        
        sm.send_event(ConnectionEvent::Connected("test-id".to_string())).await.unwrap();
        assert!(sm.connection_id().await.is_some());
    }

    #[tokio::test]
    async fn test_channel_state_transitions() {
        let sm = ChannelStateMachine::new("test-channel".to_string());
        
        assert_eq!(sm.state().await, ChannelState::Initialized);
        
        sm.process_event(ChannelEvent::Attach).await;
        assert_eq!(sm.state().await, ChannelState::Attaching);
        
        sm.process_event(ChannelEvent::Attached).await;
        assert_eq!(sm.state().await, ChannelState::Attached);
    }
}