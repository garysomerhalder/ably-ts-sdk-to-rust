use serde_json::json;
use ably_core::protocol::messages::Message;

#[test]
fn test_message_deserialization_with_actual_api_format() {
    // This is the actual format returned by Ably API
    let json_str = r#"[
        {
            "id": "VbbQyqHyQ7:9831:0",
            "timestamp": 1758088054695,
            "data": "hello from analysis",
            "action": 0,
            "serial": "01758088054695-000@6f8nI1_2gBtecQ24071208:000",
            "name": "test"
        }
    ]"#;
    
    // Try to deserialize
    let result: Result<Vec<Message>, _> = serde_json::from_str(json_str);
    
    // This should work
    assert!(result.is_ok(), "Failed to parse actual API response: {:?}", result);
    
    let messages = result.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].name.as_deref(), Some("test"));
    assert_eq!(messages[0].action, Some(0));
}

#[test]
fn test_message_with_non_optional_action() {
    // Test that we can handle action as a non-optional field
    let json = json!({
        "id": "test-id",
        "name": "test",
        "data": "hello",
        "action": 15,  // Not wrapped in Option
        "timestamp": 1234567890
    });
    
    let result: Result<Message, _> = serde_json::from_value(json);
    assert!(result.is_ok(), "Should handle non-optional action field");
}

#[tokio::test]
async fn test_actual_api_history_format() {
    // Test against real API
    let api_key = "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA";
    let client = ably_core::client::rest::RestClient::new(api_key);
    
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let channel_name = format!("json-test-{}", timestamp);
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
    
    // Try to retrieve history
    let history = channel.history()
        .execute()
        .await
        .expect("Failed to get history - JSON parsing issue!");
    
    assert!(history.items.len() > 0, "Should have at least one message");
    assert_eq!(history.items[0].name.as_deref(), Some("test-event"));
}