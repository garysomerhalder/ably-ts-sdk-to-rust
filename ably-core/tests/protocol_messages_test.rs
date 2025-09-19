//! RED Phase: Protocol Message Structure Tests
//! Tests for all 22 Ably protocol action types

use ably_core::protocol::{
    ProtocolMessage, Action, ErrorInfo, MessageFlags,
    ConnectionDetails, ChannelDetails, MessageData
};
use serde_json;

#[test]
fn test_protocol_message_serialization() {
    let msg = ProtocolMessage {
        action: Action::Message,
        channel: Some("test-channel".to_string()),
        id: Some("msg-123".to_string()),
        timestamp: Some(1234567890),
        messages: Some(vec![MessageData {
            id: Some("data-1".to_string()),
            name: Some("event.name".to_string()),
            data: Some(serde_json::json!("test data")),
            encoding: None,
            extras: None,
            timestamp: Some(1234567890),
            client_id: Some("client-1".to_string()),
            connection_id: None,
            connection_key: None,
            serial: None,
            created_at: None,
            version: None,
            action: None,
            operation: None,
        }]),
        presence: None,
        flags: None,
        connection_id: None,
        connection_key: None,
        connection_serial: None,
        msg_serial: None,
        count: None,
        error: None,
        auth: None,
        connection_details: None,
        channel_serial: None,
    };

    let json = serde_json::to_string(&msg).expect("Serialization failed");
    assert!(json.contains(r#""action":14"#)); // MESSAGE = 14
    assert!(json.contains(r#""channel":"test-channel""#));
}

#[test]
fn test_all_action_types() {
    // Test all 22 action types are defined
    let actions = vec![
        Action::Heartbeat,
        Action::Ack,
        Action::Nack,
        Action::Connect,
        Action::Connected,
        Action::Disconnect,
        Action::Disconnected,
        Action::Close,
        Action::Closed,
        Action::Error,
        Action::Attach,
        Action::Attached,
        Action::Detach,
        Action::Detached,
        Action::Presence,
        Action::Message,
        Action::Sync,
        Action::Auth,
        Action::Activate,
        Action::MessageAck,
        Action::PresenceAck,
        Action::PushAdmin,
    ];

    assert_eq!(actions.len(), 22);
    
    // Verify each action has correct numeric value
    assert_eq!(Action::Heartbeat as u8, 0);
    assert_eq!(Action::Message as u8, 14);
    assert_eq!(Action::PushAdmin as u8, 21);
}

#[test]
fn test_connection_details() {
    let details = ConnectionDetails {
        client_id: Some("client-123".to_string()),
        connection_key: Some("key-456".to_string()),
        max_message_size: Some(65536),
        max_frame_size: Some(524288),
        max_inbound_rate: Some(1000),
        connection_state_ttl: Some(120000),
        server_id: Some("server-abc".to_string()),
    };

    let json = serde_json::to_string(&details).expect("Serialization failed");
    let parsed: ConnectionDetails = serde_json::from_str(&json).expect("Deserialization failed");
    
    assert_eq!(parsed.client_id, Some("client-123".to_string()));
    assert_eq!(parsed.max_message_size, Some(65536));
}

#[test]
fn test_error_info() {
    let error = ErrorInfo {
        code: 40000,
        message: Some("Test error".to_string()),
        status_code: Some(400),
        href: Some("https://help.ably.io/error/40000".to_string()),
        request_id: Some("req-123".to_string()),
        cause: None,
    };

    let json = serde_json::to_string(&error).expect("Serialization failed");
    assert!(json.contains("40000"));
    assert!(json.contains("Test error"));
}

#[test]
fn test_message_flags() {
    assert_eq!(MessageFlags::PRESENCE as u32, 1);
    assert_eq!(MessageFlags::PUBLISH as u32, 2);
    assert_eq!(MessageFlags::SUBSCRIBE as u32, 4);
    assert_eq!(MessageFlags::PRESENCE_SUBSCRIBE as u32, 8);
    assert_eq!(MessageFlags::HAS_PRESENCE as u32, 16);
    assert_eq!(MessageFlags::HAS_BACKLOG as u32, 32);
    assert_eq!(MessageFlags::RESUMED as u32, 64);
    assert_eq!(MessageFlags::TRANSIENT as u32, 256);
    assert_eq!(MessageFlags::ATTACH_RESUME as u32, 512);
}

#[test]
fn test_channel_details() {
    let details = ChannelDetails {
        channel: "test-channel".to_string(),
        channel_serial: Some("serial-123".to_string()),
        status: None,
    };

    let json = serde_json::to_string(&details).expect("Serialization failed");
    let parsed: ChannelDetails = serde_json::from_str(&json).expect("Deserialization failed");
    
    assert_eq!(parsed.channel, "test-channel");
    assert_eq!(parsed.channel_serial, Some("serial-123".to_string()));
}

#[test]
fn test_complex_protocol_message() {
    // Test CONNECTED message with full connection details
    let msg = ProtocolMessage {
        action: Action::Connected,
        connection_id: Some("conn-123".to_string()),
        connection_key: Some("key-456".to_string()),
        connection_details: Some(ConnectionDetails {
            client_id: Some("client-789".to_string()),
            connection_key: Some("key-456".to_string()),
            max_message_size: Some(65536),
            max_frame_size: Some(524288),
            max_inbound_rate: Some(1000),
            connection_state_ttl: Some(120000),
            server_id: Some("server-xyz".to_string()),
        }),
        timestamp: Some(1234567890),
        error: None,
        channel: None,
        id: None,
        messages: None,
        presence: None,
        flags: None,
        connection_serial: None,
        msg_serial: None,
        count: None,
        auth: None,
        channel_serial: None,
    };

    let json = serde_json::to_string(&msg).expect("Serialization failed");
    let parsed: ProtocolMessage = serde_json::from_str(&json).expect("Deserialization failed");
    
    assert_eq!(parsed.action, Action::Connected);
    assert!(parsed.connection_details.is_some());
    assert_eq!(parsed.connection_id, Some("conn-123".to_string()));
}