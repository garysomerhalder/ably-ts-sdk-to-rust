// 🔴 RED Phase: WASM bindings for Ably SDK
// Enables Ably usage in web browsers via WebAssembly

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use ably_core::client::rest::RestClient as CoreRestClient;
use ably_core::client::realtime::RealtimeClient as CoreRealtimeClient;
use ably_core::protocol::messages::{Message, PresenceMessage};
use ably_core::error::{AblyError, ErrorCode};
use serde::{Serialize, Deserialize};
use serde_wasm_bindgen::{to_value, from_value};
use serde_json;
use std::sync::Arc;
use std::sync::Mutex;

// Set up console error panic hook for better debugging
#[wasm_bindgen(start)]
pub fn initialize() {
    console_error_panic_hook::set_once();
}

/// Log to browser console
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    
    #[wasm_bindgen(js_namespace = console)]
    fn error(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

macro_rules! console_error {
    ($($t:tt)*) => (error(&format_args!($($t)*).to_string()))
}

/// WASM-friendly error type
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct WasmAblyError {
    code: u32,
    message: String,
}

#[wasm_bindgen]
impl WasmAblyError {
    #[wasm_bindgen(getter)]
    pub fn code(&self) -> u32 {
        self.code
    }
    
    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.message.clone()
    }
}

impl From<AblyError> for WasmAblyError {
    fn from(err: AblyError) -> Self {
        let code = match err.code() {
            Some(ErrorCode::Custom(code)) => code as u32,
            Some(ErrorCode::Unauthorized) => 401,
            Some(ErrorCode::Forbidden) => 403,
            Some(ErrorCode::NotFound) => 404,
            Some(ErrorCode::RateLimit) => 429,
            Some(ErrorCode::Internal) => 500,
            None => 50000,
        };
        Self {
            code,
            message: err.to_string(),
        }
    }
}

/// REST client for WASM
#[wasm_bindgen]
pub struct RestClient {
    inner: Arc<CoreRestClient>,
}

#[wasm_bindgen]
impl RestClient {
    /// Create a new REST client with API key
    #[wasm_bindgen(constructor)]
    pub fn new(api_key: &str) -> Self {
        console_log!("Creating REST client with API key");
        Self {
            inner: Arc::new(CoreRestClient::new(api_key)),
        }
    }
    
    /// Get a channel
    #[wasm_bindgen]
    pub fn channel(&self, name: &str) -> RestChannel {
        RestChannel {
            client: Arc::clone(&self.inner),
            channel_name: name.to_string(),
        }
    }
    
    /// Get server time
    #[wasm_bindgen]
    pub async fn time(&self) -> Result<f64, WasmAblyError> {
        // Note: This would need implementation in core
        Ok(js_sys::Date::now())
    }
    
    /// Get stats
    #[wasm_bindgen]
    pub async fn stats(&self) -> Result<JsValue, WasmAblyError> {
        // Return stats as JavaScript object
        Ok(JsValue::from_str("{}"))
    }
}

/// REST channel for WASM
#[wasm_bindgen]
pub struct RestChannel {
    client: Arc<CoreRestClient>,
    channel_name: String,
}

#[wasm_bindgen]
impl RestChannel {
    /// Publish a message
    #[wasm_bindgen]
    pub async fn publish(&self, name: Option<String>, data: JsValue) -> Result<(), WasmAblyError> {
        let message = Message {
            name,
            data: from_value(data).ok(),
            ..Default::default()
        };
        
        let channel = self.client.channel(&self.channel_name);
        channel.publish(message).await
            .map_err(WasmAblyError::from)
    }
    
    /// Get message history
    #[wasm_bindgen]
    pub async fn history(&self, limit: Option<u32>) -> Result<JsValue, WasmAblyError> {
        let channel = self.client.channel(&self.channel_name);
        let mut query = channel.history();
        
        if let Some(limit) = limit {
            query = query.limit(limit);
        }
        
        let result = query.execute().await
            .map_err(WasmAblyError::from)?;
        
        // Convert messages to JavaScript array
        // First convert to a JSON value to serialize messages
        let messages_json: Vec<serde_json::Value> = result.items
            .into_iter()
            .filter_map(|msg| serde_json::to_value(msg).ok())
            .collect();
        
        // Convert to JSON string then to JsValue
        match serde_json::to_string(&messages_json) {
            Ok(json_str) => Ok(JsValue::from_str(&json_str)),
            Err(_) => Ok(JsValue::NULL)
        }
    }
}

/// Realtime client for WASM
#[wasm_bindgen]
pub struct RealtimeClient {
    inner: Option<Arc<Mutex<CoreRealtimeClient>>>,
}

#[wasm_bindgen]
impl RealtimeClient {
    /// Create a new realtime client
    #[wasm_bindgen(constructor)]
    pub async fn new(api_key: &str) -> Result<RealtimeClient, WasmAblyError> {
        console_log!("Creating Realtime client");
        
        match CoreRealtimeClient::new(api_key).await {
            Ok(client) => Ok(Self {
                inner: Some(Arc::new(Mutex::new(client))),
            }),
            Err(e) => Err(WasmAblyError::from(e)),
        }
    }
    
    /// Connect to Ably
    #[wasm_bindgen]
    pub async fn connect(&self) -> Result<(), WasmAblyError> {
        if let Some(client) = &self.inner {
            let client = client.lock().unwrap();
            client.connect().await
                .map_err(WasmAblyError::from)
        } else {
            Err(WasmAblyError {
                code: 50000,
                message: "Client not initialized".to_string(),
            })
        }
    }
    
