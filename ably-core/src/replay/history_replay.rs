// 🟡 YELLOW Phase: History replay implementation
// Allows recovery of channel state after reconnection

use crate::client::rest::Channel;
use crate::error::{AblyError, AblyResult};
use crate::protocol::messages::Message;
use std::collections::HashMap;

/// Position from which to start replay
#[derive(Debug, Clone)]
pub enum ReplayPosition {
    /// Start from a specific timestamp (milliseconds since epoch)
    Timestamp(i64),
    /// Start from a specific message serial
    Serial(String),
    /// Start from a specific channel serial
    ChannelSerial(String),
    /// Start from current time
    Now,
    /// Start from beginning of available history
    Beginning,
}

/// Options for history replay
#[derive(Debug, Clone)]
pub struct ReplayOptions {
    /// Starting position for replay
    pub start: ReplayPosition,
    /// Optional ending position
    pub end: Option<ReplayPosition>,
    /// Maximum number of messages to replay
    pub limit: Option<u32>,
    /// Direction of replay ("forwards" or "backwards")
    pub direction: Option<String>,
}

impl Default for ReplayOptions {
    fn default() -> Self {
        Self {
            start: ReplayPosition::Now,
            end: None,
            limit: Some(100),
            direction: Some("backwards".to_string()),
        }
    }
}

/// History replay handler
pub struct HistoryReplay<'a> {
    channel: &'a Channel<'a>,
    options: ReplayOptions,
}

impl<'a> HistoryReplay<'a> {
    /// Create a new history replay handler
    pub fn new(channel: &'a Channel<'a>, options: ReplayOptions) -> Self {
        Self { channel, options }
    }
    
    /// Execute the replay and return messages
    pub async fn execute(&self) -> AblyResult<Vec<Message>> {
        let mut params = HashMap::new();
        
        // Set start position
        match &self.options.start {
            ReplayPosition::Timestamp(ts) => {
                params.insert("start".to_string(), ts.to_string());
            }
            ReplayPosition::Serial(serial) => {
                params.insert("start".to_string(), format!("serial:{}", serial));
            }
            ReplayPosition::ChannelSerial(serial) => {
                params.insert("start".to_string(), format!("channel-serial:{}", serial));
            }
            ReplayPosition::Now => {
                // No start param means from now
            }
            ReplayPosition::Beginning => {
                params.insert("start".to_string(), "0".to_string());
            }
        }
        
        // Set end position if specified
        if let Some(ref end_pos) = self.options.end {
            match end_pos {
                ReplayPosition::Timestamp(ts) => {
                    params.insert("end".to_string(), ts.to_string());
                }
                ReplayPosition::Serial(serial) => {
                    params.insert("end".to_string(), format!("serial:{}", serial));
                }
                ReplayPosition::ChannelSerial(serial) => {
                    params.insert("end".to_string(), format!("channel-serial:{}", serial));
                }
                _ => {}
            }
        }
        
        // Set limit
        if let Some(limit) = self.options.limit {
            params.insert("limit".to_string(), limit.to_string());
        }
        
        // Set direction
        if let Some(ref direction) = self.options.direction {
            params.insert("direction".to_string(), direction.clone());
        }
        
        // Use channel's history method with params
        let mut history_query = self.channel.history();
        
        // Apply params to history query
        for (key, value) in params.iter() {
            match key.as_str() {
                "limit" => {
                    if let Ok(limit) = value.parse::<u32>() {
                        history_query = history_query.limit(limit);
                    }
                }
                "direction" => {
                    history_query = history_query.direction(value);
                }
                "start" => {
                    if let Ok(start) = value.parse::<i64>() {
                        history_query = history_query.start(start);
                    }
                }
                "end" => {
                    if let Ok(end) = value.parse::<i64>() {
                        history_query = history_query.end(end);
                    }
                }
                _ => {}
            }
        }
        
        // Execute query
        let result = history_query.execute().await?;
        Ok(result.items)
    }
    
