use serde_json::json;
use ably_core::client::rest::RestClient;
use ably_core::protocol::messages::Message;

#[tokio::test]
async fn debug_history_response() {
    let api_key = "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA";
    let client = RestClient::new(api_key);
    
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let channel_name = format!("debug-test-{}", timestamp);
    let channel = client.channel(&channel_name);
    
    // Publish a message
    let msg = Message {
        name: Some("test-event".to_string()),
        data: Some(json!("test data")),
        ..Default::default()
    };
    
    channel.publish(msg).await.expect("Failed to publish");
    
    // Wait for it to be stored
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    
    // Make raw request to see what we actually get
    let http_client = client.http_client();
    let path = format!("/channels/{}/messages", channel_name);
    
    let response = http_client
        .get(&path)
        .send()
        .await
        .expect("Failed to send request");
    
    // Get raw text first
    let raw_text = response.text().await.expect("Failed to get text");
    println!("Raw response text: {}", raw_text);
    
    // Try to parse as JSON Value
    let json_value: serde_json::Value = serde_json::from_str(&raw_text)
        .expect("Failed to parse as JSON Value");
    println!("Parsed JSON Value: {:?}", json_value);
    
    // Try to parse as Vec<Message>
    let messages: Result<Vec<Message>, _> = serde_json::from_str(&raw_text);
    match messages {
        Ok(msgs) => {
            println!("Successfully parsed {} messages", msgs.len());
            for (i, msg) in msgs.iter().enumerate() {
                println!("Message {}: name={:?}, data={:?}", i, msg.name, msg.data);
            }
        }
        Err(e) => {
            println!("Failed to parse as Vec<Message>: {}", e);
            println!("Error details: {:?}", e);
        }
    }
}