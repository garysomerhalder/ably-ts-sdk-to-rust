// 🟢 GREEN Phase: Production-ready WebSocket resilience features

use crate::error::{AblyError, AblyResult};
use crate::transport::{WebSocketTransport, TransportState};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::{sleep, interval};
use tracing::{debug, info, warn, error};

/// Auto-reconnect manager for WebSocket transport
pub struct ReconnectManager {
    transport: Arc<WebSocketTransport>,
    max_retries: u32,
    base_delay: Duration,
    max_delay: Duration,
    retry_count: Arc<RwLock<u32>>,
    last_attempt: Arc<RwLock<Option<Instant>>>,
}

impl ReconnectManager {
    pub fn new(transport: Arc<WebSocketTransport>) -> Self {
        Self {
            transport,
            max_retries: 10,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            retry_count: Arc::new(RwLock::new(0)),
            last_attempt: Arc::new(RwLock::new(None)),
        }
    }

    /// Start monitoring connection and auto-reconnect on failure
    pub async fn start_monitoring(&self) {
        let transport = Arc::clone(&self.transport);
        let retry_count = Arc::clone(&self.retry_count);
        let last_attempt = Arc::clone(&self.last_attempt);
        let base_delay = self.base_delay;
        let max_delay = self.max_delay;
        let max_retries = self.max_retries;

        tokio::spawn(async move {
            let mut check_interval = interval(Duration::from_secs(5));
            
            loop {
                check_interval.tick().await;
                
                let state = transport.state().await;
                
                match state {
                    TransportState::Disconnected | TransportState::Failed => {
                        let mut count = retry_count.write().await;
                        
                        if *count >= max_retries {
                            error!("Max reconnection attempts reached");
                            break;
                        }
                        
                        // Calculate exponential backoff with jitter
                        let delay = calculate_backoff(*count, base_delay, max_delay);
                        info!("Reconnecting in {:?} (attempt {}/{})", delay, *count + 1, max_retries);
                        
                        sleep(delay).await;
                        
                        let mut last = last_attempt.write().await;
                        *last = Some(Instant::now());
                        
                        match transport.connect().await {
                            Ok(()) => {
                                info!("Successfully reconnected");
                                *count = 0;
                            }
                            Err(e) => {
                                error!("Reconnection failed: {}", e);
                                *count += 1;
                            }
                        }
                    }
                    TransportState::Connected => {
                        // Reset retry count on successful connection
                        let mut count = retry_count.write().await;
                        if *count > 0 {
                            debug!("Connection stable, resetting retry count");
                            *count = 0;
                        }
                    }
                    _ => {}
                }
            }
        });
    }

    /// Get current retry count
    pub async fn retry_count(&self) -> u32 {
        *self.retry_count.read().await
    }

    /// Reset retry count
    pub async fn reset(&self) {
        let mut count = self.retry_count.write().await;
        *count = 0;
    }
}

/// Heartbeat manager for keeping connection alive
pub struct HeartbeatManager {
    transport: Arc<WebSocketTransport>,
    interval: Duration,
    timeout: Duration,
    last_pong: Arc<RwLock<Option<Instant>>>,
}

impl HeartbeatManager {
    pub fn new(transport: Arc<WebSocketTransport>) -> Self {
        Self {
            transport,
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(10),
            last_pong: Arc::new(RwLock::new(None)),
        }
    }

    /// Start sending periodic heartbeats
    pub async fn start(&self) {
        let transport = Arc::clone(&self.transport);
        let interval = self.interval;
        let timeout = self.timeout;
        let last_pong = Arc::clone(&self.last_pong);

        tokio::spawn(async move {
            let mut heartbeat_interval = interval(interval);
            
            loop {
                heartbeat_interval.tick().await;
                
                let state = transport.state().await;
                if state != TransportState::Connected {
                    continue;
                }
                
                debug!("Sending heartbeat");
                
                if let Err(e) = transport.send_heartbeat().await {
                    error!("Failed to send heartbeat: {}", e);
                    continue;
                }
                
                // Wait for pong response
                sleep(timeout).await;
                
                let last = last_pong.read().await;
                if let Some(last_time) = *last {
                    if last_time.elapsed() > timeout + Duration::from_secs(5) {
                        warn!("No heartbeat response, connection may be dead");
                        // Trigger disconnect to initiate reconnection
                        let _ = transport.disconnect().await;
                    }
                }
            }
        });
    }

    /// Record pong received
    pub async fn record_pong(&self) {
        let mut last = self.last_pong.write().await;
        *last = Some(Instant::now());
        debug!("Heartbeat pong received");
    }
}

