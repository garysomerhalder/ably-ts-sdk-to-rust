// 🔴 RED Phase: Delta compression with VCDIFF support
// Following Ably protocol specification for delta message compression
// Integration-First - real VCDIFF decoding for delta message handling

use crate::error::{AblyError, AblyResult};
use crate::protocol::messages::{Message, ProtocolMessage};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub mod vcdiff;
pub mod channel_delta;

pub use vcdiff::VcdiffDecoder;
pub use channel_delta::ChannelDeltaHandler;

/// Plugin interface for delta compression
pub trait DeltaDecoder: Send + Sync + std::fmt::Debug {
    /// Decode delta data using the provided source as base
    fn decode(&self, delta: &[u8], source: &[u8]) -> AblyResult<Vec<u8>>;
}

/// Main delta compression plugin for Ably channels
#[derive(Debug)]
pub struct DeltaPlugin {
    decoder: Box<dyn DeltaDecoder>,
    decode_count: Arc<Mutex<usize>>,
}

impl DeltaPlugin {
    /// Create new delta plugin with decoder
    pub fn new(decoder: impl DeltaDecoder + 'static) -> Self {
        Self {
            decoder: Box::new(decoder),
            decode_count: Arc::new(Mutex::new(0)),
        }
    }
    
    /// Decode delta message using the provided base payload
    pub fn decode_message(&self, delta_data: &[u8], base_payload: &[u8]) -> AblyResult<Vec<u8>> {
        let decoded = self.decoder.decode(delta_data, base_payload)?;
        
        // Track decode operations for debugging/metrics
        let mut count = self.decode_count.lock().unwrap();
        *count += 1;
        
        Ok(decoded)
    }
    
    /// Get number of decode operations performed
    pub fn decode_count(&self) -> usize {
        *self.decode_count.lock().unwrap()
    }
}

/// Context for delta message decoding
#[derive(Debug, Clone)]
pub struct DeltaContext {
    /// Channel options
    pub channel_options: HashMap<String, String>,
    /// Available plugins
    pub plugins: HashMap<String, Arc<DeltaPlugin>>,
    /// Base payload from previous message
    pub base_encoded_previous_payload: Option<Vec<u8>>,
}

impl Default for DeltaContext {
    fn default() -> Self {
        Self {
            channel_options: HashMap::new(),
            plugins: HashMap::new(),
            base_encoded_previous_payload: None,
        }
    }
}

impl DeltaContext {
    /// Create new delta context
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add a delta plugin
    pub fn add_plugin(&mut self, name: impl Into<String>, plugin: DeltaPlugin) {
        self.plugins.insert(name.into(), Arc::new(plugin));
    }
    
    /// Set base payload for delta decoding
    pub fn set_base_payload(&mut self, payload: Vec<u8>) {
        self.base_encoded_previous_payload = Some(payload);
    }
    
    /// Clear base payload
    pub fn clear_base_payload(&mut self) {
        self.base_encoded_previous_payload = None;
    }
    
    /// Check if delta decoding is available
    pub fn has_delta_support(&self) -> bool {
        self.plugins.contains_key("vcdiff")
    }
}

/// Delta message processor for handling incoming delta-encoded messages
#[derive(Debug)]
pub struct DeltaProcessor {
    context: DeltaContext,
    last_message_id: Option<String>,
    last_channel_serial: Option<String>,
    decode_failure_recovery_in_progress: bool,
}

impl DeltaProcessor {
    /// Create new delta processor
    pub fn new(context: DeltaContext) -> Self {
        Self {
            context,
            last_message_id: None,
            last_channel_serial: None,
            decode_failure_recovery_in_progress: false,
        }
    }
    
    /// Process protocol message with potential delta encoding
    pub fn process_message(&mut self, message: &ProtocolMessage) -> AblyResult<Vec<Message>> {
        // Handle messages array if present
        if let Some(messages) = &message.messages {
            if messages.is_empty() {
                return Ok(Vec::new());
            }
            
            let first_message = &messages[0];
            let last_message = &messages[messages.len() - 1];
            
            // Check for delta encoding
            if let Some(extras) = &first_message.extras {
                if let Some(delta) = extras.get("delta") {
                    return self.process_delta_message(messages, delta, message);
                }
            }
            
            // Non-delta message - update tracking
            self.last_message_id = last_message.id.clone();
            self.last_channel_serial = message.channel_serial.clone();
            
            return Ok(messages.clone());
        }
        
        Ok(Vec::new())
    }
    
    /// Process delta-encoded message
    fn process_delta_message(
        &mut self, 
        messages: &[Message], 
        delta: &Value,
        protocol_message: &ProtocolMessage
    ) -> AblyResult<Vec<Message>> {
        let first_message = &messages[0];
        
        // Extract delta information
        let delta_from = delta.get("from")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AblyError::decoding("Missing delta.from field".to_string()))?;
        
