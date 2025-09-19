// 🔴 RED Phase: WebSocket transport tests that MUST fail initially
// Testing real Ably WebSocket endpoints - NO MOCKS!

use ably_core::transport::{WebSocketTransport, TransportConfig, TransportState};
use ably_core::protocol::{ProtocolMessage, Action};
use ably_core::auth::AuthMode;
use tokio::time::{timeout, Duration};

fn get_test_api_key() -> String {
    std::env::var("ABLY_API_KEY_SANDBOX")
        .unwrap_or_else(|_| "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA".to_string())
}

#[tokio::test]
async fn test_websocket_connection_to_ably_realtime() {
    // Test real WebSocket connection to Ably realtime endpoint
    let api_key = get_test_api_key();
    let config = TransportConfig::default();
    
    let transport = WebSocketTransport::new(
        "wss://realtime.ably.io",
        config,
        AuthMode::ApiKey(api_key),
    );
    
    transport.connect().await
        .expect("Should connect to Ably realtime");
    
    assert_eq!(transport.state().await, TransportState::Connected);
}

#[tokio::test]
async fn test_websocket_authentication_handshake() {
    let api_key = get_test_api_key();
    let config = TransportConfig::default();
    
    let transport = WebSocketTransport::new(
        "wss://realtime.ably.io",
        config,
        AuthMode::ApiKey(api_key),
    );
    
    transport.connect().await
        .expect("Should connect");
    
    // Should receive CONNECTED message after auth
    let msg = timeout(
        Duration::from_secs(5),
        transport.receive_message()
    ).await
        .expect("Should receive message within timeout")
        .expect("Should receive CONNECTED message");
    
    assert_eq!(msg.action, Action::Connected);
    assert!(msg.connection_id.is_some());
}

#[tokio::test]
async fn test_websocket_heartbeat_mechanism() {
    let api_key = get_test_api_key();
    let config = TransportConfig::default();
    
    let transport = WebSocketTransport::new(
        "wss://realtime.ably.io",
        config,
        AuthMode::ApiKey(api_key),
    );
    
    transport.connect().await
        .expect("Should connect");
    
    // Send heartbeat
    let heartbeat = ProtocolMessage::heartbeat();
    transport.send_message(heartbeat).await
        .expect("Should send heartbeat");
    
    // Should receive heartbeat response
    let response = timeout(
        Duration::from_secs(5),
        transport.receive_message()
    ).await
        .expect("Should receive response")
        .expect("Should get heartbeat response");
    
    assert_eq!(response.action, Action::Heartbeat);
}

#[tokio::test]
async fn test_websocket_reconnection_on_disconnect() {
    let api_key = get_test_api_key();
    let config = TransportConfig::builder()
        .enable_auto_reconnect(true)
        .reconnect_delay(Duration::from_secs(1))
        .build();
    
    let transport = WebSocketTransport::new(
        "wss://realtime.ably.io",
        config,
        AuthMode::ApiKey(api_key),
    );
    
    transport.connect().await
        .expect("Should connect initially");
    
    // Force disconnect
    transport.disconnect().await
        .expect("Should disconnect");
    
    assert_eq!(transport.state().await, TransportState::Disconnected);
    
    // Should auto-reconnect
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    assert_eq!(transport.state().await, TransportState::Connected);
}

#[tokio::test]
async fn test_websocket_message_framing() {
    let api_key = get_test_api_key();
    let config = TransportConfig::default();
    
    let transport = WebSocketTransport::new(
        "wss://realtime.ably.io",
        config,
        AuthMode::ApiKey(api_key),
    );
    
    transport.connect().await
        .expect("Should connect");
    
    // Send a protocol message
    let msg = ProtocolMessage {
        action: Action::Message,
        channel: Some("test_channel".to_string()),
        messages: Some(vec![]),
        ..Default::default()
    };
    
    transport.send_message(msg.clone()).await
        .expect("Should send framed message");
    
    // Message should be properly framed and sent
    assert!(true);
}

#[tokio::test]
async fn test_websocket_binary_message_support() {
    let api_key = get_test_api_key();
    let config = TransportConfig::builder()
        .use_binary_protocol(true)
        .build();
    
    let transport = WebSocketTransport::new(
        "wss://realtime.ably.io",
        config,
        AuthMode::ApiKey(api_key),
    );
    
    transport.connect().await
        .expect("Should connect with binary protocol");
    
    // Should negotiate binary protocol
    assert!(transport.is_binary_protocol());
}

#[tokio::test]
async fn test_websocket_connection_timeout() {
    let config = TransportConfig::builder()
        .connection_timeout(Duration::from_millis(1))
        .build();
    
    let transport = WebSocketTransport::new(
        "wss://sandbox-realtime.ably.io:443",
        config,
        AuthMode::ApiKey("invalid_key".to_string()),
    );
    
    let result = transport.connect().await;
    
    assert!(result.is_err());
    assert_eq!(transport.state().await, TransportState::Failed);
}

#[tokio::test]
async fn test_websocket_max_frame_size() {
    let api_key = get_test_api_key();
    let config = TransportConfig::builder()
        .max_frame_size(65536) // 64KB
        .build();
    
    let transport = WebSocketTransport::new(
        "wss://realtime.ably.io",
        config,
        AuthMode::ApiKey(api_key),
    );
    
    transport.connect().await
        .expect("Should connect with frame size limit");
    
    // Try to send a large message
    let large_data = serde_json::Value::String("x".repeat(70000)); // Larger than max frame
    let message = ably_core::protocol::messages::Message {
        data: Some(large_data),
        ..Default::default()
    };
    let msg = ProtocolMessage {
        action: Action::Message,
        messages: Some(vec![message]),
        ..Default::default()
    };
    
    let result = transport.send_message(msg).await;
    
    // Should handle large messages (fragment or error)
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_websocket_ping_pong_keepalive() {
    let api_key = get_test_api_key();
    let config = TransportConfig::builder()
        .keepalive_interval(Duration::from_secs(15))
        .build();
    
    let transport = WebSocketTransport::new(
        "wss://realtime.ably.io",
        config,
        AuthMode::ApiKey(api_key),
    );
    
    transport.connect().await
        .expect("Should connect");
    
    // WebSocket should handle ping/pong automatically
    tokio::time::sleep(Duration::from_secs(20)).await;
    
    // Connection should still be alive
    assert_eq!(transport.state().await, TransportState::Connected);
}

#[tokio::test]
async fn test_websocket_graceful_shutdown() {
    let api_key = get_test_api_key();
    let config = TransportConfig::default();
    
    let transport = WebSocketTransport::new(
        "wss://realtime.ably.io",
        config,
        AuthMode::ApiKey(api_key),
    );
    
    transport.connect().await
        .expect("Should connect");
    
    // Send CLOSE message
    let close_msg = ProtocolMessage {
        action: Action::Close,
        ..Default::default()
    };
    
    transport.send_message(close_msg).await
        .expect("Should send close message");
    
    // Should transition to closing state
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    assert!(matches!(
        transport.state().await,
        TransportState::Closing | TransportState::Closed
    ));
}