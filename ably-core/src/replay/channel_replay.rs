// 🟡 YELLOW Phase: Channel-specific replay functionality
// Enhanced channel replay with filtering and state analysis

use super::{ReplayOptions, ReplayResult, StateSnapshot};
use crate::client::rest::RestClient;
use crate::error::{AblyError, AblyResult};
use crate::protocol::messages::Message;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BTreeMap};
use tracing::{debug, info, warn};

/// Advanced channel replay with filtering and analysis capabilities
pub struct ChannelReplay {
    client: RestClient,
    channel_name: String,
    filters: Vec<MessageFilter>,
    analyzers: Vec<StateAnalyzer>,
}

/// Filter for selecting specific messages during replay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageFilter {
    /// Filter by message name pattern
    NamePattern(String),
    /// Filter by client ID
    ClientId(String),
    /// Filter by timestamp range
    TimeRange { start: i64, end: i64 },
    /// Filter by message data content
    DataContains(String),
    /// Custom JavaScript-like filter expression
    Expression(String),
}

/// Analyzer for extracting insights from message patterns
#[derive(Debug, Clone)]
pub enum StateAnalyzer {
    /// Count messages by name
    MessageCounts,
    /// Track value changes over time
    ValueTimeline(String),
    /// Identify most active clients
    ClientActivity,
    /// Find state conflicts/overwrites
    StateConflicts,
}

/// Analysis result from channel replay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelAnalysis {
    /// Message count by name
    pub message_counts: HashMap<String, usize>,
    /// Value change timeline for tracked keys
    pub value_timelines: HashMap<String, Vec<ValueChange>>,
    /// Client activity metrics
    pub client_activity: HashMap<String, ClientMetrics>,
    /// Detected state conflicts
    pub state_conflicts: Vec<StateConflict>,
    /// Overall channel health score (0-100)
    pub health_score: u8,
}

/// A single value change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueChange {
    pub timestamp: i64,
    pub old_value: Option<serde_json::Value>,
    pub new_value: serde_json::Value,
    pub client_id: Option<String>,
}

/// Metrics for a specific client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientMetrics {
    pub message_count: usize,
    pub first_seen: i64,
    pub last_seen: i64,
    pub unique_message_names: usize,
}

/// A detected state conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateConflict {
    pub key: String,
    pub timestamp: i64,
    pub conflicting_clients: Vec<String>,
    pub values: Vec<serde_json::Value>,
}

impl ChannelReplay {
    /// Create a new channel replay instance
    pub fn new(client: RestClient, channel_name: String) -> Self {
        Self {
            client,
            channel_name,
            filters: Vec::new(),
            analyzers: Vec::new(),
        }
    }
    
    /// Add a message filter
    pub fn with_filter(mut self, filter: MessageFilter) -> Self {
        self.filters.push(filter);
        self
    }
    
    /// Add a state analyzer
    pub fn with_analyzer(mut self, analyzer: StateAnalyzer) -> Self {
        self.analyzers.push(analyzer);
        self
    }
    
    /// Execute replay with filtering and analysis
    pub async fn execute_with_analysis(&self, options: &ReplayOptions) -> AblyResult<(ReplayResult, ChannelAnalysis)> {
        info!("Starting enhanced channel replay for: {}", self.channel_name);
        
        // Get message history with time range
        let end_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        let start_time = end_time - (options.time_range_seconds * 1000) as i64;
        
        let messages_result = self
            .client
            .channel(&self.channel_name)
            .history()
            .limit(options.max_messages as u32)
            .start(start_time)
            .end(end_time)
            .execute()
            .await?;
        
        let all_messages = messages_result.items;
        debug!("Retrieved {} total messages", all_messages.len());
        
        // Apply filters
        let filtered_messages = self.apply_filters(&all_messages)?;
        debug!("Filtered to {} messages", filtered_messages.len());
        
        // Build basic replay result
        let mut replay_result = ReplayResult {
            messages_processed: filtered_messages.len(),
            presence_processed: 0,
            final_state: HashMap::new(),
            presence_state: HashMap::new(),
            duration_ms: 0,
            warnings: Vec::new(),
            success: true,
        };
        
        // Sort messages chronologically for proper state reconstruction
        let mut sorted_messages = filtered_messages;
        sorted_messages.sort_by_key(|m| m.timestamp.unwrap_or(0));
        
        // Process messages to build state
        for message in &sorted_messages {
            self.process_message_for_state(message, &mut replay_result.final_state)?;
        }
        
        // Run analyzers
        let analysis = self.run_analysis(&sorted_messages)?;
        
        info!(
            "Enhanced replay completed: {} messages processed, health score: {}",
            replay_result.messages_processed, analysis.health_score
        );
        
        Ok((replay_result, analysis))
    }
    