    /// Execute replay with pagination support
    pub async fn execute_paginated(&self) -> AblyResult<ReplayPaginator<'a>> {
        let messages = self.execute().await?;
        Ok(ReplayPaginator {
            channel: self.channel,
            current_messages: messages,
            next_serial: None,
            has_more: false, // TODO: Parse from response headers
        })
    }
}

/// Paginator for replaying large history sets
pub struct ReplayPaginator<'a> {
    channel: &'a Channel<'a>,
    current_messages: Vec<Message>,
    next_serial: Option<String>,
    has_more: bool,
}

impl<'a> ReplayPaginator<'a> {
    /// Get current page of messages
    pub fn messages(&self) -> &[Message] {
        &self.current_messages
    }
    
    /// Check if more messages are available
    pub fn has_more(&self) -> bool {
        self.has_more
    }
    
    /// Load next page of messages
    pub async fn next(&mut self) -> AblyResult<bool> {
        if !self.has_more {
            return Ok(false);
        }
        
        if let Some(ref serial) = self.next_serial {
            let replay = HistoryReplay::new(
                self.channel,
                ReplayOptions {
                    start: ReplayPosition::Serial(serial.clone()),
                    end: None,
                    limit: Some(100),
                    direction: Some("backwards".to_string()),
                },
            );
            
            self.current_messages = replay.execute().await?;
            
            // Update next_serial from last message
            if let Some(last_msg) = self.current_messages.last() {
                self.next_serial = last_msg.serial.clone();
            } else {
                self.has_more = false;
            }
            
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

/// Channel serial tracker for replay positioning
pub struct ChannelSerialTracker {
    serials: HashMap<String, String>,
}

impl ChannelSerialTracker {
    /// Create a new serial tracker
    pub fn new() -> Self {
        Self {
            serials: HashMap::new(),
        }
    }
    
    /// Update serial for a channel
    pub fn update(&mut self, channel_name: &str, serial: String) {
        self.serials.insert(channel_name.to_string(), serial);
    }
    
    /// Get last known serial for a channel
    pub fn get(&self, channel_name: &str) -> Option<&String> {
        self.serials.get(channel_name)
    }
    
    /// Clear serial for a channel
    pub fn clear(&mut self, channel_name: &str) {
        self.serials.remove(channel_name);
    }
    
    /// Clear all serials
    pub fn clear_all(&mut self) {
        self.serials.clear();
    }
}

impl Default for ChannelSerialTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Replay state for managing recovery
#[derive(Debug, Clone)]
pub struct ReplayState {
    /// Channel name
    pub channel: String,
    /// Last seen serial
    pub last_serial: Option<String>,
    /// Last seen channel serial
    pub last_channel_serial: Option<String>,
    /// Timestamp of last message
    pub last_timestamp: Option<i64>,
    /// Number of messages replayed
    pub replayed_count: usize,
    /// Whether replay is in progress
    pub in_progress: bool,
}

impl ReplayState {
    /// Create new replay state for a channel
    pub fn new(channel: String) -> Self {
        Self {
            channel,
            last_serial: None,
            last_channel_serial: None,
            last_timestamp: None,
            replayed_count: 0,
            in_progress: false,
        }
    }
    
    /// Update state from a replayed message
    pub fn update_from_message(&mut self, message: &Message) {
        if let Some(ref serial) = message.serial {
            self.last_serial = Some(serial.clone());
        }
        
        if let Some(timestamp) = message.timestamp {
            self.last_timestamp = Some(timestamp);
        }
        
        self.replayed_count += 1;
    }
    
    /// Mark replay as started
    pub fn start(&mut self) {
        self.in_progress = true;
        self.replayed_count = 0;
    }
    
    /// Mark replay as completed
    pub fn complete(&mut self) {
        self.in_progress = false;
    }
    
    /// Reset state
    pub fn reset(&mut self) {
        self.last_serial = None;
        self.last_channel_serial = None;
        self.last_timestamp = None;
        self.replayed_count = 0;
        self.in_progress = false;
    }
}