// 🔴 RED Phase: Delta compression integration tests
// Testing against real Ably sandbox API with delta channel parameters

use ably_core::client::realtime::{RealtimeClient, RealtimeClientBuilder};
use ably_core::delta::{DeltaPlugin, VcdiffDecoder};
use ably_core::protocol::messages::{Message, ProtocolMessage};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_delta_compression_plugin() {
    let api_key = std::env::var("ABLY_API_KEY_SANDBOX")
        .unwrap_or_else(|_| "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA".to_string());

    // Create delta decoder plugin
    let delta_plugin = DeltaPlugin::new(VcdiffDecoder::new());
    
    // Create realtime client with delta plugin
    let client = RealtimeClientBuilder::default()
        .api_key(api_key)
        .plugin("vcdiff", Box::new(delta_plugin))
        .build()
        .await
        .expect("Failed to create realtime client");

    // Get channel with delta mode enabled
    let channel = client.channel("test-delta-channel").await;
    channel.set_params(HashMap::from([("delta".to_string(), "vcdiff".to_string())])).await;

    // Track received messages for validation
    let received_messages = Arc::new(Mutex::new(Vec::new()));
    let messages_clone = received_messages.clone();

    // Subscribe to messages
    channel.subscribe(move |message| {
        messages_clone.lock().unwrap().push(message);
    }).await;

    // Attach to channel
    channel.attach().await.expect("Failed to attach to channel");

    // Test data - sequential changes that should benefit from delta compression
    let test_data = vec![
        json!({"foo": "bar", "count": 1, "status": "active"}),
        json!({"foo": "bar", "count": 2, "status": "active"}),
        json!({"foo": "bar", "count": 2, "status": "inactive"}),
        json!({"foo": "bar", "count": 3, "status": "inactive"}),
        json!({"foo": "bar", "count": 3, "status": "active"}),
    ];

    // Publish test messages
    for (i, data) in test_data.iter().enumerate() {
        let message = Message {
            name: Some(i.to_string()),
            data: Some(data.clone()),
            ..Default::default()
        };
        
        channel.publish(message).await.expect("Failed to publish message");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Wait for all messages to be received
    timeout(Duration::from_secs(10), async {
        loop {
            let count = received_messages.lock().unwrap().len();
            if count >= test_data.len() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }).await.expect("Timeout waiting for delta messages");

    // Validate received messages match expected data
    let messages = received_messages.lock().unwrap();
    assert_eq!(messages.len(), test_data.len());
    
    for (i, message) in messages.iter().enumerate() {
        assert_eq!(message.name.as_ref().unwrap(), &i.to_string());
        assert_eq!(message.data.as_ref().unwrap(), &test_data[i]);
    }

    client.disconnect().await.expect("Failed to disconnect");
}

#[tokio::test]
async fn test_delta_decode_failure_recovery() {
    let api_key = std::env::var("ABLY_API_KEY_SANDBOX")
        .unwrap_or_else(|_| "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA".to_string());

    // Create client with delta support
    let delta_plugin = DeltaPlugin::new(VcdiffDecoder::new());
    
    let client = RealtimeClientBuilder::default()
        .api_key(api_key)
        .plugin("vcdiff", Box::new(delta_plugin))
        .build()
        .await
        .expect("Failed to create realtime client");

    let channel = client.channel("test-delta-recovery").await;
    channel.set_params(HashMap::from([("delta".to_string(), "vcdiff".to_string())])).await;

    // Subscribe and attach
    let received_errors = Arc::new(Mutex::new(Vec::new()));
    let errors_clone = received_errors.clone();
    
    channel.on_error(move |error| {
        errors_clone.lock().unwrap().push(error);
    }).await;

    channel.attach().await.expect("Failed to attach to channel");

    // Simulate delta decode failure by publishing with missing base message
    // This should trigger decode failure recovery
    let problematic_message = Message {
        name: Some("delta-test".to_string()),
        data: Some(json!({"data": "test"})),
        extras: Some(json!({
            "delta": {
                "from": "missing-message-id"
            }
        })),
        ..Default::default()
    };

    channel.publish(problematic_message).await.expect("Failed to publish message");

    // Wait for recovery process
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Verify channel is still operational after recovery
    assert!(channel.is_attached().await);

    client.disconnect().await.expect("Failed to disconnect");
}

#[tokio::test] 
async fn test_vcdiff_decoder_basic() {
    // Test the VCDIFF decoder directly
    let decoder = VcdiffDecoder::new();
    
    // Simple test data
    let source = b"Hello, World!";
    let target = b"Hello, Ably!";
    
    // Create a mock delta (in real implementation, this would come from Ably)
    // For this test, we'll simulate the delta format
    let delta = create_mock_vcdiff_delta(source, target);
    
    let decoded = decoder.decode(&delta, source).expect("Failed to decode delta");
    assert_eq!(decoded, target);
}

// Helper function to create mock VCDIFF delta for testing
fn create_mock_vcdiff_delta(source: &[u8], target: &[u8]) -> Vec<u8> {
    // This is a simplified mock - real VCDIFF format is more complex
    // For now, we'll create a basic delta representation
    let mut delta = Vec::new();
    
    // VCDIFF header magic
    delta.extend_from_slice(b"VCD");
    delta.push(0x00); // Version
    
    // For this mock, we'll store the target directly
    // Real VCDIFF would have copy/add instructions
    delta.extend_from_slice(target);
    
    delta
}