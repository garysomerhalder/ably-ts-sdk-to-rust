// 🟢 GREEN Phase: Full integration test with WebSocket and state machine
// Tests complete connection lifecycle with real Ably service

use ably_core::client::rest::RestClient;
use ably_core::client::realtime::RealtimeClient;
use ably_core::protocol::messages::{Message, Action};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_full_websocket_lifecycle() {
    println!("\n🚀 Testing Full WebSocket Lifecycle with State Machine\n");
    
    let api_key = "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA";
    
    // Create realtime client
    let client = RealtimeClient::new(api_key).await.expect("Failed to create client");
    
    println!("1️⃣ Connecting to Ably realtime...");
    client.connect().await.expect("Failed to connect");
    
    // Wait for connection
    sleep(Duration::from_secs(1)).await;
    
    println!("2️⃣ Checking connection state...");
    assert!(client.is_connected().await, "Should be connected");
    
    let connection_id = client.connection_id().await;
    println!("   ✅ Connected with ID: {:?}", connection_id);
    assert!(connection_id.is_some(), "Should have connection ID");
    
    // Create a channel
    println!("\n3️⃣ Creating channel 'test-channel'...");
    let channel = client.channel("test-channel");
    
    // Attach to the channel
    println!("4️⃣ Attaching to channel...");
    channel.attach().await.expect("Failed to attach to channel");
    sleep(Duration::from_secs(1)).await;
    
    // Publish a message
    println!("5️⃣ Publishing message...");
    let message = Message {
        name: Some("test".to_string()),
        data: Some(serde_json::json!("Hello from Rust SDK!")),
        id: None,
        client_id: None,
        connection_id: None,
        encoding: None,
        timestamp: None,
        extras: None,
    };
    
    channel.publish(message).await.expect("Failed to publish message");
    println!("   ✅ Message published");
    
    // Subscribe to messages
    println!("6️⃣ Subscribing to messages...");
    let mut subscription = channel.subscribe().await;
    
    // Publish another message to receive
    let test_message = Message {
        name: Some("echo".to_string()),
        data: Some(serde_json::json!("Echo test")),
        id: None,
        client_id: None,
        connection_id: None,
        encoding: None,
        timestamp: None,
        extras: None,
    };
    
    channel.publish(test_message).await.expect("Failed to publish echo");
    
    // Wait for message
    println!("7️⃣ Waiting for message...");
    tokio::select! {
        Some(msg) = subscription.recv() => {
            println!("   ✅ Received message: {:?}", msg.name);
            assert_eq!(msg.name, Some("echo".to_string()));
        }
        _ = sleep(Duration::from_secs(5)) => {
            panic!("Timeout waiting for message");
        }
    }
    
    // Detach from channel
    println!("8️⃣ Detaching from channel...");
    channel.detach().await.expect("Failed to detach");
    
    // Disconnect
    println!("9️⃣ Disconnecting...");
    client.disconnect().await;
    sleep(Duration::from_secs(1)).await;
    
    assert!(!client.is_connected().await, "Should be disconnected");
    println!("   ✅ Disconnected successfully");
    
    println!("\n✨ Full lifecycle test complete!");
}

#[tokio::test]
async fn test_connection_recovery() {
    println!("\n🔄 Testing Connection Recovery\n");
    
    let api_key = "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA";
    let client = RealtimeClient::new(api_key).await.expect("Failed to create client");
    
    // Connect
    println!("1️⃣ Initial connection...");
    client.connect().await.expect("Failed to connect");
    sleep(Duration::from_secs(1)).await;
    
    let first_connection_id = client.connection_id().await;
    println!("   Connection ID: {:?}", first_connection_id);
    
    // Force disconnect
    println!("2️⃣ Forcing disconnect...");
    client.disconnect().await;
    sleep(Duration::from_secs(1)).await;
    assert!(!client.is_connected().await);
    
    // Reconnect
    println!("3️⃣ Reconnecting...");
    client.connect().await.expect("Failed to reconnect");
    sleep(Duration::from_secs(1)).await;
    
    let second_connection_id = client.connection_id().await;
    println!("   New connection ID: {:?}", second_connection_id);
    
    assert!(client.is_connected().await);
    assert_ne!(first_connection_id, second_connection_id, "Should have new connection ID");
    
    println!("\n✅ Connection recovery successful!");
}

#[tokio::test]
async fn test_heartbeat_mechanism() {
    println!("\n💓 Testing Heartbeat Mechanism\n");
    
    let api_key = "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA";
    let client = RealtimeClient::new(api_key).await.expect("Failed to create client");
    
    println!("1️⃣ Connecting...");
    client.connect().await.expect("Failed to connect");
    
    println!("2️⃣ Starting heartbeat monitor...");
    
    // Monitor heartbeats for 20 seconds
    let mut heartbeat_count = 0;
    let start = std::time::Instant::now();
    
    while start.elapsed() < Duration::from_secs(20) {
        sleep(Duration::from_secs(1)).await;
        
        if client.is_connected().await {
            heartbeat_count += 1;
            print!("💓 ");
            if heartbeat_count % 10 == 0 {
                println!(" ({} seconds)", heartbeat_count);
            }
        }
    }
    
    println!("\n3️⃣ Connection maintained for 20 seconds");
    assert!(client.is_connected().await, "Should still be connected");
    
    client.disconnect().await;
    println!("\n✅ Heartbeat mechanism working!");
}

#[tokio::test]
async fn test_concurrent_channels() {
    println!("\n🔀 Testing Concurrent Channels\n");
    
    let api_key = "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA";
    let client = RealtimeClient::new(api_key).await.expect("Failed to create client");
    
    client.connect().await.expect("Failed to connect");
    sleep(Duration::from_secs(1)).await;
    
    // Create multiple channels
    let channel1 = client.channel("channel-1");
    let channel2 = client.channel("channel-2");
    let channel3 = client.channel("channel-3");
    
    println!("1️⃣ Attaching to 3 channels concurrently...");
    
    // Attach all channels concurrently
    let (r1, r2, r3) = tokio::join!(
        channel1.attach(),
        channel2.attach(),
        channel3.attach()
    );
    
    r1.expect("Channel 1 attach failed");
    r2.expect("Channel 2 attach failed");
    r3.expect("Channel 3 attach failed");
    
    println!("   ✅ All channels attached");
    
    // Publish to all channels
    println!("2️⃣ Publishing to all channels...");
    
    let msg = Message {
        name: Some("concurrent-test".to_string()),
        data: Some(serde_json::json!("Testing concurrent channels")),
        id: None,
        client_id: None,
        connection_id: None,
        encoding: None,
        timestamp: None,
        extras: None,
    };
    
    let (p1, p2, p3) = tokio::join!(
        channel1.publish(msg.clone()),
        channel2.publish(msg.clone()),
        channel3.publish(msg.clone())
    );
    
    p1.expect("Publish to channel 1 failed");
    p2.expect("Publish to channel 2 failed");
    p3.expect("Publish to channel 3 failed");
    
    println!("   ✅ Published to all channels");
    
    // Detach all
    println!("3️⃣ Detaching from all channels...");
    
    let (d1, d2, d3) = tokio::join!(
        channel1.detach(),
        channel2.detach(),
        channel3.detach()
    );
    
    d1.expect("Channel 1 detach failed");
    d2.expect("Channel 2 detach failed");
    d3.expect("Channel 3 detach failed");
    
    println!("   ✅ All channels detached");
    
    client.disconnect().await;
    println!("\n✅ Concurrent channels test complete!");
}