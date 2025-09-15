// MessagePack and JSON encoding/decoding for protocol messages

use crate::error::{AblyError, AblyResult};
use crate::protocol::ProtocolMessage;
use serde::{Serialize, Deserialize};

/// Encoding format for protocol messages
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EncodingFormat {
    Json,
    MessagePack,
}

/// Protocol encoder/decoder
pub struct ProtocolCodec {
    format: EncodingFormat,
}

impl ProtocolCodec {
    /// Create a new codec with specified format
    pub fn new(format: EncodingFormat) -> Self {
        Self { format }
    }

    /// Encode a protocol message
    pub fn encode<T: Serialize + ?Sized>(&self, message: &T) -> AblyResult<Vec<u8>> {
        match self.format {
            EncodingFormat::Json => {
                serde_json::to_vec(message)
                    .map_err(|e| AblyError::parse(format!("JSON encoding failed: {}", e)))
            }
            EncodingFormat::MessagePack => {
                rmp_serde::to_vec(message)
                    .map_err(|e| AblyError::parse(format!("MessagePack encoding failed: {}", e)))
            }
        }
    }

    /// Decode a protocol message
    pub fn decode<T: for<'de> Deserialize<'de>>(&self, data: &[u8]) -> AblyResult<T> {
        match self.format {
            EncodingFormat::Json => {
                serde_json::from_slice(data)
                    .map_err(|e| AblyError::parse(format!("JSON decoding failed: {}", e)))
            }
            EncodingFormat::MessagePack => {
                rmp_serde::from_slice(data)
                    .map_err(|e| AblyError::parse(format!("MessagePack decoding failed: {}", e)))
            }
        }
    }

    /// Encode multiple messages
    pub fn encode_batch(&self, messages: &[ProtocolMessage]) -> AblyResult<Vec<u8>> {
        self.encode(messages)
    }

    /// Decode multiple messages
    pub fn decode_batch(&self, data: &[u8]) -> AblyResult<Vec<ProtocolMessage>> {
        self.decode(data)
    }

    /// Get the format
    pub fn format(&self) -> EncodingFormat {
        self.format
    }

    /// Check if using binary format
    pub fn is_binary(&self) -> bool {
        self.format == EncodingFormat::MessagePack
    }
}

/// Data encoding for message payloads
pub mod data_encoding {
    use base64::Engine;
    use serde_json::Value;
    use std::collections::HashMap;

    /// Encoding types
    pub const UTF8: &str = "utf-8";
    pub const JSON: &str = "json";
    pub const BASE64: &str = "base64";
    pub const CIPHER_AES128: &str = "cipher+aes-128-cbc";
    pub const CIPHER_AES256: &str = "cipher+aes-256-cbc";

    /// Encode data based on encoding string
    pub fn encode_data(data: &Value, encoding: &str) -> Result<Vec<u8>, String> {
        let encodings: Vec<&str> = encoding.split('/').collect();
        let mut result = serde_json::to_vec(data)
            .map_err(|e| format!("Failed to serialize data: {}", e))?;

        for enc in encodings {
            result = match enc {
                UTF8 => result, // Already UTF-8
                JSON => result, // Already JSON
                BASE64 => {
                    let engine = base64::engine::general_purpose::STANDARD;
                    engine.encode(&result).into_bytes()
                }
                enc if enc.starts_with("cipher+") => {
                    // Cipher encoding would be implemented here
                    return Err(format!("Cipher encoding not yet implemented: {}", enc));
                }
                _ => {
                    return Err(format!("Unknown encoding: {}", enc));
                }
            };
        }

        Ok(result)
    }

    /// Decode data based on encoding string
    pub fn decode_data(data: &[u8], encoding: &str) -> Result<Value, String> {
        let encodings: Vec<&str> = encoding.split('/').rev().collect();
        let mut result = data.to_vec();

        for enc in encodings {
            result = match enc {
                UTF8 => result, // Already UTF-8
                JSON => result, // Will parse as JSON at the end
                BASE64 => {
                    let engine = base64::engine::general_purpose::STANDARD;
                    engine.decode(&result)
                        .map_err(|e| format!("Base64 decoding failed: {}", e))?
                }
                enc if enc.starts_with("cipher+") => {
                    // Cipher decoding would be implemented here
                    return Err(format!("Cipher decoding not yet implemented: {}", enc));
                }
                _ => {
                    return Err(format!("Unknown encoding: {}", enc));
                }
            };
        }

        // Parse as JSON
        serde_json::from_slice(&result)
            .map_err(|e| format!("JSON parsing failed: {}", e))
    }

    /// Check if encoding includes encryption
    pub fn is_encrypted(encoding: &str) -> bool {
        encoding.contains("cipher+")
    }

    /// Get required key size for cipher
    pub fn get_key_size(encoding: &str) -> Option<usize> {
        if encoding.contains("aes-128") {
            Some(16)
        } else if encoding.contains("aes-256") {
            Some(32)
        } else {
            None
        }
    }
}

/// Protocol version negotiation
pub struct ProtocolVersion {
    pub major: u8,
    pub minor: u8,
}

impl ProtocolVersion {
    /// Current protocol version
    pub const CURRENT: Self = Self { major: 3, minor: 0 };

    /// Check if version is supported
    pub fn is_supported(&self) -> bool {
        self.major == 3 && self.minor <= 0
    }

    /// Format as string for connection parameters
    pub fn to_string(&self) -> String {
        format!("{}", self.major)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{Action, Message};
    use serde_json::json;

    #[test]
    fn test_json_encoding() {
        let codec = ProtocolCodec::new(EncodingFormat::Json);
        let msg = ProtocolMessage {
            action: Action::Heartbeat,
            ..Default::default()
        };

        let encoded = codec.encode(&msg).unwrap();
        let decoded: ProtocolMessage = codec.decode(&encoded).unwrap();
        
        assert_eq!(decoded.action, Action::Heartbeat);
    }

    #[test]
    fn test_msgpack_encoding() {
        let codec = ProtocolCodec::new(EncodingFormat::MessagePack);
        let msg = ProtocolMessage {
            action: Action::Message,
            channel: Some("test".to_string()),
            ..Default::default()
        };

        let encoded = codec.encode(&msg).unwrap();
        let decoded: ProtocolMessage = codec.decode(&encoded).unwrap();
        
        assert_eq!(decoded.action, Action::Message);
        assert_eq!(decoded.channel, Some("test".to_string()));
    }

    #[test]
    fn test_base64_encoding() {
        use data_encoding::*;
        
        let data = json!("hello world");
        let encoded = encode_data(&data, BASE64).unwrap();
        let decoded = decode_data(&encoded, BASE64).unwrap();
        
        assert_eq!(decoded, data);
    }
}