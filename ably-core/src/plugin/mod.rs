// 🔴 RED Phase: Plugin system architecture
// Extensible plugin system for custom functionality

use crate::client::rest::RestClient;
use crate::client::realtime::RealtimeClient;
use crate::error::{AblyError, AblyResult};
use crate::protocol::messages::{Message, ProtocolMessage};
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Plugin trait for extending Ably functionality
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Get plugin name
    fn name(&self) -> &str;
    
    /// Get plugin version
    fn version(&self) -> &str;
    
    /// Initialize plugin
    async fn initialize(&mut self, config: PluginConfig) -> AblyResult<()>;
    
    /// Clean up plugin resources
    async fn shutdown(&mut self) -> AblyResult<()>;
    
    /// Process incoming message (before delivery)
    async fn process_inbound(&self, message: &mut Message) -> AblyResult<()> {
        // Default: no-op
        Ok(())
    }
    
    /// Process outgoing message (before sending)
    async fn process_outbound(&self, message: &mut Message) -> AblyResult<()> {
        // Default: no-op
        Ok(())
    }
    
    /// Handle protocol message
    async fn handle_protocol(&self, message: &ProtocolMessage) -> AblyResult<()> {
        // Default: no-op
        Ok(())
    }
    
    /// Get plugin state as Any for downcasting
    fn as_any(&self) -> &dyn Any;
}

/// Plugin configuration
#[derive(Debug, Clone)]
pub struct PluginConfig {
    pub enabled: bool,
    pub settings: HashMap<String, String>,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            settings: HashMap::new(),
        }
    }
}

/// Plugin manager for coordinating plugins
pub struct PluginManager {
    plugins: Arc<RwLock<Vec<Box<dyn Plugin>>>>,
    configs: Arc<RwLock<HashMap<String, PluginConfig>>>,
}

impl PluginManager {
    /// Create new plugin manager
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(Vec::new())),
            configs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register a plugin
    pub async fn register(&self, mut plugin: Box<dyn Plugin>) -> AblyResult<()> {
        let name = plugin.name().to_string();
        
        // Get or create config
        let configs = self.configs.read().await;
        let config = configs.get(&name).cloned().unwrap_or_default();
        drop(configs);
        
        // Initialize plugin
        plugin.initialize(config.clone()).await?;
        
        // Store plugin and config
        let mut plugins = self.plugins.write().await;
        plugins.push(plugin);
        
        let mut configs = self.configs.write().await;
        configs.insert(name, config);
        
        Ok(())
    }
    
    /// Process inbound message through all plugins
    pub async fn process_inbound(&self, message: &mut Message) -> AblyResult<()> {
        let plugins = self.plugins.read().await;
        
        for plugin in plugins.iter() {
            plugin.process_inbound(message).await?;
        }
        
        Ok(())
    }
    
    /// Process outbound message through all plugins
    pub async fn process_outbound(&self, message: &mut Message) -> AblyResult<()> {
        let plugins = self.plugins.read().await;
        
        for plugin in plugins.iter() {
            plugin.process_outbound(message).await?;
        }
        
        Ok(())
    }
    
    /// Handle protocol message with all plugins
    pub async fn handle_protocol(&self, message: &ProtocolMessage) -> AblyResult<()> {
        let plugins = self.plugins.read().await;
        
        for plugin in plugins.iter() {
            plugin.handle_protocol(message).await?;
        }
        
        Ok(())
    }
    
    /// Get a plugin by name
    pub async fn get<T: Plugin + 'static>(&self, name: &str) -> Option<Arc<T>> {
        let plugins = self.plugins.read().await;
        
        for plugin in plugins.iter() {
            if plugin.name() == name {
                if let Some(typed) = plugin.as_any().downcast_ref::<T>() {
                    // Note: This is simplified - in production you'd need proper Arc handling
                    return None; // Placeholder
                }
            }
        }
        
        None
    }
    
    /// Shutdown all plugins
    pub async fn shutdown(&self) -> AblyResult<()> {
        let mut plugins = self.plugins.write().await;
        
        for plugin in plugins.iter_mut() {
            plugin.shutdown().await?;
        }
        
        plugins.clear();
        
        Ok(())
    }
}

/// Built-in logging plugin
pub struct LoggingPlugin {
    level: String,
}

impl LoggingPlugin {
    pub fn new(level: &str) -> Self {
        Self {
            level: level.to_string(),
        }
    }
}

