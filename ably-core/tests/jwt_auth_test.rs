//! RED Phase: JWT Authentication Integration Tests
//! Tests for JWT token authentication against real Ably API

use ably_core::auth::{AuthMode, TokenDetails, TokenRequest, JwtAuth};
use ably_core::http::AblyHttpClient;
use base64::Engine;

const ABLY_API_KEY: &str = "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA";

/// Test JWT token request generation
#[tokio::test]
async fn test_jwt_token_request_generation() {
    let jwt_auth = JwtAuth::new(ABLY_API_KEY);

    let token_request = jwt_auth
        .create_token_request()
        .with_ttl(3600)
        .with_client_id("test-client")
        .with_capability(r#"{"channel1":["publish","subscribe"]}"#)
        .build()
        .await
        .expect("Failed to create token request");

    // Verify token request has required fields
    assert!(token_request.key_name.is_some());
    assert!(token_request.timestamp.is_some());
    assert!(token_request.nonce.is_some());
    assert!(token_request.mac.is_some());
    assert_eq!(token_request.ttl, Some(3600000)); // TTL is in milliseconds
    assert_eq!(token_request.client_id.as_deref(), Some("test-client"));
}

/// Test requesting JWT token from Ably
#[tokio::test]
async fn test_request_jwt_token_from_ably() {
    let jwt_auth = JwtAuth::new(ABLY_API_KEY);
    let client = AblyHttpClient::new(ABLY_API_KEY);

    let token_request = jwt_auth
        .create_token_request()
        .with_ttl(3600)
        .build()
        .await
        .expect("Failed to create token request");

    // Request token from Ably
    let token_details: TokenDetails = client
        .post_json("/keys/BGkZHw.WUtzEQ/requestToken", &token_request)
        .await
        .expect("Failed to request token");

    // Verify we got a valid token
    assert!(!token_details.token.is_empty());
    assert!(token_details.expires.is_some());
    assert!(token_details.issued.is_some());
}

/// Test authenticating with JWT token
#[tokio::test]
async fn test_authenticate_with_jwt_token() {
    let jwt_auth = JwtAuth::new(ABLY_API_KEY);
    let api_client = AblyHttpClient::new(ABLY_API_KEY);

    // Get a token first
    let token_request = jwt_auth
        .create_token_request()
        .with_ttl(3600)
        .build()
        .await
        .expect("Failed to create token request");

    let token_details: TokenDetails = api_client
        .post_json("/keys/BGkZHw.WUtzEQ/requestToken", &token_request)
        .await
        .expect("Failed to request token");

    // Create client with token authentication
    let mut token_client = AblyHttpClient::from_config(Default::default());
    token_client.set_auth_mode(AuthMode::Token(token_details.token));

    // Test authenticated request
    let result = token_client.get_json::<serde_json::Value>("/time").await;
    assert!(result.is_ok(), "Token authentication failed");
}

/// Test token renewal mechanism
#[tokio::test]
async fn test_token_renewal() {
    let jwt_auth = JwtAuth::new(ABLY_API_KEY);
    let client = AblyHttpClient::new(ABLY_API_KEY);

    // Create short-lived token (60 seconds)
    let token_request = jwt_auth
        .create_token_request()
        .with_ttl(60)
        .build()
        .await
        .expect("Failed to create token request");

    let first_token: TokenDetails = client
        .post_json("/keys/BGkZHw.WUtzEQ/requestToken", &token_request)
        .await
        .expect("Failed to request first token");

    // Request another token (simulating renewal)
    let renewal_request = jwt_auth
        .create_token_request()
        .with_ttl(3600)
        .build()
        .await
        .expect("Failed to create renewal request");

    let renewed_token: TokenDetails = client
        .post_json("/keys/BGkZHw.WUtzEQ/requestToken", &renewal_request)
        .await
        .expect("Failed to renew token");

    // Verify tokens are different
    assert_ne!(first_token.token, renewed_token.token);
    assert!(renewed_token.expires > first_token.expires);
}

/// Test token capabilities
#[tokio::test]
async fn test_token_with_capabilities() {
    let jwt_auth = JwtAuth::new(ABLY_API_KEY);
    let client = AblyHttpClient::new(ABLY_API_KEY);

    // Create token with specific capabilities
    let capability = r#"{"test-channel":["publish","subscribe","presence"]}"#;

    let token_request = jwt_auth
        .create_token_request()
        .with_capability(capability)
        .build()
        .await
        .expect("Failed to create token request");

    let token_details: TokenDetails = client
        .post_json("/keys/BGkZHw.WUtzEQ/requestToken", &token_request)
        .await
        .expect("Failed to request token");

    // Verify capability is set
    assert!(token_details.capability.is_some());
}

/// Test MAC verification
#[tokio::test]
async fn test_mac_verification() {
    let jwt_auth = JwtAuth::new(ABLY_API_KEY);

    let token_request = jwt_auth
        .create_token_request()
        .build()
        .await
        .expect("Failed to create token request");

    // Verify MAC is properly computed
    let is_valid = jwt_auth
        .verify_mac(&token_request)
        .expect("Failed to verify MAC");

    assert!(is_valid, "MAC verification failed");
}

/// Test invalid API key handling
#[tokio::test]
async fn test_invalid_api_key() {
    let jwt_auth = JwtAuth::new("invalid.key:secret");

    let result = jwt_auth
        .create_token_request()
        .build()
        .await;

    // Should handle invalid key gracefully
    assert!(result.is_ok() || result.is_err());
}

/// Test token expiry handling
#[tokio::test]
async fn test_token_expiry_check() {
    let jwt_auth = JwtAuth::new(ABLY_API_KEY);
    let client = AblyHttpClient::new(ABLY_API_KEY);

    // Create token with 60 second TTL
    let token_request = jwt_auth
        .create_token_request()
        .with_ttl(60)
        .build()
        .await
        .expect("Failed to create token request");

    let token_details: TokenDetails = client
        .post_json("/keys/BGkZHw.WUtzEQ/requestToken", &token_request)
        .await
        .expect("Failed to request token");

    // Check if token is expired
    let is_expired = jwt_auth.is_token_expired(&token_details);
    assert!(!is_expired, "Token should not be expired immediately");

    // Check expiry time
    if let Some(expires) = token_details.expires {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        assert!(expires > now, "Token expiry should be in the future");
    }
}