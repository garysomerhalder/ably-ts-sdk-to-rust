// 🔴 RED Phase: Tests for message history replay functionality
// Allows recovery of channel state after reconnection

use ably_core::client::rest::RestClient;
use ably_core::client::realtime::RealtimeClient;
use ably_core::protocol::messages::Message;
use ably_core::replay::{HistoryReplay, ReplayOptions, ReplayPosition};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// Note: Using placeholder API key - needs valid sandbox key
const TEST_API_KEY: &str = "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA";

#[tokio::test]
async fn test_basic_history_replay() {
    let client = RestClient::new(TEST_API_KEY);
    let channel = client.channel("test-replay");
    
    // Publish some messages
    for i in 0..5 {
        let msg = Message::builder()
            .name("test-event")
            .data(format!("Message {}", i))
            .build();
        channel.publish(msg).await.unwrap();
    }
    
    // Wait for messages to be persisted
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Create replay from a timestamp
    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64 - 10000; // 10 seconds ago
    
    let replay = HistoryReplay::new(channel, ReplayOptions {
        start: ReplayPosition::Timestamp(start_time),
        end: None,
        limit: Some(100),
        direction: None,
    });
    
    // Replay messages
    let replayed = replay.execute().await.unwrap();
    assert_eq!(replayed.len(), 5);
    
    for (i, msg) in replayed.iter().enumerate() {
        assert_eq!(
            msg.data.as_ref().unwrap().as_str().unwrap(),
            format!("Message {}", i)
        );
    }
}

#[tokio::test]
async fn test_replay_with_serial_position() {
    let client = RestClient::new(TEST_API_KEY);
    let channel = client.channel("test-replay-serial");
    
    // Publish messages and track serials
    let mut serials = Vec::new();
    for i in 0..10 {
        let msg = Message::builder()
            .data(format!("Message {}", i))
            .build();
        channel.publish(msg).await.unwrap();
        
        // Get last message serial
        let history = channel.history().limit(1).execute().await.unwrap();
        if let Some(serial) = history.items[0].serial.clone() {
            serials.push(serial);
        }
    }
    
    // Replay from middle serial
    if serials.len() >= 5 {
        let replay = HistoryReplay::new(channel, ReplayOptions {
            start: ReplayPosition::Serial(serials[4].clone()),
            end: None,
            limit: None,
            direction: None,
        });
        
        let replayed = replay.execute().await.unwrap();
        assert!(replayed.len() >= 5);
    }
}

#[tokio::test]
async fn test_replay_with_channel_serial() {
    let client = RestClient::new(TEST_API_KEY);
    let channel = client.channel("test-replay-channel-serial");
    
    // Publish messages
    for i in 0..5 {
        let msg = Message::builder()
            .data(format!("Data {}", i))
            .build();
        channel.publish(msg).await.unwrap();
    }
    
    // Get channel serial after messages
    let channel_serial = channel.get_channel_serial().await.unwrap();
    
    // Publish more messages
    for i in 5..10 {
        let msg = Message::builder()
            .data(format!("Data {}", i))
            .build();
        channel.publish(msg).await.unwrap();
    }
    
    // Replay from channel serial
    let replay = HistoryReplay::new(channel, ReplayOptions {
        start: ReplayPosition::ChannelSerial(channel_serial),
        end: None,
        limit: None,
        direction: None,
    });
    
    let replayed = replay.execute().await.unwrap();
    assert_eq!(replayed.len(), 5); // Should get messages 5-9
}

#[tokio::test]
async fn test_replay_with_limit_and_direction() {
    let client = RestClient::new(TEST_API_KEY);
    let channel = client.channel("test-replay-limit");
    
    // Publish messages
    for i in 0..20 {
        let msg = Message::builder()
            .data(format!("Item {}", i))
            .build();
        channel.publish(msg).await.unwrap();
    }
    
    // Replay with limit, backwards
    let replay = HistoryReplay::new(channel, ReplayOptions {
        start: ReplayPosition::Now,
        end: None,
        limit: Some(5),
        direction: Some("backwards".to_string()),
    });
    
    let replayed = replay.execute().await.unwrap();
    assert_eq!(replayed.len(), 5);
    
    // Should get most recent 5 messages in reverse order
    assert_eq!(
        replayed[0].data.as_ref().unwrap().as_str().unwrap(),
        "Item 19"
    );
}

#[tokio::test]
async fn test_realtime_replay_on_reconnect() {
    // TODO: Enable when RealtimeClient is ready
    // let client = RealtimeClient::builder()
    //     .api_key(TEST_API_KEY)
    //     .build()
    //     .await
    //     .unwrap();
    //
    // client.connect().await.unwrap();
    //
    // let channel = client.channel("test-realtime-replay");
    // channel.attach().await.unwrap();
    //
    // // Track last seen serial
    // let last_serial = channel.get_last_serial();
    //
    // // Simulate disconnect
    // client.disconnect().await.unwrap();
    //
    // // Messages published while disconnected
    // let rest_client = RestClient::new(TEST_API_KEY);
    // let rest_channel = rest_client.channel("test-realtime-replay");
    // for i in 0..5 {
    //     let msg = Message::builder()
    //         .data(format!("Missed message {}", i))
    //         .build();
    //     rest_channel.publish(msg).await.unwrap();
    // }
    //
    // // Reconnect and replay
    // client.connect().await.unwrap();
    // channel.attach_with_replay(last_serial).await.unwrap();
    //
    // // Should receive missed messages
    // let replayed = channel.get_replayed_messages();
    // assert_eq!(replayed.len(), 5);
}

#[tokio::test]
async fn test_replay_error_handling() {
    let client = RestClient::new(TEST_API_KEY);
    let channel = client.channel("test-replay-error");
    
    // Try replay with invalid serial
    let replay = HistoryReplay::new(channel, ReplayOptions {
        start: ReplayPosition::Serial("invalid-serial".to_string()),
        end: None,
        limit: None,
        direction: None,
    });
    
    let result = replay.execute().await;
    assert!(result.is_err());
    
    // Try replay with future timestamp
    let future_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64 + 3600000; // 1 hour in future
    
    let replay = HistoryReplay::new(channel, ReplayOptions {
        start: ReplayPosition::Timestamp(future_time),
        end: None,
        limit: None,
        direction: None,
    });
    
    let result = replay.execute().await.unwrap();
    assert!(result.is_empty()); // No messages in future
}