// 🔴 RED Phase: Message history replay system for state recovery
// Integration-First approach - works with real Ably history API
// No mocks - actual message replay from production data

use crate::client::rest::RestClient;
use crate::error::{AblyError, AblyResult};
use crate::protocol::messages::{Message, PresenceMessage};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BTreeMap};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn, error};

pub mod channel_replay;
pub mod presence_replay;
pub mod state_recovery;
pub mod history_replay;

pub use channel_replay::ChannelReplay;
pub use presence_replay::PresenceReplay;
pub use state_recovery::{StateRecovery, RecoveryState, RecoveryOptions};
pub use history_replay::{
    HistoryReplay, 
    ReplayPosition, 
    ReplayOptions as HistoryReplayOptions,
    ReplayPaginator,
    ChannelSerialTracker,
    ReplayState,
};

/// Replay manager for message history-based state reconstruction
#[derive(Debug, Clone)]
pub struct ReplayManager {
    client: RestClient,
    options: ReplayOptions,
}

/// Configuration options for replay behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayOptions {
    /// Maximum number of messages to replay (default: 1000)
    pub max_messages: usize,
    /// Time range in seconds to look back (default: 3600 = 1 hour)
    pub time_range_seconds: u64,
    /// Whether to replay presence messages (default: true)
    pub include_presence: bool,
    /// Whether to apply compression during replay (default: false)
    pub compress_state: bool,
    /// Batch size for processing messages (default: 100)
    pub batch_size: usize,
    /// Timeout for each replay operation in seconds (default: 30)
    pub timeout_seconds: u64,
}

impl Default for ReplayOptions {
    fn default() -> Self {
        Self {
            max_messages: 1000,
            time_range_seconds: 3600, // 1 hour
            include_presence: true,
            compress_state: false,
            batch_size: 100,
            timeout_seconds: 30,
        }
    }
}

/// Result of a replay operation with metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayResult {
    /// Number of messages processed
    pub messages_processed: usize,
    /// Number of presence events processed
    pub presence_processed: usize,
    /// Final state snapshot
    pub final_state: HashMap<String, serde_json::Value>,
    /// Presence state snapshot
    pub presence_state: HashMap<String, PresenceMessage>,
    /// Time taken in milliseconds
    pub duration_ms: u64,
    /// Any warnings or issues encountered
    pub warnings: Vec<String>,
    /// Whether replay completed successfully
    pub success: bool,
}

impl ReplayManager {
    /// Create a new replay manager
    pub fn new(client: RestClient) -> Self {
        Self {
            client,
            options: ReplayOptions::default(),
        }
    }
    
    /// Create replay manager with custom options
    pub fn with_options(client: RestClient, options: ReplayOptions) -> Self {
        Self { client, options }
    }
    
    /// Replay messages for a specific channel and rebuild state
    pub async fn replay_channel(&self, channel_name: &str) -> AblyResult<ReplayResult> {
        info!("Starting channel replay for: {}", channel_name);
        let start_time = SystemTime::now();
        
        let mut result = ReplayResult {
            messages_processed: 0,
            presence_processed: 0,
            final_state: HashMap::new(),
            presence_state: HashMap::new(),
            duration_ms: 0,
            warnings: Vec::new(),
            success: false,
        };
        
        // Calculate time range for history query
        let end_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        let start_time_query = end_time - (self.options.time_range_seconds * 1000) as i64;
        
        // Get message history
        let messages_result = self
            .client
            .channel(channel_name)
            .history()
            .limit(self.options.max_messages as u32)
            .start(start_time_query)
            .end(end_time)
            .execute()
            .await;
            
        let messages = match messages_result {
            Ok(paginated) => paginated.items,
            Err(e) => {
                error!("Failed to fetch message history: {}", e);
                return Err(e);
            }
        };
        
        debug!("Retrieved {} messages for replay", messages.len());
        result.messages_processed = messages.len();
        
        // Process messages in chronological order to rebuild state
        let mut sorted_messages = messages;
        sorted_messages.sort_by_key(|m| m.timestamp.unwrap_or(0));
        
        for message in sorted_messages {
            self.process_message_for_state(&message, &mut result.final_state)?;
        }
        
        // Get presence history if enabled
        if self.options.include_presence {
            let presence_result = self
                .client
                .channel(channel_name)
                .presence()
                .history()
                .limit(self.options.max_messages as u32)
                .start(start_time_query)
                .end(end_time)
                .execute()
                .await;
                
            match presence_result {
                Ok(paginated) => {
                    let mut presence_messages = paginated.items;
                    presence_messages.sort_by_key(|p| p.timestamp.unwrap_or(0));
                    
                    for presence in presence_messages {
                        self.process_presence_for_state(&presence, &mut result.presence_state)?;
                        result.presence_processed += 1;
                    }
                    
                    debug!("Processed {} presence messages", result.presence_processed);
                }
                Err(e) => {
                    warn!("Failed to fetch presence history: {}", e);
                    result.warnings.push(format!("Presence history unavailable: {}", e));
                }
            }
        }
        
        // Calculate duration
        result.duration_ms = start_time.elapsed().unwrap().as_millis() as u64;
        result.success = true;
        
        info!(
            "Channel replay completed: {} messages, {} presence, {}ms",
            result.messages_processed, result.presence_processed, result.duration_ms
        );
        
        Ok(result)
    }
    