        // Check if we have the required base message
        if Some(delta_from) != self.last_message_id.as_deref() {
            let error_msg = format!(
                "Delta message decode failure - previous message not available for message \"{}\"",
                first_message.id.as_deref().unwrap_or("unknown")
            );
            return Err(AblyError::decoding(error_msg));
        }
        
        // Decode delta messages
        let mut decoded_messages = Vec::new();
        
        for message in messages {
            let decoded_message = self.decode_delta_message(message)?;
            decoded_messages.push(decoded_message);
        }
        
        // Update tracking
        let last_message = &messages[messages.len() - 1];
        self.last_message_id = last_message.id.clone();
        self.last_channel_serial = protocol_message.channel_serial.clone();
        
        Ok(decoded_messages)
    }
    
    /// Decode individual delta message
    fn decode_delta_message(&self, message: &Message) -> AblyResult<Message> {
        // Get delta plugin
        let plugin = self.context.plugins.get("vcdiff")
            .ok_or_else(|| AblyError::decoding(
                "Missing Vcdiff decoder (https://github.com/ably-forks/vcdiff-decoder)".to_string()
            ))?;
        
        // Get base payload
        let base_payload = self.context.base_encoded_previous_payload.as_ref()
            .ok_or_else(|| AblyError::decoding("No base payload available for delta decode".to_string()))?;
        
        // Extract delta data from message
        let delta_data = if let Some(data) = &message.data {
            match data {
                Value::String(s) => {
                    // Assume base64 encoded delta data
                    use base64::{Engine, engine::general_purpose::STANDARD};
                    STANDARD.decode(s)
                        .map_err(|e| AblyError::decoding(format!("Failed to decode base64 delta data: {}", e)))?
                }
                _ => {
                    return Err(AblyError::decoding("Delta data must be base64 string".to_string()));
                }
            }
        } else {
            return Err(AblyError::decoding("No delta data in message".to_string()));
        };
        
        // Decode delta
        let decoded_data = plugin.decode_message(&delta_data, base_payload)
            .map_err(|e| AblyError::decoding(format!("Vcdiff delta decode failed: {}", e)))?;
        
        // Create decoded message
        let decoded_message = Message {
            id: message.id.clone(),
            name: message.name.clone(),
            data: Some(Value::String(String::from_utf8_lossy(&decoded_data).to_string())),
            encoding: message.encoding.clone(),
            timestamp: message.timestamp,
            client_id: message.client_id.clone(),
            connection_id: message.connection_id.clone(),
            connection_key: message.connection_key.clone(),
            extras: message.extras.clone(),
        };
        
        Ok(decoded_message)
    }
    
    /// Start decode failure recovery process
    pub fn start_decode_failure_recovery(&mut self) -> bool {
        if !self.decode_failure_recovery_in_progress {
            self.decode_failure_recovery_in_progress = true;
            true
        } else {
            false
        }
    }
    
    /// Complete decode failure recovery
    pub fn complete_decode_failure_recovery(&mut self) {
        self.decode_failure_recovery_in_progress = false;
    }
    
    /// Check if decode failure recovery is in progress
    pub fn is_recovery_in_progress(&self) -> bool {
        self.decode_failure_recovery_in_progress
    }
    
    /// Get last channel serial for recovery
    pub fn last_channel_serial(&self) -> Option<&str> {
        self.last_channel_serial.as_deref()
    }
}

/// Error codes for delta compression
pub mod error_codes {
    /// Vcdiff delta decode failed
    pub const VCDIFF_DECODE_FAILED: u16 = 40018;
    /// Missing Vcdiff decoder plugin
    pub const MISSING_VCDIFF_DECODER: u16 = 40019;
    /// Browser does not support deltas (Uint8Array required)
    pub const DELTA_NOT_SUPPORTED: u16 = 40020;
    /// Browser does not support deltas
    pub const BROWSER_DELTA_NOT_SUPPORTED: u16 = 40021;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_delta_context_creation() {
        let context = DeltaContext::new();
        assert!(!context.has_delta_support());
        assert!(context.base_encoded_previous_payload.is_none());
    }
    
    #[test]
    fn test_delta_context_plugin_management() {
        let mut context = DeltaContext::new();
        let decoder = VcdiffDecoder::new();
        let plugin = DeltaPlugin::new(decoder);
        
        context.add_plugin("vcdiff", plugin);
        assert!(context.has_delta_support());
    }
    
    #[test]
    fn test_delta_processor_creation() {
        let context = DeltaContext::new();
        let processor = DeltaProcessor::new(context);
        
        assert!(!processor.is_recovery_in_progress());
        assert!(processor.last_channel_serial().is_none());
    }
    
    #[test]
    fn test_decode_failure_recovery() {
        let context = DeltaContext::new();
        let mut processor = DeltaProcessor::new(context);
        
        // Start recovery
        assert!(processor.start_decode_failure_recovery());
        assert!(processor.is_recovery_in_progress());
        
        // Cannot start recovery again
        assert!(!processor.start_decode_failure_recovery());
        
        // Complete recovery
        processor.complete_decode_failure_recovery();
        assert!(!processor.is_recovery_in_progress());
    }
}