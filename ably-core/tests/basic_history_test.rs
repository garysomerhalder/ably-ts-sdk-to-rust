// Test basic history functionality without encryption

use ably_core::client::rest::RestClient;
use ably_core::protocol::messages::Message;
use std::time::Duration;

const TEST_API_KEY: &str = "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA";

#[tokio::test]
async fn test_basic_history_no_encryption() {
    let client = RestClient::new(TEST_API_KEY);
    let channel = client.channel("test-basic-history");
    
    // Publish a simple message
    let message = Message::builder()
        .name("test-event")
        .data("Test message content")
        .build();
    
    channel.publish(message).await.unwrap();
    
    // Wait a bit
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Retrieve history
    let history = channel.history().limit(1).execute().await;
    
    match history {
        Ok(result) => {
            println!("History retrieved successfully!");
            println!("Number of items: {}", result.items.len());
            if !result.items.is_empty() {
                println!("Message data: {:?}", result.items[0].data);
                println!("Message name: {:?}", result.items[0].name);
                println!("Message encoding: {:?}", result.items[0].encoding);
            }
        }
        Err(e) => {
            panic!("Error retrieving history: {:?}", e);
        }
    }
}