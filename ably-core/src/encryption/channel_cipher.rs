// 🟡 YELLOW Phase: Channel-level encryption integration
// Provides high-level interface for encrypting/decrypting channel messages

use super::{AblyCrypto, CipherParams};
use crate::error::{AblyError, AblyResult};
use crate::protocol::messages::Message;
use serde_json::Value;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

/// Channel cipher for automatic message encryption/decryption
#[derive(Debug, Clone)]
pub struct ChannelCipher {
    params: CipherParams,
}

impl ChannelCipher {
    /// Create a new channel cipher with the given parameters
    pub fn new(params: CipherParams) -> Self {
        Self { params }
    }
    
    /// Create a channel cipher from a base64-encoded key
    pub fn from_base64_key(base64_key: &str) -> AblyResult<Self> {
        let params = CipherParams::from_base64_key(base64_key)?;
        Ok(Self::new(params))
    }
    
    /// Create a channel cipher with a randomly generated key
    pub fn generate_random(key_length: Option<usize>) -> AblyResult<Self> {
        let params = CipherParams::generate_random_key(key_length)?;
        Ok(Self::new(params))
    }
    
    /// Get the cipher parameters
    pub fn params(&self) -> &CipherParams {
        &self.params
    }
    
    /// Get the encryption key as base64
    pub fn key_base64(&self) -> String {
        self.params.key_as_base64()
    }
    
    /// Get the algorithm string for this cipher
    pub fn algorithm(&self) -> String {
        self.params.algorithm_string()
    }
    
    /// Encrypt a message's data field
    /// Returns a new message with encrypted data and cipher metadata
    pub fn encrypt_message(&self, mut message: Message) -> AblyResult<Message> {
        // Only encrypt if data is present
        if let Some(data) = &message.data {
            let plaintext = match data {
                Value::String(s) => s.as_bytes().to_vec(),
                Value::Array(_) | Value::Object(_) => {
                    serde_json::to_vec(data)
                        .map_err(|e| AblyError::encoding(format!("Failed to serialize message data: {}", e)))?
                }
                Value::Number(n) => n.to_string().as_bytes().to_vec(),
                Value::Bool(b) => b.to_string().as_bytes().to_vec(),
                Value::Null => Vec::new(),
            };
            
            // Encrypt the data
            let ciphertext = AblyCrypto::encrypt(&self.params, &plaintext)?;
            
            // Encode as base64 for transmission
            let encoded_data = BASE64.encode(&ciphertext);
            
            // Update message with encrypted data
            message.data = Some(Value::String(encoded_data));
            message.encoding = Some(self.add_cipher_encoding(message.encoding.as_deref()));
        }
        
        Ok(message)
    }
    
    /// Decrypt a message's data field
    /// Returns a new message with decrypted data
    pub fn decrypt_message(&self, mut message: Message) -> AblyResult<Message> {
        // Check if message has cipher encoding
        if let Some(encoding) = &message.encoding {
            if self.has_cipher_encoding(encoding) {
                if let Some(Value::String(encrypted_data)) = &message.data {
                    // Decode from base64
                    let ciphertext = BASE64.decode(encrypted_data)
                        .map_err(|e| AblyError::decoding(format!("Failed to decode encrypted data: {}", e)))?;
                    
                    // Decrypt the data
                    let plaintext = AblyCrypto::decrypt(&self.params, &ciphertext)?;
                    
                    // Remove cipher encoding and update data
                    message.encoding = Some(self.remove_cipher_encoding(encoding));
                    message.data = Some(self.parse_decrypted_data(&plaintext, &message.encoding)?);
                }
            }
        }
        
        Ok(message)
    }
    
    /// Add cipher encoding to the existing encoding string
    fn add_cipher_encoding(&self, existing: Option<&str>) -> String {
        let cipher_encoding = format!("cipher+{}", self.algorithm());
        match existing {
            Some(enc) if !enc.is_empty() => format!("{}/{}", enc, cipher_encoding),
            _ => cipher_encoding,
        }
    }
    
    /// Check if encoding contains cipher information
    fn has_cipher_encoding(&self, encoding: &str) -> bool {
        encoding.contains("cipher+") || encoding.contains(&self.algorithm())
    }
    
    /// Remove cipher encoding from the encoding string
    fn remove_cipher_encoding(&self, encoding: &str) -> String {
        let cipher_pattern = format!("cipher+{}", self.algorithm());
        
        // Split by '/' and filter out cipher encoding
        let parts: Vec<&str> = encoding
            .split('/')
            .filter(|part| !part.contains("cipher+"))
            .collect();
        
        parts.join("/")
    }
    
    /// Parse decrypted data based on remaining encoding
    fn parse_decrypted_data(&self, data: &[u8], encoding: &Option<String>) -> AblyResult<Value> {
        match encoding.as_deref() {
            Some("json") | Some("application/json") => {
                serde_json::from_slice(data)
                    .map_err(|e| AblyError::decoding(format!("Failed to parse JSON data: {}", e)))
            }
            Some("base64") => {
                let decoded = BASE64.decode(data)
                    .map_err(|e| AblyError::decoding(format!("Failed to decode base64: {}", e)))?;
                Ok(Value::String(String::from_utf8_lossy(&decoded).to_string()))
            }
            _ => {
                // Default to string
                let text = String::from_utf8_lossy(data).to_string();
                Ok(Value::String(text))
            }
        }
    }
}

/// Channel options with encryption support
#[derive(Debug, Clone, Default)]
pub struct ChannelOptions {
    /// Cipher for message encryption/decryption
    pub cipher: Option<ChannelCipher>,
    // Other channel options can be added here
}

