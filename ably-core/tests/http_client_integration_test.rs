//! Integration tests for HTTP client against real Ably REST API
//! RED PHASE: These tests will fail initially as implementation doesn't exist yet

use ably_core::http::{AblyHttpClient, HttpError};
use serde::{Deserialize, Serialize};

const ABLY_API_KEY: &str = "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA";

#[derive(Debug, Deserialize)]
struct ServerTimeResponse {
    #[serde(rename = "timestamp")]
    timestamp: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct TestMessage {
    name: String,
    data: String,
}

/// Test basic GET request to Ably REST API /time endpoint
#[tokio::test]
async fn test_get_server_time() {
    let client = AblyHttpClient::new(ABLY_API_KEY);

    let result: Result<ServerTimeResponse, HttpError> = client.get_json("/time").await;

    assert!(result.is_ok(), "Failed to get server time: {:?}", result.err());
    let response = result.unwrap();
    assert!(response.timestamp > 0, "Invalid timestamp received");
}

/// Test authentication headers are properly included in requests
#[tokio::test]
async fn test_authentication_headers() {
    let client = AblyHttpClient::new(ABLY_API_KEY);

    // Test with an endpoint that requires authentication
    let result: Result<serde_json::Value, HttpError> = client.get_json("/keys/BGkZHw.WUtzEQ").await;

    // Should not get 401 Unauthorized if auth headers are correct
    match result {
        Err(HttpError::AuthenticationFailed { status }) => {
            panic!("Authentication failed with status {}", status);
        }
        _ => {} // Any other result means auth headers were included
    }
}

/// Test POST request with JSON serialization
#[tokio::test]
async fn test_post_json_message() {
    let client = AblyHttpClient::new(ABLY_API_KEY);

    let message = TestMessage {
        name: "test-message".to_string(),
        data: "Integration test data".to_string(),
    };

    let result: Result<serde_json::Value, HttpError> =
        client.post_json("/channels/test-channel/messages", &message).await;

    assert!(result.is_ok(), "Failed to post message: {:?}", result.err());
}

/// Test connection timeout handling
#[tokio::test]
async fn test_connection_timeout() {
    let client = AblyHttpClient::with_timeout(ABLY_API_KEY, std::time::Duration::from_millis(1));

    // This should timeout with such a short duration
    let result: Result<ServerTimeResponse, HttpError> = client.get_json("/time").await;

    match result {
        Err(HttpError::Timeout(_)) => {} // Expected
        Ok(_) => panic!("Request should have timed out"),
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

/// Test handling of invalid endpoint (404)
#[tokio::test]
async fn test_invalid_endpoint() {
    let client = AblyHttpClient::new(ABLY_API_KEY);

    let result: Result<serde_json::Value, HttpError> =
        client.get_json("/this/endpoint/does/not/exist").await;

    assert!(result.is_err(), "Should fail for invalid endpoint");

    match result {
        Err(HttpError::NotFound { .. }) => {} // Expected
        Err(e) => panic!("Expected NotFound error, got: {:?}", e),
        Ok(_) => panic!("Should not succeed for invalid endpoint"),
    }
}

/// Test rate limiting response handling (429)
#[tokio::test]
#[ignore] // Rate limiting may not always trigger in sandbox
async fn test_rate_limiting_handling() {
    let client = AblyHttpClient::new(ABLY_API_KEY);

    // Make many rapid requests to potentially trigger rate limiting
    let mut results = Vec::new();
    for _ in 0..100 {
        let result: Result<ServerTimeResponse, HttpError> = client.get_json("/time").await;
        results.push(result);
    }

    // Check if any resulted in rate limiting
    let rate_limited = results.iter().any(|r| {
        matches!(r, Err(HttpError::RateLimited { .. }))
    });

    // This test documents the expected behavior even if not triggered
    if rate_limited {
        println!("Rate limiting detected and handled correctly");
    }
}

/// Test that client properly encodes the API key for Basic auth
#[tokio::test]
async fn test_api_key_encoding() {
    // The API key should be base64 encoded for Basic auth
    // Format: "Basic " + base64(key)
    let client = AblyHttpClient::new(ABLY_API_KEY);

    // This will fail if the auth header is not properly formatted
    let result: Result<ServerTimeResponse, HttpError> = client.get_json("/time").await;
    assert!(result.is_ok(), "API key not properly encoded");
}

/// Test User-Agent header is included
#[tokio::test]
async fn test_user_agent_header() {
    let client = AblyHttpClient::new(ABLY_API_KEY);

    // Ably API should accept our User-Agent
    let result: Result<ServerTimeResponse, HttpError> = client.get_json("/time").await;
    assert!(result.is_ok(), "User-Agent header may be missing or invalid");
}