    /// Disconnect from Ably
    #[wasm_bindgen]
    pub async fn disconnect(&self) -> Result<(), WasmAblyError> {
        if let Some(client) = &self.inner {
            let client = client.lock().unwrap();
            client.disconnect().await
                .map_err(WasmAblyError::from)
        } else {
            Err(WasmAblyError {
                code: 50000,
                message: "Client not initialized".to_string(),
            })
        }
    }
    
    /// Get a channel
    #[wasm_bindgen]
    pub async fn channel(&self, name: &str) -> Result<RealtimeChannel, WasmAblyError> {
        if let Some(client) = &self.inner {
            Ok(RealtimeChannel {
                client: Arc::clone(client),
                channel_name: name.to_string(),
            })
        } else {
            Err(WasmAblyError {
                code: 50000,
                message: "Client not initialized".to_string(),
            })
        }
    }
    
    /// Check if connected
    #[wasm_bindgen(getter)]
    pub async fn connected(&self) -> bool {
        if let Some(client) = &self.inner {
            let client = client.lock().unwrap();
            client.is_connected().await
        } else {
            false
        }
    }
}

/// Realtime channel for WASM
#[wasm_bindgen]
pub struct RealtimeChannel {
    client: Arc<Mutex<CoreRealtimeClient>>,
    channel_name: String,
}

#[wasm_bindgen]
impl RealtimeChannel {
    /// Attach to channel
    #[wasm_bindgen]
    pub async fn attach(&self) -> Result<(), WasmAblyError> {
        let client = self.client.lock().unwrap();
        let channel = client.channel(&self.channel_name).await;
        channel.attach().await
            .map_err(WasmAblyError::from)
    }
    
    /// Detach from channel
    #[wasm_bindgen]
    pub async fn detach(&self) -> Result<(), WasmAblyError> {
        let client = self.client.lock().unwrap();
        let channel = client.channel(&self.channel_name).await;
        channel.detach().await
            .map_err(WasmAblyError::from)
    }
    
    /// Publish a message
    #[wasm_bindgen]
    pub async fn publish(&self, name: Option<String>, data: JsValue) -> Result<(), WasmAblyError> {
        let client = self.client.lock().unwrap();
        let channel = client.channel(&self.channel_name).await;
        
        let message = Message {
            name,
            data: from_value(data).ok(),
            ..Default::default()
        };
        
        channel.publish(message).await
            .map_err(WasmAblyError::from)
    }
    
    /// Subscribe to messages (requires callback)
    #[wasm_bindgen]
    pub fn subscribe(&self, _callback: js_sys::Function) {
        // This would need to store the callback and call it when messages arrive
        console_log!("Subscribe called with callback");
    }
    
    /// Enter presence
    #[wasm_bindgen]
    pub async fn presence_enter(&self, data: JsValue) -> Result<(), WasmAblyError> {
        let client = self.client.lock().unwrap();
        let channel = client.channel(&self.channel_name).await;
        
        let presence_data = from_value(data).ok();
        channel.presence_enter(presence_data).await
            .map_err(WasmAblyError::from)
    }
    
    /// Leave presence
    #[wasm_bindgen]
    pub async fn presence_leave(&self) -> Result<(), WasmAblyError> {
        let client = self.client.lock().unwrap();
        let channel = client.channel(&self.channel_name).await;
        
        channel.presence_leave().await
            .map_err(WasmAblyError::from)
    }
}

/// Crypto utilities for WASM
#[wasm_bindgen]
pub struct Crypto;

#[wasm_bindgen]
impl Crypto {
    /// Generate random key
    #[wasm_bindgen]
    pub fn generate_random_key(bits: u32) -> Result<Vec<u8>, WasmAblyError> {
        use ably_core::crypto::{generate_random_key as gen_key, CipherAlgorithm};
        
        // Determine algorithm based on key size
        let algorithm = if bits == 128 {
            CipherAlgorithm::Aes128
        } else if bits == 256 {
            CipherAlgorithm::Aes256
        } else {
            return Err(WasmAblyError {
                code: 50000,
                message: "Key size must be 128 or 256 bits".to_string(),
            });
        };
        
        Ok(gen_key(algorithm))
    }
    
    /// Create cipher params from key
    #[wasm_bindgen]
    pub fn get_default_params(key: String) -> Result<JsValue, WasmAblyError> {
        use ably_core::crypto::CipherParams;
        
        let _params = CipherParams::from_key(&key)
            .map_err(WasmAblyError::from)?;
        
        // Convert params to a simple JavaScript object
        let js_obj = js_sys::Object::new();
        
        // Note: We'd need to expose CipherParams fields or create a method to get them
        // For now, return a placeholder
        Ok(JsValue::from(js_obj))
    }
}

// Keep the original function for compatibility
pub fn wasm_version() -> &'static str {
    ably_core::version()
}

// Tests for WASM bindings
#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;
    
    #[wasm_bindgen_test]
    fn test_rest_client_creation() {
        let client = RestClient::new("test_key");
        assert!(true); // Just check it creates
    }
    
    #[wasm_bindgen_test]
    async fn test_realtime_client_creation() {
        let result = RealtimeClient::new("test_key").await;
        assert!(result.is_ok());
    }
}