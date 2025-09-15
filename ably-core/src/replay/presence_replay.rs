// 🟡 YELLOW Phase: Presence-specific replay functionality
// Advanced presence state reconstruction and analysis

use super::{ReplayOptions, ReplayResult};
use crate::client::rest::RestClient;
use crate::error::{AblyError, AblyResult};
use crate::protocol::messages::PresenceMessage;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BTreeMap};
use tracing::{debug, info, warn};

/// Presence replay with advanced analysis capabilities
#[derive(Debug)]
pub struct PresenceReplay {
    client: RestClient,
    channel_name: String,
    options: PresenceReplayOptions,
}

/// Options specific to presence replay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceReplayOptions {
    /// Track enter/leave patterns
    pub track_patterns: bool,
    /// Analyze session durations
    pub analyze_sessions: bool,
    /// Detect presence anomalies
    pub detect_anomalies: bool,
    /// Track concurrent user peaks
    pub track_peaks: bool,
}

impl Default for PresenceReplayOptions {
    fn default() -> Self {
        Self {
            track_patterns: true,
            analyze_sessions: true,
            detect_anomalies: false,
            track_peaks: true,
        }
    }
}

/// Comprehensive presence analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceAnalysis {
    /// Current presence state
    pub current_presence: HashMap<String, PresenceMessage>,
    /// Session information for each client
    pub client_sessions: HashMap<String, Vec<PresenceSession>>,
    /// Presence pattern analysis
    pub patterns: PresencePatterns,
    /// Detected anomalies
    pub anomalies: Vec<PresenceAnomaly>,
    /// Peak concurrent user metrics
    pub peak_metrics: PeakMetrics,
    /// Overall presence health score
    pub health_score: u8,
}

/// A single presence session for a client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceSession {
    pub client_id: String,
    pub enter_time: i64,
    pub leave_time: Option<i64>,
    pub duration_ms: Option<u64>,
    pub data_changes: Vec<DataChange>,
    pub is_complete: bool,
}

/// A data change within a presence session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataChange {
    pub timestamp: i64,
    pub old_data: Option<serde_json::Value>,
    pub new_data: Option<serde_json::Value>,
}

/// Presence patterns and statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresencePatterns {
    /// Average session duration in milliseconds
    pub avg_session_duration_ms: u64,
    /// Most active hours of the day (0-23)
    pub peak_hours: Vec<u8>,
    /// Client with longest session
    pub longest_session_client: Option<String>,
    /// Client with most sessions
    pub most_active_client: Option<String>,
    /// Common enter/leave patterns
    pub common_patterns: Vec<String>,
}

/// Detected presence anomaly
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceAnomaly {
    pub anomaly_type: AnomalyType,
    pub client_id: String,
    pub timestamp: i64,
    pub description: String,
    pub severity: AnomolySeverity,
}

/// Types of presence anomalies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalyType {
    /// Client entered without leaving previous session
    DuplicateEnter,
    /// Client left without entering
    OrphanedLeave,
    /// Unusually long session
    ExcessiveSession,
    /// Rapid enter/leave cycles
    FlappingPresence,
    /// Suspicious timing patterns
    TimingAnomaly,
}

/// Severity of anomalies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomolySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Peak presence metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeakMetrics {
    /// Maximum concurrent users
    pub max_concurrent: usize,
    /// Timestamp of peak
    pub peak_timestamp: i64,
    /// Average concurrent users
    pub avg_concurrent: f64,
    /// Concurrency over time
    pub concurrency_timeline: Vec<ConcurrencyPoint>,
}

/// A point in the concurrency timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcurrencyPoint {
    pub timestamp: i64,
    pub concurrent_count: usize,
}

impl PresenceReplay {
    /// Create a new presence replay instance
    pub fn new(client: RestClient, channel_name: String) -> Self {
        Self {
            client,
            channel_name,
            options: PresenceReplayOptions::default(),
        }
    }
    
    /// Create with custom options
    pub fn with_options(client: RestClient, channel_name: String, options: PresenceReplayOptions) -> Self {
        Self {
            client,
            channel_name,
            options,
        }
    }
    
