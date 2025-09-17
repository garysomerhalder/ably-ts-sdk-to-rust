use ably_core::client::rest::RestClient;
use serde_json::json;

#[tokio::test]
async fn debug_token_request() {
    let api_key = "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA";
    let client = RestClient::new(api_key);
    
    // Parse the API key to get app ID and key ID  
    let parts: Vec<&str> = api_key.split('.').collect();
    let app_id = parts[0];
    let key_parts: Vec<&str> = parts[1].split(':').collect();
    let key_id = key_parts[0];
    let key_name = format!("{}.{}", app_id, key_id);
    
    println!("App ID: {}", app_id);
    println!("Key ID: {}", key_id);
    println!("Key Name: {}", key_name);
    
    // Try to make raw request to see what we get
    let http_client = client.http_client();
    let path = format!("/keys/{}/requestToken", key_name);
    println!("Request path: {}", path);
    
    // Create request body with timestamp and keyName
    let mut request_body = serde_json::Map::new();
    request_body.insert("timestamp".to_string(), json!(chrono::Utc::now().timestamp_millis()));
    request_body.insert("keyName".to_string(), json!(key_name));
    
    println!("Request body: {:?}", request_body);
    
    let response = http_client
        .post(&path)
        .json(&request_body)
        .send()
        .await
        .expect("Failed to send request");
    
    // Get raw text first
    let raw_text = response.text().await.expect("Failed to get text");
    println!("Raw token response: {}", raw_text);
    
    // Try to parse as JSON Value
    let json_value: serde_json::Value = serde_json::from_str(&raw_text)
        .expect("Failed to parse as JSON Value");
    println!("Parsed JSON Value: {:?}", json_value);
}