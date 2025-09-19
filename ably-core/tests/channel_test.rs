// 🔴 RED Phase: Channel attach/detach tests
// Testing real channel operations with Ably

use ably_core::channel::{Channel, ChannelState, ChannelOptions, ChannelMode};
use ably_core::client::realtime::RealtimeClient;
use ably_core::protocol::{ProtocolMessage, Action};
use tokio::time::{timeout, Duration};
use std::sync::Arc;

fn get_test_api_key() -> String {
    std::env::var("ABLY_API_KEY_SANDBOX")
        .unwrap_or_else(|_| "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA".to_string())
}

#[tokio::test]
async fn test_channel_attach() {
    let api_key = get_test_api_key();
    let client = RealtimeClient::with_api_key(&api_key).await;

    client.connect().await
        .expect("Should connect to Ably");

    let channel = client.channel("test-channel").await;

    channel.attach().await
        .expect("Should attach to channel");

    assert_eq!(channel.state().await, ChannelState::Attached);
}

#[tokio::test]
async fn test_channel_detach() {
    let api_key = get_test_api_key();
    let client = RealtimeClient::with_api_key(&api_key).await;

    client.connect().await.unwrap();

    let channel = client.channel("test-channel").await;

    channel.attach().await.unwrap();
    assert_eq!(channel.state().await, ChannelState::Attached);

    channel.detach().await
        .expect("Should detach from channel");

    assert_eq!(channel.state().await, ChannelState::Detached);
}

#[tokio::test]
async fn test_channel_auto_attach_on_publish() {
    let api_key = get_test_api_key();
    let client = RealtimeClient::with_api_key(&api_key).await;

    client.connect().await.unwrap();

    let channel = client.channel("auto-attach-channel").await;

    // Publishing should auto-attach
    channel.publish("test-event", "test-data").await
        .expect("Should publish and auto-attach");

    assert_eq!(channel.state().await, ChannelState::Attached);
}

#[tokio::test]
async fn test_channel_attach_with_options() {
    let api_key = get_test_api_key();
    let client = RealtimeClient::with_api_key(&api_key).await;

    client.connect().await.unwrap();

    let channel = client.channel_with_options("options-channel", ChannelOptions {
        params: Some(vec![("rewind", "10")]),
        modes: Some(vec![ChannelMode::Subscribe, ChannelMode::Publish]),
        cipher: None,
    }).await;

    channel.attach().await
        .expect("Should attach with options");

    assert_eq!(channel.state().await, ChannelState::Attached);
}

#[tokio::test]
async fn test_multiple_channel_attach() {
    let api_key = get_test_api_key();
    let client = RealtimeClient::with_api_key(&api_key).await;

    client.connect().await.unwrap();

    let channel1 = client.channel("channel-1").await;
    let channel2 = client.channel("channel-2").await;
    let channel3 = client.channel("channel-3").await;

    // Attach all channels
    channel1.attach().await.unwrap();
    channel2.attach().await.unwrap();
    channel3.attach().await.unwrap();

    assert_eq!(channel1.state().await, ChannelState::Attached);
    assert_eq!(channel2.state().await, ChannelState::Attached);
    assert_eq!(channel3.state().await, ChannelState::Attached);
}

#[tokio::test]
async fn test_channel_reattach_after_disconnect() {
    let api_key = get_test_api_key();
    let client = RealtimeClient::with_api_key(&api_key).await;

    client.connect().await.unwrap();

    let channel = client.channel("reattach-channel").await;
    channel.attach().await.unwrap();

    // Simulate disconnect
    client.disconnect().await.unwrap();

    // Reconnect
    client.connect().await.unwrap();

    // Channel should reattach automatically
    timeout(Duration::from_secs(5), async {
        while channel.state().await != ChannelState::Attached {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }).await.expect("Channel should reattach after reconnection");

    assert_eq!(channel.state().await, ChannelState::Attached);
}

#[tokio::test]
async fn test_channel_state_transitions() {
    let api_key = get_test_api_key();
    let client = RealtimeClient::with_api_key(&api_key).await;

    client.connect().await.unwrap();

    let channel = client.channel("state-test-channel").await;

    // Initial state
    assert_eq!(channel.state().await, ChannelState::Initialized);

    // Attaching - spawn the attach task to let it run in the background
    let channel_clone = Arc::clone(&channel);
    let attach_handle = tokio::spawn(async move {
        channel_clone.attach().await
    });

    // Give it time to start and change state to Attaching
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(channel.state().await, ChannelState::Attaching);

    // Wait for attach to complete
    attach_handle.await.unwrap().unwrap();
    assert_eq!(channel.state().await, ChannelState::Attached);

    // Detaching - spawn the detach task to let it run in the background
    let channel_clone = Arc::clone(&channel);
    let detach_handle = tokio::spawn(async move {
        channel_clone.detach().await
    });

    // Give it time to start and change state to Detaching
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(channel.state().await, ChannelState::Detaching);

    // Wait for detach to complete
    detach_handle.await.unwrap().unwrap();
    assert_eq!(channel.state().await, ChannelState::Detached);
}

#[tokio::test]
async fn test_channel_error_handling() {
    let api_key = get_test_api_key();
    let client = RealtimeClient::with_api_key(&api_key).await;

    client.connect().await.unwrap();

    // Try to attach to invalid channel name
    let channel = client.channel("").await; // Empty channel name

    let result = channel.attach().await;
    assert!(result.is_err(), "Should fail to attach to invalid channel");

    assert_eq!(channel.state().await, ChannelState::Failed);
}