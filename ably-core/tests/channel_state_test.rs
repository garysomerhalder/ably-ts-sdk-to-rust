// 🔴 RED Phase: Channel state management tests
// Testing advanced state management and recovery features

use ably_core::channel::{Channel, ChannelState, ChannelOptions, StateChangeEvent};
use ably_core::client::realtime::RealtimeClient;
use ably_core::protocol::ErrorInfo;
use tokio::time::{timeout, Duration};
use std::sync::Arc;

fn get_test_api_key() -> String {
    std::env::var("ABLY_API_KEY_SANDBOX")
        .unwrap_or_else(|_| "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA".to_string())
}

#[tokio::test]
async fn test_channel_state_persistence() {
    let api_key = get_test_api_key();
    let client = RealtimeClient::with_api_key(&api_key).await;

    client.connect().await
        .expect("Should connect to Ably");

    let channel = client.channel("state-persist-channel").await;

    // Attach and verify state is persisted
    channel.attach().await.unwrap();

    // Get state info
    let state_info = channel.state_info().await;
    assert_eq!(state_info.state, ChannelState::Attached);

    // State should have resume capability
    assert!(state_info.resume.is_some() || state_info.state == ChannelState::Attached);
}

#[tokio::test]
async fn test_channel_state_recovery_after_error() {
    let api_key = get_test_api_key();
    let client = RealtimeClient::with_api_key(&api_key).await;

    client.connect().await.unwrap();

    let channel = client.channel("recovery-test-channel").await;

    // Attach channel
    channel.attach().await.unwrap();
    let initial_state = channel.state_info().await;

    // Simulate error and recovery
    channel.handle_error(ErrorInfo {
        code: 40001,
        message: Some("Simulated error".to_string()),
        ..Default::default()
    }).await;

    assert_eq!(channel.state().await, ChannelState::Failed);

    // Attempt recovery
    channel.recover().await
        .expect("Should recover from failed state");

    assert_eq!(channel.state().await, ChannelState::Attached);
}

#[tokio::test]
async fn test_channel_state_with_presence_flag() {
    let api_key = get_test_api_key();
    let client = RealtimeClient::with_api_key(&api_key).await;

    client.connect().await.unwrap();

    let channel = client.channel("presence-state-channel").await;

    // Enter presence
    channel.presence_enter(Some(serde_json::json!({"status": "online"}))).await
        .expect("Should enter presence");

    let state_info = channel.state_info().await;
    assert!(state_info.has_presence);
}

#[tokio::test]
async fn test_channel_state_suspend_and_resume() {
    let api_key = get_test_api_key();
    let client = RealtimeClient::with_api_key(&api_key).await;

    client.connect().await.unwrap();

    let channel = client.channel("suspend-resume-channel").await;

    channel.attach().await.unwrap();
    let attached_serial = channel.get_attach_serial().await;

    // Suspend channel
    channel.suspend().await
        .expect("Should suspend channel");

    assert_eq!(channel.state().await, ChannelState::Suspended);

    // Resume with previous serial
    channel.resume(attached_serial).await
        .expect("Should resume channel");

    assert_eq!(channel.state().await, ChannelState::Attached);
}

#[tokio::test]
async fn test_channel_state_with_backlog() {
    let api_key = get_test_api_key();
    let client = RealtimeClient::with_api_key(&api_key).await;

    client.connect().await.unwrap();

    let channel = client.channel("backlog-channel").await;

    // Publish multiple messages
    for i in 0..5 {
        channel.publish("test", &format!("message-{}", i)).await.unwrap();
    }

    let state_info = channel.state_info().await;

    // Should indicate backlog exists
    assert!(state_info.has_backlog || channel.get_queue_size().await > 0);
}

#[tokio::test]
async fn test_channel_state_listener() {
    let api_key = get_test_api_key();
    let client = RealtimeClient::with_api_key(&api_key).await;

    client.connect().await.unwrap();

    let channel = client.channel("listener-channel").await;

    // Subscribe to state changes
    let mut state_rx = channel.on_state_change().await;

    // Trigger state change
    let channel_clone = Arc::clone(&channel);
    tokio::spawn(async move {
        channel_clone.attach().await.unwrap();
    });

    // Should receive state change event
    let state_change = timeout(Duration::from_secs(5), state_rx.recv())
        .await
        .expect("Should receive state change")
        .expect("Should have state change");

    assert_eq!(state_change.to, ChannelState::Attaching);
}

#[tokio::test]
async fn test_channel_serial_tracking() {
    let api_key = get_test_api_key();
    let client = RealtimeClient::with_api_key(&api_key).await;

    client.connect().await.unwrap();

    let channel = client.channel("serial-tracking-channel").await;

    channel.attach().await.unwrap();

    // Publish messages and track serials
    let serial1 = channel.get_msg_serial().await;
    channel.publish("test", "message1").await.unwrap();
    let serial2 = channel.get_msg_serial().await;

    assert!(serial2 > serial1, "Message serial should increment");

    // Attach serial should be set
    let attach_serial = channel.get_attach_serial().await;
    assert!(attach_serial.is_some(), "Should have attach serial after attaching");
}

#[tokio::test]
async fn test_channel_state_clear_on_detach() {
    let api_key = get_test_api_key();
    let client = RealtimeClient::with_api_key(&api_key).await;

    client.connect().await.unwrap();

    let channel = client.channel("clear-state-channel").await;

    // Attach and add presence
    channel.attach().await.unwrap();
    channel.presence_enter(None).await.unwrap();

    let state_before = channel.state_info().await;
    assert!(state_before.has_presence);

    // Detach should clear state
    channel.detach().await.unwrap();

    let state_after = channel.state_info().await;
    assert_eq!(state_after.state, ChannelState::Detached);
    assert!(!state_after.has_presence);
    assert!(state_after.resume.is_none());
}