    /// Execute presence replay with full analysis
    pub async fn execute_with_analysis(&self, replay_options: &ReplayOptions) -> AblyResult<PresenceAnalysis> {
        info!("Starting presence replay analysis for: {}", self.channel_name);
        
        // Get presence history
        let end_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        let start_time = end_time - (replay_options.time_range_seconds * 1000) as i64;
        
        let presence_result = self
            .client
            .channel(&self.channel_name)
            .presence()
            .history()
            .limit(replay_options.max_messages as u32)
            .start(start_time)
            .end(end_time)
            .execute()
            .await?;
        
        let presence_messages = presence_result.items;
        debug!("Retrieved {} presence messages", presence_messages.len());
        
        // Sort messages chronologically
        let mut sorted_messages = presence_messages;
        sorted_messages.sort_by_key(|p| p.timestamp.unwrap_or(0));
        
        // Perform comprehensive analysis
        let analysis = self.analyze_presence_messages(&sorted_messages)?;
        
        info!(
            "Presence analysis completed: {} clients, {} sessions, health score: {}",
            analysis.current_presence.len(),
            analysis.client_sessions.values().map(|sessions| sessions.len()).sum::<usize>(),
            analysis.health_score
        );
        
        Ok(analysis)
    }
    
    /// Analyze presence messages to extract insights
    fn analyze_presence_messages(&self, messages: &[PresenceMessage]) -> AblyResult<PresenceAnalysis> {
        let mut analysis = PresenceAnalysis {
            current_presence: HashMap::new(),
            client_sessions: HashMap::new(),
            patterns: PresencePatterns {
                avg_session_duration_ms: 0,
                peak_hours: Vec::new(),
                longest_session_client: None,
                most_active_client: None,
                common_patterns: Vec::new(),
            },
            anomalies: Vec::new(),
            peak_metrics: PeakMetrics {
                max_concurrent: 0,
                peak_timestamp: 0,
                avg_concurrent: 0.0,
                concurrency_timeline: Vec::new(),
            },
            health_score: 100,
        };
        
        // Track sessions and build current state
        self.build_sessions_and_state(messages, &mut analysis)?;
        
        // Analyze patterns if enabled
        if self.options.track_patterns {
            self.analyze_patterns(&mut analysis);
        }
        
        // Detect anomalies if enabled
        if self.options.detect_anomalies {
            self.detect_anomalies(messages, &mut analysis);
        }
        
        // Track peaks if enabled
        if self.options.track_peaks {
            self.analyze_peak_metrics(messages, &mut analysis);
        }
        
        // Calculate health score
        analysis.health_score = self.calculate_presence_health(&analysis);
        
        Ok(analysis)
    }
    
    /// Build sessions and current presence state
    fn build_sessions_and_state(&self, messages: &[PresenceMessage], analysis: &mut PresenceAnalysis) -> AblyResult<()> {
        let mut active_sessions: HashMap<String, PresenceSession> = HashMap::new();
        
        for message in messages {
            if let Some(client_id) = &message.client_id {
                let timestamp = message.timestamp.unwrap_or(0);
                
                match message.action.as_deref() {
                    Some("enter") => {
                        // Start new session
                        let session = PresenceSession {
                            client_id: client_id.clone(),
                            enter_time: timestamp,
                            leave_time: None,
                            duration_ms: None,
                            data_changes: Vec::new(),
                            is_complete: false,
                        };
                        
                        // Check for anomaly: entering while already present
                        if active_sessions.contains_key(client_id) {
                            if self.options.detect_anomalies {
                                analysis.anomalies.push(PresenceAnomaly {
                                    anomaly_type: AnomalyType::DuplicateEnter,
                                    client_id: client_id.clone(),
                                    timestamp,
                                    description: "Client entered while already present".to_string(),
                                    severity: AnomolySeverity::Medium,
                                });
                            }
                        }
                        
                        active_sessions.insert(client_id.clone(), session);
                        analysis.current_presence.insert(client_id.clone(), message.clone());
                    }
                    
                    Some("update") => {
                        if let Some(session) = active_sessions.get_mut(client_id) {
                            // Add data change to current session
                            let data_change = DataChange {
                                timestamp,
                                old_data: analysis.current_presence.get(client_id)
                                    .and_then(|p| p.data.clone()),
                                new_data: message.data.clone(),
                            };
                            session.data_changes.push(data_change);
                        }
                        
                        // Update current presence
                        analysis.current_presence.insert(client_id.clone(), message.clone());
                    }
                    
                    Some("leave") => {
                        if let Some(mut session) = active_sessions.remove(client_id) {
                            // Complete the session
                            session.leave_time = Some(timestamp);
                            session.duration_ms = Some((timestamp - session.enter_time) as u64);
                            session.is_complete = true;
                            
                            // Add to client sessions
                            analysis.client_sessions
                                .entry(client_id.clone())
                                .or_insert_with(Vec::new)
                                .push(session);
                        } else if self.options.detect_anomalies {
                            // Anomaly: leaving without entering
                            analysis.anomalies.push(PresenceAnomaly {
                                anomaly_type: AnomalyType::OrphanedLeave,
                                client_id: client_id.clone(),
                                timestamp,
                                description: "Client left without entering".to_string(),
                                severity: AnomolySeverity::Medium,
                            });
                        }
                        
                        // Remove from current presence
                        analysis.current_presence.remove(client_id);
                    }
                    
                    _ => {
                        debug!("Unknown presence action: {:?}", message.action);
                    }
                }
            }
        }
        
        // Handle incomplete sessions
        for (client_id, session) in active_sessions {
            analysis.client_sessions
                .entry(client_id)
                .or_insert_with(Vec::new)
                .push(session);
        }
        
        debug!("Built {} sessions for {} clients", 
               analysis.client_sessions.values().map(|s| s.len()).sum::<usize>(),
               analysis.client_sessions.len());
        
        Ok(())
    }
    