    /// Apply all configured filters to messages
    fn apply_filters(&self, messages: &[Message]) -> AblyResult<Vec<Message>> {
        let mut filtered = messages.to_vec();
        
        for filter in &self.filters {
            filtered = self.apply_single_filter(&filtered, filter)?;
        }
        
        Ok(filtered)
    }
    
    /// Apply a single filter to messages
    fn apply_single_filter(&self, messages: &[Message], filter: &MessageFilter) -> AblyResult<Vec<Message>> {
        let filtered: Vec<Message> = messages
            .iter()
            .filter(|msg| self.message_matches_filter(msg, filter))
            .cloned()
            .collect();
        
        debug!(
            "Filter {:?} reduced {} messages to {}",
            filter,
            messages.len(),
            filtered.len()
        );
        
        Ok(filtered)
    }
    
    /// Check if a message matches a specific filter
    fn message_matches_filter(&self, message: &Message, filter: &MessageFilter) -> bool {
        match filter {
            MessageFilter::NamePattern(pattern) => {
                if let Some(name) = &message.name {
                    name.contains(pattern)
                } else {
                    false
                }
            }
            MessageFilter::ClientId(client_id) => {
                message.client_id.as_ref() == Some(client_id)
            }
            MessageFilter::TimeRange { start, end } => {
                if let Some(timestamp) = message.timestamp {
                    timestamp >= *start && timestamp <= *end
                } else {
                    false
                }
            }
            MessageFilter::DataContains(content) => {
                if let Some(data) = &message.data {
                    data.to_string().contains(content)
                } else {
                    false
                }
            }
            MessageFilter::Expression(_expr) => {
                // For now, just return true. In a real implementation,
                // this would evaluate a JavaScript-like expression
                warn!("Expression filters not yet implemented");
                true
            }
        }
    }
    
    /// Process a message to update state (similar to base implementation)
    fn process_message_for_state(
        &self,
        message: &Message,
        state: &mut HashMap<String, serde_json::Value>,
    ) -> AblyResult<()> {
        if let Some(name) = &message.name {
            if let Some(data) = &message.data {
                state.insert(name.clone(), data.clone());
            }
        }
        Ok(())
    }
    
    /// Run all configured analyzers on the messages
    fn run_analysis(&self, messages: &[Message]) -> AblyResult<ChannelAnalysis> {
        let mut analysis = ChannelAnalysis {
            message_counts: HashMap::new(),
            value_timelines: HashMap::new(),
            client_activity: HashMap::new(),
            state_conflicts: Vec::new(),
            health_score: 100,
        };
        
        // Always run basic analysis
        self.analyze_message_counts(messages, &mut analysis);
        self.analyze_client_activity(messages, &mut analysis);
        
        // Run configured analyzers
        for analyzer in &self.analyzers {
            match analyzer {
                StateAnalyzer::MessageCounts => {
                    // Already done above
                }
                StateAnalyzer::ValueTimeline(key) => {
                    self.analyze_value_timeline(messages, key, &mut analysis);
                }
                StateAnalyzer::ClientActivity => {
                    // Already done above
                }
                StateAnalyzer::StateConflicts => {
                    self.analyze_state_conflicts(messages, &mut analysis);
                }
            }
        }
        
        // Calculate health score
        analysis.health_score = self.calculate_health_score(&analysis);
        
        Ok(analysis)
    }
    
    /// Analyze message counts by name
    fn analyze_message_counts(&self, messages: &[Message], analysis: &mut ChannelAnalysis) {
        for message in messages {
            if let Some(name) = &message.name {
                *analysis.message_counts.entry(name.clone()).or_insert(0) += 1;
            }
        }
        
        debug!("Analyzed message counts: {} unique names", analysis.message_counts.len());
    }
    
