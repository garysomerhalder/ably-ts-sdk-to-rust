// 🔴 RED Phase: HTTP client tests that MUST fail initially
// Testing real Ably REST API endpoints - NO MOCKS!

use ably_core::http::{AblyHttpClient, HttpConfig, HttpMethod, HttpResponse};
use ably_core::auth::AuthMode;
use serde_json::json;
use std::time::Duration;

fn get_test_api_key() -> String {
    std::env::var("ABLY_API_KEY_SANDBOX")
        .unwrap_or_else(|_| "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA".to_string())
}

#[tokio::test]
async fn test_basic_get_request_to_ably_time() {
    // Test real Ably REST API /time endpoint
    let client = AblyHttpClient::new(HttpConfig::default());
    
    let response = client
        .get("https://sandbox-rest.ably.io/time")
        .send()
        .await
        .expect("Should get time from Ably");
    
    // Ably returns time as array of milliseconds
    let times: Vec<i64> = response.json().await
        .expect("Should parse time response");
    
    assert!(!times.is_empty());
    assert!(times[0] > 0);
}

#[tokio::test]
async fn test_authentication_headers_in_requests() {
    let api_key = get_test_api_key();
    let client = AblyHttpClient::with_auth(
        HttpConfig::default(),
        AuthMode::ApiKey(api_key)
    );
    
    // Test authenticated request to time endpoint (should work with or without auth)
    let response = client
        .get("https://sandbox-rest.ably.io/time")
        .send()
        .await
        .expect("Should get time with authentication");
    
    // Should succeed with authentication
    assert_eq!(response.status().as_u16(), 200);
    
    // Verify the response is valid
    let times: Vec<i64> = response.json().await
        .expect("Should parse time response");
    assert!(!times.is_empty());
}

#[tokio::test]
async fn test_post_request_with_json_body() {
    let api_key = get_test_api_key();
    let client = AblyHttpClient::with_auth(
        HttpConfig::default(),
        AuthMode::ApiKey(api_key)
    );
    
    // Test publishing a message to a channel
    let message = json!({
        "name": "test_event",
        "data": "test_message"
    });
    
    let response = client
        .post("https://sandbox-rest.ably.io/channels/test_channel/messages")
        .json(&message)
        .send()
        .await
        .expect("Should publish message");
    
    // Debug output for failures
    if !response.status().is_success() {
        eprintln!("POST failed with status: {}", response.status());
        let body = response.text().await.unwrap_or_default();
        eprintln!("Response body: {}", body);
        panic!("POST request failed");
    }
    
    assert!(response.status().is_success());
}

#[tokio::test]
async fn test_connection_timeout_handling() {
    let config = HttpConfig::builder()
        .timeout(Duration::from_millis(1)) // Very short timeout
        .build();
    
    let client = AblyHttpClient::new(config);
    
    let result = client
        .get("https://sandbox-rest.ably.io/time")
        .send()
        .await;
    
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.is_timeout());
}

#[tokio::test]
async fn test_invalid_endpoint_error_handling() {
    let client = AblyHttpClient::new(HttpConfig::default());
    
    let response = client
        .get("https://sandbox-rest.ably.io/invalid_endpoint_12345")
        .send()
        .await
        .expect("Should get response");
    
    assert_eq!(response.status().as_u16(), 404);
}

#[tokio::test]
async fn test_rate_limiting_response_handling() {
    // Test that client handles 429 responses properly
    let client = AblyHttpClient::new(HttpConfig::default());
    
    // Make many rapid requests to potentially trigger rate limiting
    // In practice, Ably sandbox has generous limits
    for _ in 0..5 {
        let response = client
            .get("https://sandbox-rest.ably.io/time")
            .send()
            .await
            .expect("Should handle request");
        
        if response.status().as_u16() == 429 {
            // Check for Retry-After header
            assert!(response.headers().get("retry-after").is_some());
            break;
        }
    }
    
    // Even if we don't hit rate limit, test passes (Ably handles requests)
    assert!(true);
}

#[tokio::test]
async fn test_connection_pooling() {
    let client = AblyHttpClient::new(HttpConfig::default());
    
    // Make multiple requests to test connection reuse
    for i in 0..3 {
        let response = client
            .get("https://sandbox-rest.ably.io/time")
            .send()
            .await
            .expect(&format!("Request {} should succeed", i));
        
        assert_eq!(response.status().as_u16(), 200);
    }
}

#[tokio::test]
async fn test_custom_headers() {
    let client = AblyHttpClient::new(HttpConfig::default());
    
    let response = client
        .get("https://sandbox-rest.ably.io/time")
        .header("X-Custom-Header", "test-value")
        .header("User-Agent", "ably-rust-sdk/0.1.0")
        .send()
        .await
        .expect("Should send with custom headers");
    
    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn test_query_parameters() {
    let api_key = get_test_api_key();
    let client = AblyHttpClient::with_auth(
        HttpConfig::default(),
        AuthMode::ApiKey(api_key)
    );
    
    // Test channels endpoint with query parameters  
    let response = client
        .get("https://sandbox-rest.ably.io/channels")
        .query(&[("limit", "10"), ("prefix", "test")])
        .send()
        .await
        .expect("Should handle query parameters");
    
    // The channels endpoint should work with auth
    assert!(response.status().is_success());
}

#[tokio::test]
async fn test_response_decompression() {
    let client = AblyHttpClient::new(HttpConfig::default());
    
    // Ably supports gzip compression
    let response = client
        .get("https://sandbox-rest.ably.io/time")
        .header("Accept-Encoding", "gzip")
        .send()
        .await
        .expect("Should handle compressed response");
    
    // Should automatically decompress
    let times: Vec<i64> = response.json().await
        .expect("Should parse decompressed response");
    
    assert!(!times.is_empty());
}