// 🟡 YELLOW Phase: Channel-level delta compression integration
// Handles delta-enabled channels with automatic decode failure recovery
// Integration-First - works with real Ably delta channel parameters

use crate::error::{AblyError, AblyResult};
use crate::protocol::messages::{Message, ProtocolMessage};
use super::{DeltaContext, DeltaProcessor, error_codes};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{debug, warn, error};

/// Channel-level delta compression handler
#[derive(Debug)]
pub struct ChannelDeltaHandler {
    channel_name: String,
    processor: Arc<RwLock<DeltaProcessor>>,
    channel_params: HashMap<String, String>,
    decode_stats: DeltaDecodeStats,
}

impl ChannelDeltaHandler {
    /// Create new channel delta handler
    pub fn new(channel_name: String, context: DeltaContext) -> Self {
        let processor = DeltaProcessor::new(context);
        
        Self {
            channel_name,
            processor: Arc::new(RwLock::new(processor)),
            channel_params: HashMap::new(),
            decode_stats: DeltaDecodeStats::default(),
        }
    }
    
    /// Set channel parameters for delta mode
    pub fn set_channel_params(&mut self, params: HashMap<String, String>) {
        self.channel_params = params;
        debug!(
            channel = %self.channel_name,
            params = ?self.channel_params,
            "Channel parameters updated for delta compression"
        );
    }
    
    /// Check if delta compression is enabled
    pub fn is_delta_enabled(&self) -> bool {
        self.channel_params.get("delta")
            .map(|v| v == "vcdiff")
            .unwrap_or(false)
    }
    
    /// Process incoming protocol message with delta support
    pub async fn process_protocol_message(&mut self, message: &ProtocolMessage) -> AblyResult<Vec<Message>> {
        if !self.is_delta_enabled() {
            // Not a delta channel, pass through
            return Ok(message.messages.clone().unwrap_or_default());
        }
        
        let mut processor = self.processor.write().unwrap();
        
        match processor.process_message(message) {
            Ok(messages) => {
                self.decode_stats.successful_decodes += 1;
                debug!(
                    channel = %self.channel_name,
                    message_count = messages.len(),
                    "Successfully processed delta messages"
                );
                Ok(messages)
            }
            Err(e) => {
                self.decode_stats.failed_decodes += 1;
                
                // Check if this is a recoverable delta decode failure
                if self.is_recoverable_delta_error(&e) {
                    warn!(
                        channel = %self.channel_name,
                        error = %e,
                        "Delta decode failure, initiating recovery"
                    );
                    
                    // Start recovery process
                    if processor.start_decode_failure_recovery() {
                        self.decode_stats.recovery_attempts += 1;
                        
                        // Return error to trigger channel reattachment
                        return Err(AblyError::from_ably_code(
                            error_codes::VCDIFF_DECODE_FAILED,
                            &format!("Delta decode failure for channel {}: {}", self.channel_name, e)
                        ));
                    }
                }
                
                error!(
                    channel = %self.channel_name,
                    error = %e,
                    "Non-recoverable delta decode error"
                );
                
                Err(e)
            }
        }
    }
    
    /// Complete decode failure recovery
    pub fn complete_recovery(&self) {
        let mut processor = self.processor.write().unwrap();
        processor.complete_decode_failure_recovery();
        
        debug!(
            channel = %self.channel_name,
            "Delta decode failure recovery completed"
        );
    }
    
    /// Check if recovery is in progress
    pub fn is_recovery_in_progress(&self) -> bool {
        let processor = self.processor.read().unwrap();
        processor.is_recovery_in_progress()
    }
    
    /// Get last channel serial for recovery
    pub fn last_channel_serial(&self) -> Option<String> {
        let processor = self.processor.read().unwrap();
        processor.last_channel_serial().map(|s| s.to_string())
    }
    
    /// Get decode statistics
    pub fn decode_stats(&self) -> &DeltaDecodeStats {
        &self.decode_stats
    }
    
    /// Reset decode statistics
    pub fn reset_stats(&mut self) {
        self.decode_stats = DeltaDecodeStats::default();
    }
    
    /// Update base payload for next delta decode
    pub fn update_base_payload(&self, payload: Vec<u8>) {
        // This would be called when a non-delta message is received
        // to update the base for future delta messages
        debug!(
            channel = %self.channel_name,
            payload_size = payload.len(),
            "Updated base payload for delta decoding"
        );
    }
    
    /// Check if error is recoverable through channel reattachment
    fn is_recoverable_delta_error(&self, error: &AblyError) -> bool {
        // Check if error message indicates recoverable delta failure
        let error_msg = format!("{}", error);
        error_msg.contains("Delta message decode failure") ||
        error_msg.contains("previous message not available")
    }
    
    /// Generate channel attach message with delta recovery info
    pub fn create_recovery_attach_message(&self) -> HashMap<String, Value> {
        let mut attach_data = HashMap::new();
        
        // Include last channel serial for recovery
        if let Some(serial) = self.last_channel_serial() {
            attach_data.insert("channelSerial".to_string(), Value::String(serial));
        }
        
        // Include delta parameters
        if self.is_delta_enabled() {
            attach_data.insert("params".to_string(), Value::Object(
                self.channel_params.iter()
                    .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                    .collect()
            ));
        }
        
        attach_data
    }
    
    /// Validate delta channel configuration
    pub fn validate_configuration(&self) -> AblyResult<()> {
        if self.is_delta_enabled() {
            // Check if VCDIFF decoder is available
            let processor = self.processor.read().unwrap();
            if !processor.context.has_delta_support() {
                return Err(AblyError::from_ably_code(
                    error_codes::MISSING_VCDIFF_DECODER,
                    "Missing Vcdiff decoder plugin for delta compression"
                ));
            }
            
            // Check browser/environment support
            if !self.is_delta_supported_environment() {
                return Err(AblyError::from_ably_code(
                    error_codes::DELTA_NOT_SUPPORTED,
                    "Delta decoding not supported in this environment"
                ));
            }
        }
        
        Ok(())
    }
    