impl ChannelOptions {
    /// Create new channel options
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add encryption with the given cipher
    pub fn with_cipher(mut self, cipher: ChannelCipher) -> Self {
        self.cipher = Some(cipher);
        self
    }
    
    /// Add encryption with a base64-encoded key
    pub fn with_cipher_key(mut self, base64_key: &str) -> AblyResult<Self> {
        let cipher = ChannelCipher::from_base64_key(base64_key)?;
        self.cipher = Some(cipher);
        Ok(self)
    }
    
    /// Add encryption with a randomly generated key
    pub fn with_random_cipher(mut self, key_length: Option<usize>) -> AblyResult<Self> {
        let cipher = ChannelCipher::generate_random(key_length)?;
        self.cipher = Some(cipher);
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_channel_cipher_creation() {
        let cipher = ChannelCipher::generate_random(Some(256)).unwrap();
        assert_eq!(cipher.algorithm(), "aes-256-cbc");
        assert!(!cipher.key_base64().is_empty());
    }
    
    #[test]
    fn test_message_encryption_decryption() {
        let cipher = ChannelCipher::generate_random(Some(256)).unwrap();
        
        let original_message = Message {
            id: Some("test-msg-1".to_string()),
            name: Some("test".to_string()),
            data: Some(json!("Hello, encrypted world!")),
            encoding: None,
            timestamp: Some(1234567890),
            client_id: None,
            connection_id: None,
            connection_key: None,
            extras: None,
        };
        
        // Encrypt
        let encrypted = cipher.encrypt_message(original_message.clone()).unwrap();
        assert!(encrypted.encoding.is_some());
        assert!(encrypted.encoding.as_ref().unwrap().contains("cipher+aes-256-cbc"));
        
        // Data should be different (encrypted)
        assert_ne!(encrypted.data, original_message.data);
        
        // Decrypt
        let decrypted = cipher.decrypt_message(encrypted).unwrap();
        assert_eq!(decrypted.data, original_message.data);
        assert_eq!(decrypted.name, original_message.name);
        assert_eq!(decrypted.id, original_message.id);
    }
    
    #[test]
    fn test_message_with_existing_encoding() {
        let cipher = ChannelCipher::generate_random(Some(128)).unwrap();
        
        let original_message = Message {
            id: Some("test-msg-2".to_string()),
            name: Some("test".to_string()),
            data: Some(json!({"key": "value"})),
            encoding: Some("json".to_string()),
            timestamp: Some(1234567890),
            client_id: None,
            connection_id: None,
            connection_key: None,
            extras: None,
        };
        
        // Encrypt
        let encrypted = cipher.encrypt_message(original_message.clone()).unwrap();
        assert_eq!(encrypted.encoding, Some("json/cipher+aes-128-cbc".to_string()));
        
        // Decrypt
        let decrypted = cipher.decrypt_message(encrypted).unwrap();
        assert_eq!(decrypted.data, original_message.data);
        assert_eq!(decrypted.encoding, Some("json".to_string()));
    }
    
    #[test]
    fn test_channel_options() {
        let options = ChannelOptions::new()
            .with_random_cipher(Some(256))
            .unwrap();
        
        assert!(options.cipher.is_some());
        assert_eq!(options.cipher.unwrap().algorithm(), "aes-256-cbc");
    }
    
    #[test]
    fn test_cipher_encoding_manipulation() {
        let cipher = ChannelCipher::generate_random(Some(256)).unwrap();
        
        // Test adding cipher encoding
        assert_eq!(cipher.add_cipher_encoding(None), "cipher+aes-256-cbc");
        assert_eq!(
            cipher.add_cipher_encoding(Some("json")), 
            "json/cipher+aes-256-cbc"
        );
        
        // Test checking cipher encoding
        assert!(cipher.has_cipher_encoding("cipher+aes-256-cbc"));
        assert!(cipher.has_cipher_encoding("json/cipher+aes-256-cbc"));
        assert!(!cipher.has_cipher_encoding("json"));
        
        // Test removing cipher encoding
        assert_eq!(cipher.remove_cipher_encoding("cipher+aes-256-cbc"), "");
        assert_eq!(cipher.remove_cipher_encoding("json/cipher+aes-256-cbc"), "json");
    }
    
    #[test]
    fn test_different_data_types_encryption() {
        let cipher = ChannelCipher::generate_random(Some(256)).unwrap();
        
        // Test string data
        let string_msg = Message {
            id: Some("str".to_string()),
            name: Some("test".to_string()),
            data: Some(json!("plain string")),
            encoding: None,
            timestamp: Some(1234567890),
            client_id: None,
            connection_id: None,
            connection_key: None,
            extras: None,
        };
        
        let encrypted = cipher.encrypt_message(string_msg.clone()).unwrap();
        let decrypted = cipher.decrypt_message(encrypted).unwrap();
        assert_eq!(decrypted.data, string_msg.data);
        
        // Test object data
        let object_msg = Message {
            id: Some("obj".to_string()),
            name: Some("test".to_string()),
            data: Some(json!({"number": 42, "bool": true})),
            encoding: None,
            timestamp: Some(1234567890),
            client_id: None,
            connection_id: None,
            connection_key: None,
            extras: None,
        };
        
        let encrypted = cipher.encrypt_message(object_msg.clone()).unwrap();
        let decrypted = cipher.decrypt_message(encrypted).unwrap();
        assert_eq!(decrypted.data, object_msg.data);
    }
}