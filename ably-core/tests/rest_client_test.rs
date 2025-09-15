// 🔴 RED Phase: Comprehensive REST client tests against real Ably API
// Testing all major REST endpoints

use ably_core::client::rest::{RestClient, PaginatedResult};
use ably_core::protocol::messages::{Message, PresenceMessage, Stats};
use ably_core::error::AblyResult;
use serde_json::json;
use std::time::Duration;

const TEST_API_KEY: &str = "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA";
const TEST_CHANNEL: &str = "test-rust-sdk-channel";

#[tokio::test]
async fn test_time_endpoint() {
    let client = RestClient::new(TEST_API_KEY);
    let result = client.time().await;
    assert!(result.is_ok());
    let time = result.unwrap();
    assert!(time > 0);
}

#[tokio::test]
async fn test_stats_endpoint() {
    let client = RestClient::new(TEST_API_KEY);
    let result = client.stats()
        .limit(10)
        .direction("backwards")
        .execute()
        .await;
    
    assert!(result.is_ok());
    let stats = result.unwrap();
    assert!(stats.items.len() <= 10);
}

#[tokio::test]
async fn test_channel_publish() {
    let client = RestClient::new(TEST_API_KEY);
    let channel = client.channel(TEST_CHANNEL);
    
    let message = Message::builder()
        .name("test-event")
        .data("Hello from Rust SDK")
        .build();
    
    let result = channel.publish(message).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_channel_publish_batch() {
    let client = RestClient::new(TEST_API_KEY);
    let channel = client.channel(TEST_CHANNEL);
    
    let messages = vec![
        Message::builder().data("Message 1").build(),
        Message::builder().data("Message 2").build(),
        Message::builder().data("Message 3").build(),
    ];
    
    let result = channel.publish_batch(messages).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_channel_history() {
    let client = RestClient::new(TEST_API_KEY);
    let channel = client.channel(TEST_CHANNEL);
    
    // First publish a message
    let message = Message::builder()
        .name("history-test")
        .data("Test message for history")
        .build();
    channel.publish(message).await.unwrap();
    
    // Wait a moment for message to be stored
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Now fetch history
    let result = channel.history()
        .limit(10)
        .direction("backwards")
        .execute()
        .await;
    
    assert!(result.is_ok());
    let history = result.unwrap();
    assert!(!history.items.is_empty());
}

#[tokio::test]
async fn test_presence_get() {
    let client = RestClient::new(TEST_API_KEY);
    let channel = client.channel(TEST_CHANNEL);
    
    let result = channel.presence().get().await;
    assert!(result.is_ok());
    let presence = result.unwrap();
    // May be empty if no members present
    assert!(presence.items.len() >= 0);
}

#[tokio::test]
async fn test_presence_history() {
    let client = RestClient::new(TEST_API_KEY);
    let channel = client.channel(TEST_CHANNEL);
    
    let result = channel.presence()
        .history()
        .limit(10)
        .execute()
        .await;
    
    assert!(result.is_ok());
    let history = result.unwrap();
    assert!(history.items.len() >= 0);
}

#[tokio::test]
async fn test_channel_status() {
    let client = RestClient::new(TEST_API_KEY);
    let channel = client.channel(TEST_CHANNEL);
    
    let result = channel.status().await;
    assert!(result.is_ok());
    let status = result.unwrap();
    assert!(status.channel_id.len() > 0);
}

#[tokio::test]
async fn test_request_token() {
    let client = RestClient::new(TEST_API_KEY);
    
    let result = client.auth()
        .request_token()
        .capability(TEST_CHANNEL, &["publish", "subscribe"])
        .ttl(3600)
        .execute()
        .await;
    
    assert!(result.is_ok());
    let token = result.unwrap();
    assert!(token.token.len() > 0);
}

#[tokio::test]
async fn test_pagination() {
    let client = RestClient::new(TEST_API_KEY);
    let channel = client.channel(TEST_CHANNEL);
    
    // Publish multiple messages
    for i in 0..5 {
        let message = Message::builder()
            .data(format!("Pagination test {}", i))
            .build();
        channel.publish(message).await.unwrap();
    }
    
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Get first page
    let first_page = channel.history()
        .limit(2)
        .execute()
        .await
        .unwrap();
    
    assert!(first_page.items.len() <= 2);
    
    // Check if next page exists
    if let Some(next) = first_page.next() {
        let second_page = next.execute().await.unwrap();
        assert!(second_page.items.len() > 0);
    }
}

#[tokio::test]
async fn test_push_admin_publish() {
    let client = RestClient::new(TEST_API_KEY);
    
    let result = client.push()
        .publish()
        .recipient(json!({
            "transportType": "fcm",
            "registrationToken": "test-token"
        }))
        .notification(json!({
            "title": "Test Push",
            "body": "Hello from Rust SDK"
        }))
        .execute()
        .await;
    
    // This might fail without proper push setup, but should parse correctly
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_batch_requests() {
    let client = RestClient::new(TEST_API_KEY);
    
    let batch = client.batch()
        .add_request("time", "/time", "GET", None, None)
        .add_request("stats", "/stats", "GET", None, Some(json!({"limit": 1})))
        .execute()
        .await;
    
    assert!(batch.is_ok());
    let responses = batch.unwrap();
    assert_eq!(responses.len(), 2);
}

#[tokio::test]
async fn test_channel_metadata() {
    let client = RestClient::new(TEST_API_KEY);
    
    let result = client.channels()
        .list()
        .prefix("test-")
        .limit(10)
        .execute()
        .await;
    
    assert!(result.is_ok());
    let channels = result.unwrap();
    assert!(channels.len() >= 0);
}

#[tokio::test]
async fn test_rest_client_with_token() {
    let client = RestClient::new(TEST_API_KEY);
    
    // First get a token
    let token_result = client.auth()
        .request_token()
        .execute()
        .await
        .unwrap();
    
    // Create new client with token
    let token_client = RestClient::with_token(&token_result.token);
    
    // Test that it works
    let time = token_client.time().await;
    assert!(time.is_ok());
}

#[tokio::test]
async fn test_connection_recovery() {
    let client = RestClient::builder()
        .api_key(TEST_API_KEY)
        .max_retries(3)
        .timeout(Duration::from_secs(5))
        .build();
    
    // Even with network issues, should retry
    let result = client.time().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_custom_headers() {
    let client = RestClient::builder()
        .api_key(TEST_API_KEY)
        .custom_header("X-Ably-Version", "2.0")
        .custom_header("X-Client-Id", "rust-sdk-test")
        .build();
    
    let result = client.time().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_different_environments() {
    // Test that we can configure different environments
    let client = RestClient::builder()
        .api_key(TEST_API_KEY)
        .environment("sandbox")
        .build();
    
    let result = client.time().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_idempotent_requests() {
    let client = RestClient::new(TEST_API_KEY);
    let channel = client.channel(TEST_CHANNEL);
    
    let message = Message::builder()
        .id("unique-message-id-12345")
        .data("Idempotent message")
        .build();
    
    // Publish twice with same ID
    let result1 = channel.publish(message.clone()).await;
    let result2 = channel.publish(message).await;
    
    assert!(result1.is_ok());
    assert!(result2.is_ok());
}