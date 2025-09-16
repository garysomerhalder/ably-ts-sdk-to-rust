use ably_core::client::rest::RestClient;
use ably_core::protocol::messages::Message;
use serde_json::json;

#[tokio::test]
async fn test_live_ably_connection() {
    println!("\n🔧 Testing Live Ably SDK Functionality\n");
    
    // Get API key
    let api_key = std::env::var("ABLY_API_KEY_SANDBOX")
        .unwrap_or_else(|_| "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA".to_string());
    
    println!("Using API key: {}...", &api_key[..15]);
    
    // Create client
    let client = RestClient::new(api_key.clone());
    
    // Test 1: Time endpoint
    println!("\n1️⃣ Testing Time Endpoint...");
    let time = client.time().await.expect("Failed to get time");
    println!("   ✅ Server time: {} ms", time);
    assert!(time > 1700000000000); // Should be after Nov 2023
    
    // Test 2: Publish a message
    println!("\n2️⃣ Testing Channel Publish...");
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let channel_name = format!("test-rust-sdk-{}", timestamp);
    let channel = client.channel(&channel_name);
    
    let message = Message {
        name: Some("test-event".to_string()),
        data: Some(json!("Hello from Rust SDK!")),
        ..Default::default()
    };
    
    channel.publish(message).await.expect("Failed to publish message");
    println!("   ✅ Message published to channel: {}", channel_name);
    
    // Test 3: Retrieve history
    println!("\n3️⃣ Testing Channel History...");
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await; // Wait for message to be stored
    
    let history = channel.history()
        .execute()
        .await
        .expect("Failed to get history");
    
    println!("   ✅ Retrieved {} messages", history.items.len());
    assert!(history.items.len() > 0, "Should have at least one message");
    
    let first_msg = &history.items[0];
    assert_eq!(first_msg.name.as_deref(), Some("test-event"));
    
    // Test 4: Stats endpoint
    println!("\n4️⃣ Testing Stats Endpoint...");
    let stats = client.stats()
        .execute()
        .await
        .expect("Failed to get stats");
    println!("   ✅ Retrieved {} stat entries", stats.items.len());
    
    // Test 5: Channel list
    println!("\n5️⃣ Testing Channel List...");
    let channels_list = client.channels()
        .list()
        .execute()
        .await
        .expect("Failed to list channels");
    println!("   ✅ Found {} active channels", channels_list.len());
    
    // Find our test channel
    let our_channel = channels_list.iter().find(|c| c.channel_id == channel_name);
    assert!(our_channel.is_some(), "Our test channel should be in the list");
    
    println!("\n✨ All tests passed!");
}

#[tokio::test] 
async fn test_batch_publish() {
    let api_key = std::env::var("ABLY_API_KEY_SANDBOX")
        .unwrap_or_else(|_| "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA".to_string());
    
    let client = RestClient::new(api_key);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let channel_name = format!("batch-test-{}", timestamp);
    let channel = client.channel(&channel_name);
    
    // Publish multiple messages
    let mut messages = vec![];
    for i in 1..=5 {
        messages.push(Message {
            name: Some("msg".to_string()),
            data: Some(json!(format!("Message #{}", i))),
            ..Default::default()
        });
    }
    
    channel.publish_batch(messages).await.expect("Failed to publish batch");
    
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    
    let history = channel.history()
        .execute()
        .await
        .expect("Failed to get history");
    
    assert_eq!(history.items.len(), 5, "Should have 5 messages");
    println!("✅ Batch publish test passed: {} messages", history.items.len());
}

#[tokio::test]
async fn test_presence_operations() {
    let api_key = std::env::var("ABLY_API_KEY_SANDBOX")
        .unwrap_or_else(|_| "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA".to_string());
    
    let client = RestClient::new(api_key);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let channel_name = format!("presence-test-{}", timestamp);
    let channel = client.channel(&channel_name);
    
    // Get presence (should be empty initially)
    let presence = channel.presence()
        .get()
        .await
        .expect("Failed to get presence");
    
    println!("✅ Presence test passed: {} members present", presence.items.len());
    assert_eq!(presence.items.len(), 0, "Should have no presence members initially");
}