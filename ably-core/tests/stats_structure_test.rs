use serde_json::json;
use ably_core::client::rest::RestClient;

#[test]
fn test_stats_deserialization_with_actual_api_format() {
    // This is the actual format returned by Ably API for stats
    let json_str = r#"[
        {
            "intervalId": "2025-01-16:04:33",
            "unit": "minute",
            "processed": false,
            "channels": {
                "count": 5,
                "meanData": 17.4,
                "meanRate": 0.08333333333333333
            }
        }
    ]"#;
    
    // The current Stats struct won't parse this correctly
    // We need to fix the Stats definition
}

#[tokio::test]
async fn test_actual_api_stats_format() {
    // Test against real API
    let api_key = "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA";
    let client = RestClient::new(api_key);
    
    // Make raw request to see what we actually get
    let http_client = client.http_client();
    let response = http_client
        .get("/stats")
        .query(&[("limit", "1")])
        .send()
        .await
        .expect("Failed to send request");
    
    // Get raw text first
    let raw_text = response.text().await.expect("Failed to get text");
    println!("Raw stats response (truncated): {}", &raw_text[..500.min(raw_text.len())]);
    
    // Parse as JSON Value
    let json_value: serde_json::Value = serde_json::from_str(&raw_text)
        .expect("Failed to parse as JSON Value");
    
    if let Some(first_stat) = json_value.get(0) {
        println!("First stat entry keys: {:?}", first_stat.as_object().unwrap().keys().collect::<Vec<_>>());
        println!("intervalId: {:?}", first_stat.get("intervalId"));
        println!("unit: {:?}", first_stat.get("unit"));
        println!("channels: {:?}", first_stat.get("channels"));
    }
}