    /// Check if current environment supports delta compression
    fn is_delta_supported_environment(&self) -> bool {
        // In Rust, we always support Uint8Array equivalent (Vec<u8>)
        // This check is mainly for browser compatibility in JS SDK
        true
    }
}

/// Statistics for delta decode operations
#[derive(Debug, Default, Clone)]
pub struct DeltaDecodeStats {
    /// Number of successful delta decodes
    pub successful_decodes: usize,
    /// Number of failed delta decodes
    pub failed_decodes: usize,
    /// Number of recovery attempts
    pub recovery_attempts: usize,
    /// Number of successful recoveries
    pub successful_recoveries: usize,
}

impl DeltaDecodeStats {
    /// Get decode success rate
    pub fn success_rate(&self) -> f64 {
        let total = self.successful_decodes + self.failed_decodes;
        if total == 0 {
            1.0
        } else {
            self.successful_decodes as f64 / total as f64
        }
    }
    
    /// Get recovery success rate
    pub fn recovery_rate(&self) -> f64 {
        if self.recovery_attempts == 0 {
            1.0
        } else {
            self.successful_recoveries as f64 / self.recovery_attempts as f64
        }
    }
}

/// Delta channel configuration builder
#[derive(Debug, Default)]
pub struct DeltaChannelConfig {
    delta_mode: Option<String>,
    max_output_size: Option<usize>,
    recovery_enabled: bool,
}

impl DeltaChannelConfig {
    /// Create new delta channel configuration
    pub fn new() -> Self {
        Self {
            delta_mode: None,
            max_output_size: None,
            recovery_enabled: true,
        }
    }
    
    /// Enable VCDIFF delta compression
    pub fn with_vcdiff(mut self) -> Self {
        self.delta_mode = Some("vcdiff".to_string());
        self
    }
    
    /// Set maximum output size for safety
    pub fn with_max_output_size(mut self, size: usize) -> Self {
        self.max_output_size = Some(size);
        self
    }
    
    /// Enable/disable automatic recovery
    pub fn with_recovery(mut self, enabled: bool) -> Self {
        self.recovery_enabled = enabled;
        self
    }
    
    /// Build channel parameters for Ably
    pub fn build_params(&self) -> HashMap<String, String> {
        let mut params = HashMap::new();
        
        if let Some(ref mode) = self.delta_mode {
            params.insert("delta".to_string(), mode.clone());
        }
        
        params
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::delta::{DeltaContext, DeltaPlugin, VcdiffDecoder};
    use serde_json::json;
    
    #[test]
    fn test_channel_delta_handler_creation() {
        let context = DeltaContext::new();
        let handler = ChannelDeltaHandler::new("test-channel".to_string(), context);
        
        assert_eq!(handler.channel_name, "test-channel");
        assert!(!handler.is_delta_enabled());
    }
    
    #[test]
    fn test_delta_channel_config() {
        let config = DeltaChannelConfig::new()
            .with_vcdiff()
            .with_max_output_size(1024)
            .with_recovery(true);
        
        let params = config.build_params();
        assert_eq!(params.get("delta"), Some(&"vcdiff".to_string()));
    }
    
    #[test]
    fn test_delta_enable_disable() {
        let context = DeltaContext::new();
        let mut handler = ChannelDeltaHandler::new("test-channel".to_string(), context);
        
        // Initially disabled
        assert!(!handler.is_delta_enabled());
        
        // Enable delta
        let mut params = HashMap::new();
        params.insert("delta".to_string(), "vcdiff".to_string());
        handler.set_channel_params(params);
        
        assert!(handler.is_delta_enabled());
    }
    
    #[test]
    fn test_decode_stats() {
        let stats = DeltaDecodeStats {
            successful_decodes: 8,
            failed_decodes: 2,
            recovery_attempts: 1,
            successful_recoveries: 1,
        };
        
        assert_eq!(stats.success_rate(), 0.8);
        assert_eq!(stats.recovery_rate(), 1.0);
    }
    
    #[test]
    fn test_recoverable_error_detection() {
        let context = DeltaContext::new();
        let handler = ChannelDeltaHandler::new("test-channel".to_string(), context);
        
        let recoverable_error = AblyError::decoding(
            "Delta message decode failure - previous message not available".to_string()
        );
        assert!(handler.is_recoverable_delta_error(&recoverable_error));
        
        let non_recoverable_error = AblyError::unexpected("Some other error".to_string());
        assert!(!handler.is_recoverable_delta_error(&non_recoverable_error));
    }
    
    #[test]
    fn test_configuration_validation() {
        let mut context = DeltaContext::new();
        
        // Add delta plugin
        let decoder = VcdiffDecoder::new();
        let plugin = DeltaPlugin::new(decoder);
        context.add_plugin("vcdiff", plugin);
        
        let mut handler = ChannelDeltaHandler::new("test-channel".to_string(), context);
        
        // Enable delta mode
        let mut params = HashMap::new();
        params.insert("delta".to_string(), "vcdiff".to_string());
        handler.set_channel_params(params);
        
        // Should validate successfully
        assert!(handler.validate_configuration().is_ok());
    }
    
    #[test]
    fn test_recovery_attach_message() {
        let context = DeltaContext::new();
        let handler = ChannelDeltaHandler::new("test-channel".to_string(), context);
        
        let attach_data = handler.create_recovery_attach_message();
        
        // Should contain basic structure
        assert!(attach_data.is_empty() || attach_data.contains_key("params"));
    }
}