// 🔴 RED Phase: Comprehensive test for JSON parsing failures
// This test demonstrates the mismatch between expected data structures and actual API responses

use ably_core::client::rest::{RestClient, ChannelStatus};
use ably_core::protocol::messages::Stats;
use serde_json::{json, Value};

const TEST_API_KEY: &str = "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA";
const TEST_CHANNEL: &str = "test-json-parsing-channel";

/// Test that demonstrates actual API response formats vs expected structures
#[tokio::test]
async fn test_actual_api_response_formats() {
    let client = RestClient::new(TEST_API_KEY);
    
    // Test 1: Stats endpoint - currently fails to parse
    println!("Testing stats endpoint...");
    let stats_result = client.stats().limit(1).execute().await;
    match stats_result {
        Ok(stats) => {
            println!("Stats parsed successfully, {} items", stats.items.len());
        }
        Err(e) => {
            println!("Stats parsing failed: {}", e);
            // This should fail due to wrong structure
        }
    }
    
    // Test 2: Channel status - currently fails to parse
    println!("Testing channel status...");
    let status_result = client.channel(TEST_CHANNEL).status().await;
    match status_result {
        Ok(status) => {
            println!("Channel status parsed successfully: {}", status.channel_id);
        }
        Err(e) => {
            println!("Channel status parsing failed: {}", e);
            // This should fail due to structure mismatch
        }
    }
    
    // Test 3: Token request - currently uses wrong endpoint
    println!("Testing token request...");
    let token_result = client.auth().request_token().execute().await;
    match token_result {
        Ok(token) => {
            println!("Token parsed successfully: {}", token.token);
        }
        Err(e) => {
            println!("Token request failed: {}", e);
            // This should fail due to wrong endpoint path
        }
    }
    
    // Test 4: Channel history - may fail with complex responses
    println!("Testing channel history...");
    let history_result = client.channel(TEST_CHANNEL).history().limit(5).execute().await;
    match history_result {
        Ok(history) => {
            println!("History parsed successfully, {} items", history.items.len());
        }
        Err(e) => {
            println!("History parsing failed: {}", e);
        }
    }
    
    // Test 5: Presence endpoints
    println!("Testing presence get...");
    let presence_result = client.channel(TEST_CHANNEL).presence().get().await;
    match presence_result {
        Ok(presence) => {
            println!("Presence parsed successfully, {} items", presence.items.len());
        }
        Err(e) => {
            println!("Presence parsing failed: {}", e);
        }
    }
    
    // Test 6: Presence history
    println!("Testing presence history...");
    let presence_history_result = client.channel(TEST_CHANNEL).presence().history().limit(5).execute().await;
    match presence_history_result {
        Ok(history) => {
            println!("Presence history parsed successfully, {} items", history.items.len());
        }
        Err(e) => {
            println!("Presence history parsing failed: {}", e);
        }
    }
    
    // Test 7: Channel metadata
    println!("Testing channel metadata...");
    let channels_result = client.channels().list().limit(5).execute().await;
    match channels_result {
        Ok(channels) => {
            println!("Channel metadata parsed successfully, {} channels", channels.len());
        }
        Err(e) => {
            println!("Channel metadata parsing failed: {}", e);
        }
    }
    
    // This test is expected to fail in multiple places due to JSON parsing issues
    // Once we fix the data structures, this test should pass
    println!("Test completed - check output above for parsing failures");
}

/// Test to validate what the actual API responses look like
#[tokio::test]
async fn test_raw_api_responses() {
    use ably_core::http::{AblyHttpClient, HttpConfig};
    use ably_core::auth::AuthMode;
    
    // Explicitly configure for sandbox environment
    let config = HttpConfig::default(); // This should already point to sandbox
    println!("Using base URL: {}", config.base_url);
    let auth = AuthMode::ApiKey(TEST_API_KEY.to_string());
    let http_client = AblyHttpClient::with_auth(config, auth);
    
    // Get raw stats response
    println!("Raw stats response:");
    let stats_response = http_client.get("/stats?limit=1").send().await.unwrap();
    let stats_json: Value = stats_response.json().await.unwrap();
    println!("{}", serde_json::to_string_pretty(&stats_json).unwrap());
    
    // Get raw channel status response
    println!("\nRaw channel status response:");
    let status_response = http_client.get(&format!("/channels/{}", TEST_CHANNEL)).send().await.unwrap();
    let status_json: Value = status_response.json().await.unwrap();
    println!("{}", serde_json::to_string_pretty(&status_json).unwrap());
    
    // Try different token endpoints to find the correct one
    println!("\nTesting token endpoints...");
    
    let token_endpoints = ["/keys/token", "/tokens", "/keys/request", "/token"];
    for endpoint in &token_endpoints {
        println!("Trying endpoint: {}", endpoint);
        let result = http_client.post(endpoint).json(&json!({})).send().await;
        match result {
            Ok(response) => {
                let status = response.status();
                println!("  Status: {}", status);
                if status.is_success() {
                    let json: Value = response.json().await.unwrap();
                    println!("  Success response: {}", serde_json::to_string_pretty(&json).unwrap());
                } else {
                    let text = response.text().await.unwrap_or_default();
                    println!("  Error response: {}", text);
                }
            }
            Err(e) => {
                println!("  Request failed: {}", e);
            }
        }
    }
    
    // Get raw presence response
    println!("\nRaw presence response:");
    let presence_response = http_client.get(&format!("/channels/{}/presence", TEST_CHANNEL)).send().await.unwrap();
    let presence_json: Value = presence_response.json().await.unwrap();
    println!("{}", serde_json::to_string_pretty(&presence_json).unwrap());
    
    // Get raw channels list response
    println!("\nRaw channels list response:");
    let channels_response = http_client.get("/channels?limit=1").send().await.unwrap();
    let channels_json: Value = channels_response.json().await.unwrap();
    println!("{}", serde_json::to_string_pretty(&channels_json).unwrap());
}

/// Test that demonstrates the expected vs actual data structure mismatches
#[test]
fn test_data_structure_examples() {
    // Example of actual stats response format (from API testing)
    let actual_stats_json = json!([
        {
            "channels": {
                "peak": 1,
                "min": 1,
                "mean": 0.7
            },
            "processed": null,
            "intervalId": "2025-09-16:04:39",
            "unit": "minute"
        }
    ]);
    
    // Try to parse with current Stats structure - this should fail
    let stats_parse_result: Result<Vec<Stats>, _> = serde_json::from_value(actual_stats_json.clone());
    assert!(stats_parse_result.is_err(), "Current Stats structure should fail to parse actual response");
    
    // Example of actual channel status response format
    let actual_channel_status_json = json!({
        "name": "test-rust-sdk-channel",
        "channelId": "test-rust-sdk-channel",
        "status": {
            "isActive": true,
            "occupancy": {
                "metrics": {
                    "connections": 0,
                    "publishers": 0,
                    "subscribers": 0,
                    "presenceConnections": 0,
                    "presenceMembers": 0,
                    "presenceSubscribers": 0,
                    "objectSubscribers": 0,
                    "objectPublishers": 0
                }
            }
        }
    });
    
    // Try to parse with current ChannelStatus structure - check if it works
    let status_parse_result: Result<ChannelStatus, _> = serde_json::from_value(actual_channel_status_json.clone());
    match status_parse_result {
        Ok(_) => println!("ChannelStatus structure works with actual response"),
        Err(e) => println!("ChannelStatus structure fails: {}", e),
    }
}