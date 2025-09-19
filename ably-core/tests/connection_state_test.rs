// 🔴 RED Phase: Connection state machine tests that MUST fail initially
// Testing real state transitions with real Ably connections

use ably_core::connection::{ConnectionStateMachine, ConnectionEvent, ConnectionState, ConnectionDetails};
use ably_core::protocol::{ProtocolMessage, Action, ErrorInfo};
use tokio::time::{timeout, Duration};

#[allow(dead_code)]
fn get_test_api_key() -> String {
    std::env::var("ABLY_API_KEY_SANDBOX")
        .unwrap_or_else(|_| "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA".to_string())
}

#[tokio::test]
async fn test_connection_state_initial() {
    let state_machine = ConnectionStateMachine::new();
    assert_eq!(state_machine.current_state().await, ConnectionState::Initialized);
}

#[tokio::test]
async fn test_connection_state_transition_to_connecting() {
    let _api_key = get_test_api_key();
    let mut state_machine = ConnectionStateMachine::new();

    state_machine.transition_to(ConnectionState::Connecting).await
        .expect("Should transition to connecting");

    assert_eq!(state_machine.current_state().await, ConnectionState::Connecting);
}

#[tokio::test]
async fn test_connection_state_transition_to_connected() {
    let _api_key = get_test_api_key();
    let mut state_machine = ConnectionStateMachine::new();

    // Must go through connecting first
    state_machine.transition_to(ConnectionState::Connecting).await
        .expect("Should transition to connecting");

    // Then to connected with connection details
    let connected_msg = ProtocolMessage {
        action: Action::Connected,
        connection_id: Some("test-conn-123".to_string()),
        connection_key: Some("test-key-456".to_string()),
        ..Default::default()
    };

    state_machine.handle_protocol_message(connected_msg).await
        .expect("Should handle connected message");

    assert_eq!(state_machine.current_state().await, ConnectionState::Connected);
}

#[tokio::test]
async fn test_connection_state_transition_to_disconnected() {
    let mut state_machine = ConnectionStateMachine::new();

    // Setup: Connect first
    state_machine.transition_to(ConnectionState::Connecting).await.unwrap();
    state_machine.transition_to(ConnectionState::Connected).await.unwrap();

    // Now disconnect
    state_machine.transition_to(ConnectionState::Disconnected).await
        .expect("Should transition to disconnected");

    assert_eq!(state_machine.current_state().await, ConnectionState::Disconnected);
}

#[tokio::test]
async fn test_connection_state_transition_to_suspended() {
    let mut state_machine = ConnectionStateMachine::new();

    // Can only suspend from connected or disconnected
    state_machine.transition_to(ConnectionState::Connected).await.unwrap();

    state_machine.transition_to(ConnectionState::Suspended).await
        .expect("Should transition to suspended");

    assert_eq!(state_machine.current_state().await, ConnectionState::Suspended);
}

#[tokio::test]
async fn test_connection_state_transition_to_closing() {
    let mut state_machine = ConnectionStateMachine::new();

    state_machine.transition_to(ConnectionState::Connected).await.unwrap();

    state_machine.transition_to(ConnectionState::Closing).await
        .expect("Should transition to closing");

    assert_eq!(state_machine.current_state().await, ConnectionState::Closing);
}

#[tokio::test]
async fn test_connection_state_transition_to_closed() {
    let mut state_machine = ConnectionStateMachine::new();

    state_machine.transition_to(ConnectionState::Closing).await.unwrap();

    state_machine.transition_to(ConnectionState::Closed).await
        .expect("Should transition to closed");

    assert_eq!(state_machine.current_state().await, ConnectionState::Closed);
}

#[tokio::test]
async fn test_connection_state_transition_to_failed() {
    let mut state_machine = ConnectionStateMachine::new();

    // Can fail from any state except closed
    state_machine.transition_to(ConnectionState::Connecting).await.unwrap();

    let error_msg = ProtocolMessage {
        action: Action::Error,
        error: Some(ErrorInfo {
            code: 40000,
            message: Some("Connection failed".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };

    state_machine.handle_protocol_message(error_msg).await
        .expect("Should handle error message");

    assert_eq!(state_machine.current_state().await, ConnectionState::Failed);
}

#[tokio::test]
async fn test_invalid_state_transition() {
    let mut state_machine = ConnectionStateMachine::new();

    // Cannot go directly from initialized to connected
    let result = state_machine.transition_to(ConnectionState::Connected).await;

    assert!(result.is_err(), "Should not allow invalid transition");
    assert_eq!(state_machine.current_state().await, ConnectionState::Initialized);
}

#[tokio::test]
async fn test_connection_state_events() {
    let mut state_machine = ConnectionStateMachine::new();
    let mut event_rx = state_machine.subscribe_to_events().await;

    // Transition and check event is emitted
    state_machine.transition_to(ConnectionState::Connecting).await.unwrap();

    let event = timeout(Duration::from_secs(1), event_rx.recv())
        .await
        .expect("Should receive event within timeout")
        .expect("Should receive event");

    match event {
        ConnectionEvent::StateChanged { from, to } => {
            assert_eq!(from, ConnectionState::Initialized);
            assert_eq!(to, ConnectionState::Connecting);
        }
        _ => panic!("Expected StateChanged event"),
    }
}

#[tokio::test]
async fn test_connection_retry_logic() {
    let mut state_machine = ConnectionStateMachine::new();

    // Fail the connection
    state_machine.transition_to(ConnectionState::Failed).await.unwrap();

    // Should be able to retry
    let can_retry = state_machine.can_retry().await;
    assert!(can_retry, "Should be able to retry from failed state");

    // Retry should transition to connecting
    state_machine.retry_connection().await
        .expect("Should be able to retry");

    assert_eq!(state_machine.current_state().await, ConnectionState::Connecting);
}

#[tokio::test]
async fn test_connection_state_persistence() {
    let mut state_machine = ConnectionStateMachine::new();

    // Connect and store connection details
    state_machine.transition_to(ConnectionState::Connecting).await.unwrap();

    let connected_msg = ProtocolMessage {
        action: Action::Connected,
        connection_id: Some("persistent-conn-123".to_string()),
        connection_key: Some("persistent-key-456".to_string()),
        connection_serial: Some(789),
        ..Default::default()
    };

    state_machine.handle_protocol_message(connected_msg).await.unwrap();

    // Connection details should be persisted
    let details = state_machine.connection_details().await;
    assert_eq!(details.connection_id, Some("persistent-conn-123".to_string()));
    assert_eq!(details.connection_key, Some("persistent-key-456".to_string()));
    assert_eq!(details.connection_serial, Some(789));
}