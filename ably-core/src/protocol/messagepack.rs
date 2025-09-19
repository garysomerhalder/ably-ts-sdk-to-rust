//! MessagePack serialization support for Ably protocol messages
//! YELLOW Phase: Minimal MessagePack implementation

use super::{
    ProtocolMessage, Message, PresenceMessage,
    ConnectionDetails, ErrorInfo, AuthDetails,
};
use crate::error::{AblyError, AblyResult};
use serde::{Serialize, Deserialize};
use rmp_serde;

/// MessagePack encoder for Ably protocol types
pub struct MessagePackEncoder;

impl MessagePackEncoder {
    /// Encode any serializable type to MessagePack bytes
    pub fn encode<T: Serialize>(value: &T) -> AblyResult<Vec<u8>> {
        rmp_serde::to_vec(value)
            .map_err(|e| AblyError::EncodingError {
                encoding: "msgpack".to_string(),
                message: format!("MessagePack encoding failed: {}", e),
            })
    }

    /// Encode with named fields (more readable but larger)
    pub fn encode_named<T: Serialize>(value: &T) -> AblyResult<Vec<u8>> {
        rmp_serde::to_vec_named(value)
            .map_err(|e| AblyError::EncodingError {
                encoding: "msgpack".to_string(),
                message: format!("MessagePack named encoding failed: {}", e),
            })
    }
}

/// MessagePack decoder for Ably protocol types
pub struct MessagePackDecoder;

impl MessagePackDecoder {
    /// Decode MessagePack bytes to a deserializable type
    pub fn decode<T: for<'de> Deserialize<'de>>(data: &[u8]) -> AblyResult<T> {
        rmp_serde::from_slice(data)
            .map_err(|e| AblyError::EncodingError {
                encoding: "msgpack".to_string(),
                message: format!("MessagePack decoding failed: {}", e),
            })
    }
}

/// Extension trait for protocol messages
pub trait MessagePackExt {
    fn to_msgpack(&self) -> AblyResult<Vec<u8>>;
    fn from_msgpack(data: &[u8]) -> AblyResult<Self>
    where
        Self: Sized;
}

impl MessagePackExt for ProtocolMessage {
    fn to_msgpack(&self) -> AblyResult<Vec<u8>> {
        MessagePackEncoder::encode(self)
    }

    fn from_msgpack(data: &[u8]) -> AblyResult<Self> {
        MessagePackDecoder::decode(data)
    }
}

impl MessagePackExt for Message {
    fn to_msgpack(&self) -> AblyResult<Vec<u8>> {
        MessagePackEncoder::encode(self)
    }

    fn from_msgpack(data: &[u8]) -> AblyResult<Self> {
        MessagePackDecoder::decode(data)
    }
}

impl MessagePackExt for PresenceMessage {
    fn to_msgpack(&self) -> AblyResult<Vec<u8>> {
        MessagePackEncoder::encode(self)
    }

    fn from_msgpack(data: &[u8]) -> AblyResult<Self> {
        MessagePackDecoder::decode(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Action;

    #[test]
    fn test_basic_encoding() {
        let msg = ProtocolMessage {
            action: Action::Heartbeat,
            ..Default::default()
        };

        let encoded = msg.to_msgpack().expect("Encoding failed");
        assert!(!encoded.is_empty());
        
        let decoded = ProtocolMessage::from_msgpack(&encoded)
            .expect("Decoding failed");
        assert_eq!(decoded.action, Action::Heartbeat);
    }

    #[test]
    fn test_message_with_data() {
        let msg = Message {
            name: Some("test".to_string()),
            data: Some(serde_json::json!("hello world")),
            ..Default::default()
        };

        let encoded = MessagePackEncoder::encode(&msg)
            .expect("Encoding failed");
        let decoded: Message = MessagePackDecoder::decode(&encoded)
            .expect("Decoding failed");
        
        assert_eq!(decoded.name, Some("test".to_string()));
    }

    #[test]
    fn test_size_comparison() {
        let msg = ProtocolMessage {
            action: Action::Connected,
            connection_id: Some("conn-123".to_string()),
            connection_key: Some("key-456".to_string()),
            ..Default::default()
        };

        let msgpack = msg.to_msgpack().expect("MessagePack encoding failed");
        let json = serde_json::to_vec(&msg).expect("JSON encoding failed");
        
        // MessagePack should generally be smaller
        println!("MessagePack size: {} bytes", msgpack.len());
        println!("JSON size: {} bytes", json.len());
    }
}