    /// Analyze client activity patterns
    fn analyze_client_activity(&self, messages: &[Message], analysis: &mut ChannelAnalysis) {
        for message in messages {
            if let Some(client_id) = &message.client_id {
                let timestamp = message.timestamp.unwrap_or(0);
                let message_name = message.name.clone().unwrap_or_default();
                
                let metrics = analysis.client_activity
                    .entry(client_id.clone())
                    .or_insert(ClientMetrics {
                        message_count: 0,
                        first_seen: timestamp,
                        last_seen: timestamp,
                        unique_message_names: 0,
                    });
                
                metrics.message_count += 1;
                metrics.first_seen = metrics.first_seen.min(timestamp);
                metrics.last_seen = metrics.last_seen.max(timestamp);
                
                // Track unique message names (simplified)
                metrics.unique_message_names = analysis.message_counts.len();
            }
        }
        
        debug!("Analyzed activity for {} clients", analysis.client_activity.len());
    }
    
    /// Analyze value changes over time for a specific key
    fn analyze_value_timeline(&self, messages: &[Message], key: &str, analysis: &mut ChannelAnalysis) {
        let mut timeline = Vec::new();
        let mut last_value: Option<serde_json::Value> = None;
        
        for message in messages {
            if message.name.as_deref() == Some(key) {
                if let Some(data) = &message.data {
                    let change = ValueChange {
                        timestamp: message.timestamp.unwrap_or(0),
                        old_value: last_value.clone(),
                        new_value: data.clone(),
                        client_id: message.client_id.clone(),
                    };
                    timeline.push(change);
                    last_value = Some(data.clone());
                }
            }
        }
        
        if !timeline.is_empty() {
            analysis.value_timelines.insert(key.to_string(), timeline);
            debug!("Analyzed timeline for key '{}': {} changes", key, analysis.value_timelines[key].len());
        }
    }
    
    /// Detect state conflicts where multiple clients update the same key simultaneously
    fn analyze_state_conflicts(&self, messages: &[Message], analysis: &mut ChannelAnalysis) {
        let mut key_updates: HashMap<String, Vec<&Message>> = HashMap::new();
        
        // Group messages by key
        for message in messages {
            if let Some(name) = &message.name {
                key_updates.entry(name.clone()).or_default().push(message);
            }
        }
        
        // Look for potential conflicts (multiple updates within short time windows)
        for (key, updates) in key_updates {
            if updates.len() > 1 {
                let mut sorted_updates = updates;
                sorted_updates.sort_by_key(|m| m.timestamp.unwrap_or(0));
                
                // Check for updates within 1 second of each other
                for window in sorted_updates.windows(2) {
                    let time_diff = window[1].timestamp.unwrap_or(0) - window[0].timestamp.unwrap_or(0);
                    
                    if time_diff < 1000 { // 1 second
                        let conflict = StateConflict {
                            key: key.clone(),
                            timestamp: window[1].timestamp.unwrap_or(0),
                            conflicting_clients: vec![
                                window[0].client_id.clone().unwrap_or_default(),
                                window[1].client_id.clone().unwrap_or_default(),
                            ],
                            values: vec![
                                window[0].data.clone().unwrap_or_default(),
                                window[1].data.clone().unwrap_or_default(),
                            ],
                        };
                        analysis.state_conflicts.push(conflict);
                    }
                }
            }
        }
        
        debug!("Detected {} potential state conflicts", analysis.state_conflicts.len());
    }
    