/// Message queue for handling message ordering and acknowledgments
pub struct MessageQueue {
    pending: Arc<RwLock<Vec<QueuedMessage>>>,
    acknowledged: Arc<RwLock<Vec<String>>>,
    max_queue_size: usize,
}

#[derive(Debug, Clone)]
struct QueuedMessage {
    id: String,
    content: crate::protocol::ProtocolMessage,
    timestamp: Instant,
    retry_count: u32,
}

impl MessageQueue {
    pub fn new() -> Self {
        Self {
            pending: Arc::new(RwLock::new(Vec::new())),
            acknowledged: Arc::new(RwLock::new(Vec::new())),
            max_queue_size: 1000,
        }
    }

    /// Queue a message for sending
    pub async fn enqueue(&self, message: crate::protocol::ProtocolMessage) -> AblyResult<String> {
        let mut pending = self.pending.write().await;
        
        if pending.len() >= self.max_queue_size {
            return Err(AblyError::Internal {
                message: "Message queue full".to_string(),
            });
        }
        
        let id = uuid::Uuid::new_v4().to_string();
        let queued = QueuedMessage {
            id: id.clone(),
            content: message,
            timestamp: Instant::now(),
            retry_count: 0,
        };
        
        pending.push(queued);
        Ok(id)
    }

    /// Get pending messages for retry
    pub async fn get_pending(&self) -> Vec<crate::protocol::ProtocolMessage> {
        let pending = self.pending.read().await;
        pending.iter().map(|q| q.content.clone()).collect()
    }

    /// Mark message as acknowledged
    pub async fn acknowledge(&self, id: String) {
        let mut pending = self.pending.write().await;
        pending.retain(|q| q.id != id);
        
        let mut acked = self.acknowledged.write().await;
        acked.push(id);
        
        // Limit acknowledged list size
        if acked.len() > 1000 {
            acked.drain(0..500);
        }
    }

    /// Clear all pending messages
    pub async fn clear(&self) {
        let mut pending = self.pending.write().await;
        pending.clear();
    }
}

/// Connection statistics tracking
#[derive(Debug, Default)]
pub struct ConnectionStats {
    pub connected_at: Option<Instant>,
    pub disconnected_at: Option<Instant>,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub reconnect_count: u32,
    pub error_count: u32,
}

impl ConnectionStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_connect(&mut self) {
        self.connected_at = Some(Instant::now());
    }

    pub fn record_disconnect(&mut self) {
        self.disconnected_at = Some(Instant::now());
    }

    pub fn record_message_sent(&mut self, size: usize) {
        self.messages_sent += 1;
        self.bytes_sent += size as u64;
    }

    pub fn record_message_received(&mut self, size: usize) {
        self.messages_received += 1;
        self.bytes_received += size as u64;
    }

    pub fn record_reconnect(&mut self) {
        self.reconnect_count += 1;
    }

    pub fn record_error(&mut self) {
        self.error_count += 1;
    }

    pub fn uptime(&self) -> Option<Duration> {
        self.connected_at.map(|t| t.elapsed())
    }
}

/// Calculate exponential backoff with jitter
fn calculate_backoff(retry_count: u32, base_delay: Duration, max_delay: Duration) -> Duration {
    use rand::Rng;
    
    let exponential = base_delay * 2u32.pow(retry_count);
    let capped = exponential.min(max_delay);
    
    // Add jitter (0-25% of delay)
    let jitter_range = capped.as_millis() / 4;
    let jitter = rand::thread_rng().gen_range(0..=jitter_range) as u64;
    
    capped + Duration::from_millis(jitter)
}

// Extension trait for WebSocketTransport
impl WebSocketTransport {
    /// Send a heartbeat message
    pub async fn send_heartbeat(&self) -> AblyResult<()> {
        let heartbeat = crate::protocol::ProtocolMessage::heartbeat();
        self.send_message(heartbeat).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exponential_backoff() {
        let base = Duration::from_secs(1);
        let max = Duration::from_secs(60);
        
        let delay0 = calculate_backoff(0, base, max);
        assert!(delay0 >= base && delay0 < base * 2);
        
        let delay3 = calculate_backoff(3, base, max);
        assert!(delay3 >= Duration::from_secs(8) && delay3 < Duration::from_secs(11));
        
        let delay10 = calculate_backoff(10, base, max);
        assert!(delay10 <= max + Duration::from_secs(15)); // Max + jitter
    }
}