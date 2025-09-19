//! RED Phase: MessagePack Serialization Tests
//! Tests for MessagePack encoding/decoding of Ably messages

use ably_core::protocol::{
    ProtocolMessage, Action, Message, MessagePackEncoder,
    MessagePackDecoder, ConnectionDetails
};
use serde_json;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;

#[test]
fn test_messagepack_round_trip_protocol_message() {
    let msg = ProtocolMessage {
        action: Action::Message,
        channel: Some("test-channel".to_string()),
        messages: Some(vec![Message {
            name: Some("event".to_string()),
            data: Some(serde_json::json!("test data")),
            ..Default::default()
        }]),
        ..Default::default()
    };

    // Encode to MessagePack
    let encoded = MessagePackEncoder::encode(&msg)
        .expect("Failed to encode to MessagePack");
    
    // Decode from MessagePack
    let decoded: ProtocolMessage = MessagePackDecoder::decode(&encoded)
        .expect("Failed to decode from MessagePack");
    
    assert_eq!(decoded.action, Action::Message);
    assert_eq!(decoded.channel, Some("test-channel".to_string()));
    assert!(decoded.messages.is_some());
}

#[test]
fn test_messagepack_compact_encoding() {
    let msg = ProtocolMessage {
        action: Action::Heartbeat,
        ..Default::default()
    };

    let encoded = MessagePackEncoder::encode(&msg)
        .expect("Failed to encode");
    
    // MessagePack should be more compact than JSON
    let json_encoded = serde_json::to_vec(&msg).expect("Failed to encode JSON");
    assert!(encoded.len() <= json_encoded.len(), 
            "MessagePack ({} bytes) should be smaller than JSON ({} bytes)",
            encoded.len(), json_encoded.len());
}

#[test]
fn test_messagepack_binary_data() {
    // Test encoding binary data
    let binary_data = vec![0u8, 1, 2, 3, 255];
    let msg = Message {
        name: Some("binary-msg".to_string()),
        data: Some(serde_json::json!(BASE64.encode(&binary_data))),
        encoding: Some("base64".to_string()),
        ..Default::default()
    };

    let encoded = MessagePackEncoder::encode(&msg)
        .expect("Failed to encode binary message");
    
    let decoded: Message = MessagePackDecoder::decode(&encoded)
        .expect("Failed to decode binary message");
    
    assert_eq!(decoded.name, Some("binary-msg".to_string()));
    assert_eq!(decoded.encoding, Some("base64".to_string()));
}

#[test]
fn test_messagepack_all_action_types() {
    let actions = vec![
        Action::Heartbeat,
        Action::Connect,
        Action::Message,
        Action::Presence,
        Action::Error,
    ];

    for action in actions {
        let msg = ProtocolMessage {
            action,
            ..Default::default()
        };

        let encoded = MessagePackEncoder::encode(&msg)
            .expect("Failed to encode");
        let decoded: ProtocolMessage = MessagePackDecoder::decode(&encoded)
            .expect("Failed to decode");
        
        assert_eq!(decoded.action, action, "Action {:?} round-trip failed", action);
    }
}

#[test]
fn test_messagepack_nested_structures() {
    let msg = ProtocolMessage {
        action: Action::Connected,
        connection_details: Some(ConnectionDetails {
            client_id: Some("client-123".to_string()),
            connection_key: Some("key-456".to_string()),
            max_message_size: Some(65536),
            ..Default::default()
        }),
        ..Default::default()
    };

    let encoded = MessagePackEncoder::encode(&msg)
        .expect("Failed to encode nested structure");
    let decoded: ProtocolMessage = MessagePackDecoder::decode(&encoded)
        .expect("Failed to decode nested structure");
    
    assert!(decoded.connection_details.is_some());
    let details = decoded.connection_details.unwrap();
    assert_eq!(details.client_id, Some("client-123".to_string()));
    assert_eq!(details.max_message_size, Some(65536));
}

#[test]
fn test_messagepack_error_handling() {
    // Test invalid data
    let invalid_data = vec![255, 255, 255, 255];
    let result: Result<ProtocolMessage, _> = MessagePackDecoder::decode(&invalid_data);
    assert!(result.is_err(), "Should fail to decode invalid data");
    
    // Test empty data
    let empty_data = vec![];
    let result: Result<ProtocolMessage, _> = MessagePackDecoder::decode(&empty_data);
    assert!(result.is_err(), "Should fail to decode empty data");
}