    /// Process a single message to update state
    fn process_message_for_state(
        &self,
        message: &Message,
        state: &mut HashMap<String, serde_json::Value>,
    ) -> AblyResult<()> {
        if let Some(name) = &message.name {
            if let Some(data) = &message.data {
                // Store the latest value for each message name
                state.insert(name.clone(), data.clone());
                
                // Handle special state operations
                match name.as_str() {
                    "state:clear" => {
                        debug!("Processing state clear command");
                        state.clear();
                    }
                    "state:delete" => {
                        if let Some(key_to_delete) = data.as_str() {
                            debug!("Deleting key from state: {}", key_to_delete);
                            state.remove(key_to_delete);
                        }
                    }
                    "state:merge" => {
                        if let Some(obj) = data.as_object() {
                            debug!("Merging object into state: {} keys", obj.len());
                            for (key, value) in obj {
                                state.insert(key.clone(), value.clone());
                            }
                        }
                    }
                    _ => {
                        // Regular message - just store the latest value
                        debug!("Storing state for key: {}", name);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Process a presence message to update presence state
    fn process_presence_for_state(
        &self,
        presence: &PresenceMessage,
        presence_state: &mut HashMap<String, PresenceMessage>,
    ) -> AblyResult<()> {
        if let Some(client_id) = &presence.client_id {
            match presence.action.as_deref() {
                Some("enter") | Some("update") => {
                    debug!("Client {} entered/updated presence", client_id);
                    presence_state.insert(client_id.clone(), presence.clone());
                }
                Some("leave") => {
                    debug!("Client {} left presence", client_id);
                    presence_state.remove(client_id);
                }
                _ => {
                    // Unknown action - keep existing state
                    debug!("Unknown presence action: {:?}", presence.action);
                }
            }
        }
        
        Ok(())
    }
    
    /// Replay multiple channels concurrently
    pub async fn replay_channels(&self, channel_names: &[&str]) -> AblyResult<HashMap<String, ReplayResult>> {
        info!("Starting multi-channel replay for {} channels", channel_names.len());
        
        let mut results = HashMap::new();
        
        // For now, process sequentially to avoid rate limiting
        // In production, this could be parallelized with proper rate limiting
        for channel_name in channel_names {
            match self.replay_channel(channel_name).await {
                Ok(result) => {
                    results.insert(channel_name.to_string(), result);
                }
                Err(e) => {
                    error!("Failed to replay channel {}: {}", channel_name, e);
                    // Create a failed result
                    let failed_result = ReplayResult {
                        messages_processed: 0,
                        presence_processed: 0,
                        final_state: HashMap::new(),
                        presence_state: HashMap::new(),
                        duration_ms: 0,
                        warnings: vec![format!("Replay failed: {}", e)],
                        success: false,
                    };
                    results.insert(channel_name.to_string(), failed_result);
                }
            }
        }
        
        info!("Multi-channel replay completed for {} channels", results.len());
        Ok(results)
    }
    
    /// Create a snapshot of current state that can be used for recovery
    pub async fn create_snapshot(&self, channel_name: &str) -> AblyResult<StateSnapshot> {
        let replay_result = self.replay_channel(channel_name).await?;
        
        Ok(StateSnapshot {
            channel: channel_name.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
            message_state: replay_result.final_state,
            presence_state: replay_result.presence_state,
            metadata: SnapshotMetadata {
                messages_processed: replay_result.messages_processed,
                presence_processed: replay_result.presence_processed,
                replay_duration_ms: replay_result.duration_ms,
                options: self.options.clone(),
            },
        })
    }
}

/// A point-in-time snapshot of channel state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    /// Channel name
    pub channel: String,
    /// Timestamp when snapshot was created
    pub timestamp: i64,
    /// Message-based state
    pub message_state: HashMap<String, serde_json::Value>,
    /// Presence state
    pub presence_state: HashMap<String, PresenceMessage>,
    /// Metadata about the snapshot
    pub metadata: SnapshotMetadata,
}

/// Metadata about how a snapshot was created
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    /// Number of messages processed
    pub messages_processed: usize,
    /// Number of presence events processed
    pub presence_processed: usize,
    /// Time taken to create snapshot
    pub replay_duration_ms: u64,
    /// Options used for replay
    pub options: ReplayOptions,
}

impl StateSnapshot {
    /// Check if this snapshot is recent (within the given seconds)
    pub fn is_recent(&self, max_age_seconds: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        
        let age_ms = now - self.timestamp;
        age_ms < (max_age_seconds * 1000) as i64
    }
    
    /// Get a specific value from the message state
    pub fn get_state_value(&self, key: &str) -> Option<&serde_json::Value> {
        self.message_state.get(key)
    }
    
    /// Check if a client is present
    pub fn is_client_present(&self, client_id: &str) -> bool {
        self.presence_state.contains_key(client_id)
    }
    
    /// Get all present client IDs
    pub fn present_clients(&self) -> Vec<String> {
        self.presence_state.keys().cloned().collect()
    }
    
    /// Serialize snapshot to JSON for storage
    pub fn to_json(&self) -> AblyResult<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| AblyError::encoding(format!("Failed to serialize snapshot: {}", e)))
    }
    
    /// Deserialize snapshot from JSON
    pub fn from_json(json: &str) -> AblyResult<Self> {
        serde_json::from_str(json)
            .map_err(|e| AblyError::decoding(format!("Failed to deserialize snapshot: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::rest::RestClient;
    
    fn get_test_api_key() -> String {
        std::env::var("ABLY_API_KEY_SANDBOX")
            .unwrap_or_else(|_| "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA".to_string())
    }
    
    #[test]
    fn test_replay_options_default() {
        let options = ReplayOptions::default();
        assert_eq!(options.max_messages, 1000);
        assert_eq!(options.time_range_seconds, 3600);
        assert!(options.include_presence);
        assert!(!options.compress_state);
        assert_eq!(options.batch_size, 100);
        assert_eq!(options.timeout_seconds, 30);
    }
    
    #[test]
    fn test_replay_manager_creation() {
        let client = RestClient::new(get_test_api_key());
        let manager = ReplayManager::new(client);
        assert_eq!(manager.options.max_messages, 1000);
    }
    
    #[test]
    fn test_replay_manager_with_custom_options() {
        let client = RestClient::new(get_test_api_key());
        let options = ReplayOptions {
            max_messages: 500,
            time_range_seconds: 1800,
            include_presence: false,
            compress_state: true,
            batch_size: 50,
            timeout_seconds: 60,
        };
        
        let manager = ReplayManager::with_options(client, options.clone());
        assert_eq!(manager.options.max_messages, 500);
        assert_eq!(manager.options.time_range_seconds, 1800);
        assert!(!manager.options.include_presence);
        assert!(manager.options.compress_state);
    }
    
    #[test]
    fn test_state_snapshot_is_recent() {
        let snapshot = StateSnapshot {
            channel: "test".to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
            message_state: HashMap::new(),
            presence_state: HashMap::new(),
            metadata: SnapshotMetadata {
                messages_processed: 0,
                presence_processed: 0,
                replay_duration_ms: 0,
                options: ReplayOptions::default(),
            },
        };
        
        assert!(snapshot.is_recent(60)); // Should be recent within 60 seconds
    }
    
    #[test]
    fn test_state_snapshot_serialization() {
        let snapshot = StateSnapshot {
            channel: "test".to_string(),
            timestamp: 1234567890,
            message_state: {
                let mut map = HashMap::new();
                map.insert("key1".to_string(), serde_json::json!("value1"));
                map
            },
            presence_state: HashMap::new(),
            metadata: SnapshotMetadata {
                messages_processed: 10,
                presence_processed: 5,
                replay_duration_ms: 100,
                options: ReplayOptions::default(),
            },
        };
        
        let json = snapshot.to_json().unwrap();
        let restored = StateSnapshot::from_json(&json).unwrap();
        
        assert_eq!(restored.channel, "test");
        assert_eq!(restored.timestamp, 1234567890);
        assert_eq!(restored.metadata.messages_processed, 10);
        assert_eq!(restored.get_state_value("key1"), Some(&serde_json::json!("value1")));
    }
    
    // Integration tests with real API would go here
    // These require network access and the ABLY_API_KEY_SANDBOX environment variable
}