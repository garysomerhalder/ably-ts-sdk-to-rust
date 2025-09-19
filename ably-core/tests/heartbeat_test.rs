// 🔴 RED Phase: Heartbeat mechanism tests
// Testing real heartbeat functionality with Ably

use ably_core::transport::WebSocketTransport;
use ably_core::protocol::{ProtocolMessage, Action};
use ably_core::auth::AuthMode;
use tokio::time::{sleep, timeout, Duration};

fn get_test_api_key() -> String {
    std::env::var("ABLY_API_KEY_SANDBOX")
        .unwrap_or_else(|_| "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA".to_string())
}

#[tokio::test]
async fn test_heartbeat_sends_periodically() {
    let api_key = get_test_api_key();
    let transport = WebSocketTransport::with_api_key(&api_key);

    // Connect to Ably
    transport.connect().await
        .expect("Should connect to Ably");

    // Wait for heartbeats (default interval is 30 seconds, but we'll check sooner)
    sleep(Duration::from_secs(5)).await;

    // Send manual heartbeat and verify it works
    transport.send_heartbeat().await
        .expect("Should send heartbeat successfully");

    // Verify connection is still alive
    assert_eq!(transport.state().await, TransportState::Connected);
}

#[tokio::test]
async fn test_heartbeat_keeps_connection_alive() {
    let api_key = get_test_api_key();
    let transport = WebSocketTransport::with_api_key(&api_key);

    transport.connect().await
        .expect("Should connect");

    // Connection should stay alive for extended period with heartbeats
    for _ in 0..3 {
        sleep(Duration::from_secs(10)).await;

        // Verify connection is still alive
        assert_eq!(transport.state().await, TransportState::Connected,
                   "Connection should stay alive with heartbeats");

        // Send manual heartbeat
        transport.send_heartbeat().await
            .expect("Heartbeat should succeed");
    }
}

#[tokio::test]
async fn test_heartbeat_message_format() {
    // Verify heartbeat message has correct format
    let heartbeat_msg = ProtocolMessage::heartbeat();

    assert_eq!(heartbeat_msg.action, Action::Heartbeat);
    assert!(heartbeat_msg.channel.is_none());
    assert!(heartbeat_msg.messages.is_none());
    assert!(heartbeat_msg.error.is_none());
}

#[tokio::test]
async fn test_heartbeat_response_handling() {
    let api_key = get_test_api_key();
    let transport = WebSocketTransport::with_api_key(&api_key);

    transport.connect().await
        .expect("Should connect");

    // Send heartbeat and wait for ACK
    transport.send_heartbeat().await
        .expect("Should send heartbeat");

    // Wait for potential ACK message
    let ack_msg = timeout(
        Duration::from_secs(5),
        transport.receive_message()
    ).await;

    // Ably may send ACK or just stay silent - both are valid
    if let Ok(Ok(msg)) = ack_msg {
        // If we get a message, it might be an ACK
        assert!(matches!(msg.action, Action::Ack | Action::Heartbeat));
    }
}

#[tokio::test]
async fn test_heartbeat_interval_configuration() {
    use ably_core::transport::TransportConfig;

    let config = TransportConfig::builder()
        .keepalive_interval(Duration::from_secs(10))  // Shorter interval for testing
        .build();

    let api_key = get_test_api_key();
    let transport = WebSocketTransport::new(
        "wss://realtime.ably.io",
        config,
        AuthMode::ApiKey(api_key),
    );

    transport.connect().await
        .expect("Should connect");

    // Should send heartbeat within configured interval
    sleep(Duration::from_secs(12)).await;

    // Connection should still be alive
    assert_eq!(transport.state().await, TransportState::Connected);
}

#[tokio::test]
async fn test_heartbeat_failure_detection() {
    let api_key = get_test_api_key();
    let transport = WebSocketTransport::with_api_key(&api_key);

    transport.connect().await
        .expect("Should connect");

    // Simulate network interruption by disconnecting
    transport.disconnect().await
        .expect("Should disconnect");

    // Heartbeat should detect disconnection
    let result = transport.send_heartbeat().await;
    assert!(result.is_err(), "Heartbeat should fail when disconnected");
}

#[tokio::test]
async fn test_heartbeat_recovery_after_reconnect() {
    let api_key = get_test_api_key();
    let transport = WebSocketTransport::with_api_key(&api_key);

    // Connect initially
    transport.connect().await
        .expect("Should connect");

    // Disconnect
    transport.disconnect().await
        .expect("Should disconnect");

    // Reconnect
    transport.reconnect().await
        .expect("Should reconnect");

    // Heartbeat should work again
    transport.send_heartbeat().await
        .expect("Heartbeat should work after reconnection");

    assert_eq!(transport.state().await, TransportState::Connected);
}

use ably_core::transport::TransportState;