    /// Analyze presence patterns
    fn analyze_patterns(&self, analysis: &mut PresenceAnalysis) {
        let all_sessions: Vec<&PresenceSession> = analysis.client_sessions
            .values()
            .flatten()
            .collect();
        
        if all_sessions.is_empty() {
            return;
        }
        
        // Calculate average session duration
        let completed_sessions: Vec<&PresenceSession> = all_sessions
            .iter()
            .filter(|s| s.is_complete && s.duration_ms.is_some())
            .copied()
            .collect();
        
        if !completed_sessions.is_empty() {
            let total_duration: u64 = completed_sessions
                .iter()
                .map(|s| s.duration_ms.unwrap())
                .sum();
            analysis.patterns.avg_session_duration_ms = total_duration / completed_sessions.len() as u64;
        }
        
        // Find longest session
        if let Some(longest) = completed_sessions
            .iter()
            .max_by_key(|s| s.duration_ms.unwrap_or(0))
        {
            analysis.patterns.longest_session_client = Some(longest.client_id.clone());
        }
        
        // Find most active client
        if let Some((client_id, sessions)) = analysis.client_sessions
            .iter()
            .max_by_key(|(_, sessions)| sessions.len())
        {
            analysis.patterns.most_active_client = Some(client_id.clone());
        }
        
        // Analyze peak hours (simplified)
        let mut hour_counts = vec![0; 24];
        for session in &all_sessions {
            let hour = ((session.enter_time / 1000 / 3600) % 24) as usize;
            hour_counts[hour] += 1;
        }
        
        let max_hour_count = *hour_counts.iter().max().unwrap_or(&0);
        analysis.patterns.peak_hours = hour_counts
            .iter()
            .enumerate()
            .filter(|(_, &count)| count > max_hour_count / 2)
            .map(|(hour, _)| hour as u8)
            .collect();
        
        debug!("Analyzed patterns: avg duration {}ms, peak hours: {:?}",
               analysis.patterns.avg_session_duration_ms,
               analysis.patterns.peak_hours);
    }
    
    /// Detect presence anomalies
    fn detect_anomalies(&self, messages: &[PresenceMessage], analysis: &mut PresenceAnalysis) {
        // Look for rapid enter/leave cycles (flapping)
        let mut client_events: HashMap<String, Vec<&PresenceMessage>> = HashMap::new();
        
        for message in messages {
            if let Some(client_id) = &message.client_id {
                client_events.entry(client_id.clone()).or_default().push(message);
            }
        }
        
        for (client_id, events) in client_events {
            if events.len() > 10 { // Threshold for suspicious activity
                // Check for rapid cycling
                let mut enter_leave_cycles = 0;
                for window in events.windows(2) {
                    if window[0].action.as_deref() == Some("enter") && 
                       window[1].action.as_deref() == Some("leave") {
                        let time_diff = window[1].timestamp.unwrap_or(0) - window[0].timestamp.unwrap_or(0);
                        if time_diff < 5000 { // Less than 5 seconds
                            enter_leave_cycles += 1;
                        }
                    }
                }
                
                if enter_leave_cycles > 3 {
                    analysis.anomalies.push(PresenceAnomaly {
                        anomaly_type: AnomalyType::FlappingPresence,
                        client_id,
                        timestamp: events.last().unwrap().timestamp.unwrap_or(0),
                        description: format!("Rapid enter/leave cycles detected: {}", enter_leave_cycles),
                        severity: AnomolySeverity::High,
                    });
                }
            }
        }
        
        debug!("Detected {} presence anomalies", analysis.anomalies.len());
    }
    