    /// Calculate overall channel health score based on various factors
    fn calculate_health_score(&self, analysis: &ChannelAnalysis) -> u8 {
        let mut score = 100u8;
        
        // Penalize for state conflicts
        score = score.saturating_sub((analysis.state_conflicts.len() * 10) as u8);
        
        // Penalize for too many messages from single client (possible spam)
        if let Some(max_client_activity) = analysis.client_activity.values().map(|m| m.message_count).max() {
            let total_messages: usize = analysis.client_activity.values().map(|m| m.message_count).sum();
            if max_client_activity > total_messages / 2 {
                score = score.saturating_sub(20); // One client dominates
            }
        }
        
        // Bonus for good distribution of activity
        if analysis.client_activity.len() > 1 {
            score = score.saturating_add(5);
        }
        
        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::rest::RestClient;
    use serde_json::json;
    
    fn get_test_api_key() -> String {
        std::env::var("ABLY_API_KEY_SANDBOX")
            .unwrap_or_else(|_| "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA".to_string())
    }
    
    #[test]
    fn test_channel_replay_creation() {
        let client = RestClient::new(get_test_api_key());
        let replay = ChannelReplay::new(client, "test-channel".to_string());
        assert_eq!(replay.channel_name, "test-channel");
        assert!(replay.filters.is_empty());
        assert!(replay.analyzers.is_empty());
    }
    
    #[test]
    fn test_channel_replay_with_filters() {
        let client = RestClient::new(get_test_api_key());
        let replay = ChannelReplay::new(client, "test-channel".to_string())
            .with_filter(MessageFilter::NamePattern("state:".to_string()))
            .with_filter(MessageFilter::ClientId("client-123".to_string()));
        
        assert_eq!(replay.filters.len(), 2);
    }
    
    #[test]
    fn test_channel_replay_with_analyzers() {
        let client = RestClient::new(get_test_api_key());
        let replay = ChannelReplay::new(client, "test-channel".to_string())
            .with_analyzer(StateAnalyzer::MessageCounts)
            .with_analyzer(StateAnalyzer::ValueTimeline("temperature".to_string()));
        
        assert_eq!(replay.analyzers.len(), 2);
    }
    
    #[test]
    fn test_message_filter_name_pattern() {
        let client = RestClient::new(get_test_api_key());
        let replay = ChannelReplay::new(client, "test".to_string());
        
        let message = Message {
            id: Some("msg1".to_string()),
            name: Some("state:temperature".to_string()),
            data: Some(json!(25.5)),
            client_id: Some("sensor1".to_string()),
            connection_id: None,
            connection_key: None,
            encoding: None,
            timestamp: Some(1234567890),
            extras: None,
            serial: None,
            created_at: None,
            version: None,
            action: None,
            operation: None,
        };
        
        let filter = MessageFilter::NamePattern("state:".to_string());
        assert!(replay.message_matches_filter(&message, &filter));
        
        let filter = MessageFilter::NamePattern("event:".to_string());
        assert!(!replay.message_matches_filter(&message, &filter));
    }
    
    #[test]
    fn test_message_filter_client_id() {
        let client = RestClient::new(get_test_api_key());
        let replay = ChannelReplay::new(client, "test".to_string());
        
        let message = Message {
            id: Some("msg1".to_string()),
            name: Some("temperature".to_string()),
            data: Some(json!(25.5)),
            client_id: Some("sensor1".to_string()),
            connection_id: None,
            connection_key: None,
            encoding: None,
            timestamp: Some(1234567890),
            extras: None,
            serial: None,
            created_at: None,
            version: None,
            action: None,
            operation: None,
        };
        
        let filter = MessageFilter::ClientId("sensor1".to_string());
        assert!(replay.message_matches_filter(&message, &filter));
        
        let filter = MessageFilter::ClientId("sensor2".to_string());
        assert!(!replay.message_matches_filter(&message, &filter));
    }
    
    #[test]
    fn test_message_filter_time_range() {
        let client = RestClient::new(get_test_api_key());
        let replay = ChannelReplay::new(client, "test".to_string());
        
        let message = Message {
            id: Some("msg1".to_string()),
            name: Some("temperature".to_string()),
            data: Some(json!(25.5)),
            client_id: Some("sensor1".to_string()),
            connection_id: None,
            connection_key: None,
            encoding: None,
            timestamp: Some(1234567890),
            extras: None,
            serial: None,
            created_at: None,
            version: None,
            action: None,
            operation: None,
        };
        
        let filter = MessageFilter::TimeRange {
            start: 1234567000,
            end: 1234568000,
        };
        assert!(replay.message_matches_filter(&message, &filter));
        
        let filter = MessageFilter::TimeRange {
            start: 1234560000,
            end: 1234567000,
        };
        assert!(!replay.message_matches_filter(&message, &filter));
    }
}