#[async_trait]
impl Plugin for LoggingPlugin {
    fn name(&self) -> &str {
        "logging"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    async fn initialize(&mut self, config: PluginConfig) -> AblyResult<()> {
        if let Some(level) = config.settings.get("level") {
            self.level = level.clone();
        }
        Ok(())
    }
    
    async fn shutdown(&mut self) -> AblyResult<()> {
        Ok(())
    }
    
    async fn process_inbound(&self, message: &mut Message) -> AblyResult<()> {
        tracing::debug!("Inbound message: {:?}", message.name);
        Ok(())
    }
    
    async fn process_outbound(&self, message: &mut Message) -> AblyResult<()> {
        tracing::debug!("Outbound message: {:?}", message.name);
        Ok(())
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Built-in metrics plugin
pub struct MetricsPlugin {
    message_count: Arc<RwLock<u64>>,
    byte_count: Arc<RwLock<u64>>,
}

impl MetricsPlugin {
    pub fn new() -> Self {
        Self {
            message_count: Arc::new(RwLock::new(0)),
            byte_count: Arc::new(RwLock::new(0)),
        }
    }
    
    pub async fn get_message_count(&self) -> u64 {
        *self.message_count.read().await
    }
    
    pub async fn get_byte_count(&self) -> u64 {
        *self.byte_count.read().await
    }
}

#[async_trait]
impl Plugin for MetricsPlugin {
    fn name(&self) -> &str {
        "metrics"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    async fn initialize(&mut self, _config: PluginConfig) -> AblyResult<()> {
        Ok(())
    }
    
    async fn shutdown(&mut self) -> AblyResult<()> {
        Ok(())
    }
    
    async fn process_inbound(&self, message: &mut Message) -> AblyResult<()> {
        let mut count = self.message_count.write().await;
        *count += 1;
        
        if let Some(data) = &message.data {
            let size = serde_json::to_string(data).unwrap_or_default().len() as u64;
            let mut bytes = self.byte_count.write().await;
            *bytes += size;
        }
        
        Ok(())
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Built-in encryption plugin wrapper
pub struct EncryptionPlugin {
    cipher: crate::crypto::ChannelCipher,
}

impl EncryptionPlugin {
    pub fn new(cipher: crate::crypto::ChannelCipher) -> Self {
        Self { cipher }
    }
}

#[async_trait]
impl Plugin for EncryptionPlugin {
    fn name(&self) -> &str {
        "encryption"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    async fn initialize(&mut self, _config: PluginConfig) -> AblyResult<()> {
        Ok(())
    }
    
    async fn shutdown(&mut self) -> AblyResult<()> {
        Ok(())
    }
    
    async fn process_outbound(&self, message: &mut Message) -> AblyResult<()> {
        // Encrypt message data before sending
        if let Some(data) = &message.data {
            let json_str = serde_json::to_string(data)
                .map_err(|e| AblyError::unexpected(format!("Failed to serialize: {}", e)))?;
            let encrypted = self.cipher.encrypt(json_str.as_bytes())?;
            let encoded = encrypted.to_base64();
            
            message.data = Some(serde_json::Value::String(encoded));
            message.encoding = Some("utf-8/cipher+aes-128-cbc/base64".to_string());
        }
        Ok(())
    }
    
    async fn process_inbound(&self, message: &mut Message) -> AblyResult<()> {
        // Decrypt message data after receiving
        if let Some(encoding) = &message.encoding {
            if encoding.contains("cipher") {
                if let Some(serde_json::Value::String(encoded)) = &message.data {
                    use crate::crypto::EncryptedData;
                    let encrypted = EncryptedData::from_base64(encoded, encoding.clone())?;
                    let decrypted = self.cipher.decrypt(&encrypted)?;
                    let json_str = String::from_utf8(decrypted)
                        .map_err(|e| AblyError::unexpected(format!("Failed to decode UTF-8: {}", e)))?;
                    message.data = Some(serde_json::from_str(&json_str)
                        .map_err(|e| AblyError::unexpected(format!("Failed to parse JSON: {}", e)))?);
                    message.encoding = None; // Clear encoding after decryption
                }
            }
        }
        Ok(())
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Plugin hook points in client
#[async_trait]
pub trait PluginClient {
    /// Get plugin manager
    fn plugin_manager(&self) -> &PluginManager;
    
    /// Register a plugin with the client
    async fn register_plugin(&self, plugin: Box<dyn Plugin>) -> AblyResult<()> {
        self.plugin_manager().register(plugin).await
    }
}

// Tests for plugin system
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_plugin_registration() {
        let manager = PluginManager::new();
        let plugin = Box::new(LoggingPlugin::new("debug"));
        
        manager.register(plugin).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_message_processing() {
        let manager = PluginManager::new();
        let plugin = Box::new(MetricsPlugin::new());
        
        manager.register(plugin).await.unwrap();
        
        let mut message = Message {
            name: Some("test".to_string()),
            data: Some(serde_json::json!({"key": "value"})),
            ..Default::default()
        };
        
        manager.process_inbound(&mut message).await.unwrap();
        manager.process_outbound(&mut message).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_multiple_plugins() {
        let manager = PluginManager::new();
        
        manager.register(Box::new(LoggingPlugin::new("info"))).await.unwrap();
        manager.register(Box::new(MetricsPlugin::new())).await.unwrap();
        
        let mut message = Message::default();
        
        // Should process through both plugins
        manager.process_inbound(&mut message).await.unwrap();
    }
}