    /// Analyze peak concurrent user metrics
    fn analyze_peak_metrics(&self, messages: &[PresenceMessage], analysis: &mut PresenceAnalysis) {
        let mut concurrent_count = 0;
        let mut max_concurrent = 0;
        let mut peak_timestamp = 0i64;
        let mut timeline = Vec::new();
        let mut total_concurrent = 0u64;
        let mut sample_count = 0u64;
        
        for message in messages {
            let timestamp = message.timestamp.unwrap_or(0);
            
            match message.action.as_deref() {
                Some("enter") => {
                    concurrent_count += 1;
                    if concurrent_count > max_concurrent {
                        max_concurrent = concurrent_count;
                        peak_timestamp = timestamp;
                    }
                }
                Some("leave") => {
                    concurrent_count = concurrent_count.saturating_sub(1);
                }
                _ => {}
            }
            
            // Sample every 1000th message or so for timeline
            if timeline.len() < 1000 || messages.len() < 1000 {
                timeline.push(ConcurrencyPoint {
                    timestamp,
                    concurrent_count,
                });
            }
            
            total_concurrent += concurrent_count as u64;
            sample_count += 1;
        }
        
        analysis.peak_metrics = PeakMetrics {
            max_concurrent,
            peak_timestamp,
            avg_concurrent: if sample_count > 0 { 
                total_concurrent as f64 / sample_count as f64 
            } else { 
                0.0 
            },
            concurrency_timeline: timeline,
        };
        
        debug!("Peak metrics: max {} users at timestamp {}", max_concurrent, peak_timestamp);
    }
    
    /// Calculate overall presence health score
    fn calculate_presence_health(&self, analysis: &PresenceAnalysis) -> u8 {
        let mut score = 100u8;
        
        // Penalize for anomalies
        for anomaly in &analysis.anomalies {
            let penalty = match anomaly.severity {
                AnomolySeverity::Low => 1,
                AnomolySeverity::Medium => 5,
                AnomolySeverity::High => 10,
                AnomolySeverity::Critical => 25,
            };
            score = score.saturating_sub(penalty);
        }
        
        // Bonus for active presence
        if analysis.current_presence.len() > 0 {
            score = score.saturating_add(5);
        }
        
        // Bonus for session completeness
        let total_sessions: usize = analysis.client_sessions.values().map(|s| s.len()).sum();
        let complete_sessions: usize = analysis.client_sessions
            .values()
            .flatten()
            .filter(|s| s.is_complete)
            .count();
        
        if total_sessions > 0 {
            let completeness_ratio = complete_sessions as f64 / total_sessions as f64;
            if completeness_ratio > 0.8 {
                score = score.saturating_add(10);
            }
        }
        
        score
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
    fn test_presence_replay_creation() {
        let client = RestClient::new(get_test_api_key());
        let replay = PresenceReplay::new(client, "test-channel".to_string());
        assert_eq!(replay.channel_name, "test-channel");
        assert!(replay.options.track_patterns);
    }
    
    #[test]
    fn test_presence_replay_options() {
        let options = PresenceReplayOptions {
            track_patterns: false,
            analyze_sessions: false,
            detect_anomalies: true,
            track_peaks: false,
        };
        
        let client = RestClient::new(get_test_api_key());
        let replay = PresenceReplay::with_options(client, "test".to_string(), options);
        
        assert!(!replay.options.track_patterns);
        assert!(!replay.options.analyze_sessions);
        assert!(replay.options.detect_anomalies);
        assert!(!replay.options.track_peaks);
    }
    
    #[test]
    fn test_presence_session_structure() {
        let session = PresenceSession {
            client_id: "test-client".to_string(),
            enter_time: 1000,
            leave_time: Some(2000),
            duration_ms: Some(1000),
            data_changes: vec![
                DataChange {
                    timestamp: 1500,
                    old_data: None,
                    new_data: Some(serde_json::json!({"status": "active"})),
                }
            ],
            is_complete: true,
        };
        
        assert_eq!(session.client_id, "test-client");
        assert_eq!(session.duration_ms, Some(1000));
        assert!(session.is_complete);
        assert_eq!(session.data_changes.len(), 1);
    }
}