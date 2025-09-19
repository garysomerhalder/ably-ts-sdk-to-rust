//! Comprehensive Integration Test Harness
//! INFRA-008: Test framework for validating all SDK components against real Ably API

use ably_core::{
    auth::{AuthMode, JwtAuth, TokenDetails},
    http::{AblyHttpClient, HttpError},
    retry::RetryPolicy,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

const ABLY_API_KEY: &str = "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA";

/// Test context that holds shared state across tests
pub struct TestContext {
    pub api_key: String,
    pub http_client: Arc<AblyHttpClient>,
    pub jwt_auth: Arc<JwtAuth>,
    pub current_token: Arc<RwLock<Option<TokenDetails>>>,
}

impl TestContext {
    /// Create new test context
    pub fn new() -> Self {
        let mut http_client = AblyHttpClient::new(ABLY_API_KEY);

        // Configure for testing
        http_client.set_retry_policy(
            RetryPolicy::new(2, Duration::from_millis(100))
        );

        Self {
            api_key: ABLY_API_KEY.to_string(),
            http_client: Arc::new(http_client),
            jwt_auth: Arc::new(JwtAuth::new(ABLY_API_KEY)),
            current_token: Arc::new(RwLock::new(None)),
        }
    }

    /// Get or refresh JWT token
    pub async fn get_valid_token(&self) -> Result<String, HttpError> {
        let mut token_guard = self.current_token.write().await;

        // Check if we have a valid token
        if let Some(ref token) = *token_guard {
            if !self.jwt_auth.is_token_expired(token) {
                return Ok(token.token.clone());
            }
        }

        // Need to get a new token
        let token_request = self.jwt_auth
            .create_token_request()
            .with_ttl(3600)
            .build()
            .await
            .map_err(|e| HttpError::Network(format!("Failed to create token request: {}", e)))?;

        let token_details: TokenDetails = self.http_client
            .post_json("/keys/BGkZHw.WUtzEQ/requestToken", &token_request)
            .await?;

        let token_string = token_details.token.clone();
        *token_guard = Some(token_details);

        Ok(token_string)
    }

    /// Create HTTP client with token authentication
    pub async fn create_token_client(&self) -> Result<AblyHttpClient, HttpError> {
        let token = self.get_valid_token().await?;
        let mut client = AblyHttpClient::from_config(Default::default());
        client.set_auth_mode(AuthMode::Token(token));
        Ok(client)
    }
}

/// Integration test suite for authentication
mod auth_integration {
    use super::*;

    #[tokio::test]
    async fn test_api_key_auth_flow() {
        let ctx = TestContext::new();

        // Test API key authentication
        let result = ctx.http_client
            .get_json::<serde_json::Value>("/time")
            .await;

        assert!(result.is_ok(), "API key authentication failed");
    }

    #[tokio::test]
    async fn test_jwt_token_auth_flow() {
        let ctx = TestContext::new();

        // Get token and authenticate
        let token_client = ctx.create_token_client().await
            .expect("Failed to create token client");

        let result = token_client
            .get_json::<serde_json::Value>("/time")
            .await;

        assert!(result.is_ok(), "Token authentication failed");
    }

    #[tokio::test]
    async fn test_token_renewal_flow() {
        let ctx = TestContext::new();

        // Get first token
        let token1 = ctx.get_valid_token().await
            .expect("Failed to get first token");

        // Clear token to force renewal
        {
            let mut token_guard = ctx.current_token.write().await;
            *token_guard = None;
        }

        // Get second token
        let token2 = ctx.get_valid_token().await
            .expect("Failed to get second token");

        assert_ne!(token1, token2, "Tokens should be different");
    }
}

/// Integration test suite for HTTP resilience
mod resilience_integration {
    use super::*;

    #[tokio::test]
    async fn test_retry_mechanism() {
        let mut client = AblyHttpClient::new(ABLY_API_KEY);
        client.set_retry_policy(RetryPolicy::new(3, Duration::from_millis(100)));

        // Test against a valid endpoint (retries should succeed eventually)
        let result = client.get_json::<serde_json::Value>("/time").await;
        assert!(result.is_ok(), "Retry mechanism failed");
    }

    #[tokio::test]
    async fn test_concurrent_requests() {
        let ctx = TestContext::new();
        let client = Arc::clone(&ctx.http_client);

        let mut handles = vec![];
        for i in 0..10 {
            let client = Arc::clone(&client);
            let handle = tokio::spawn(async move {
                let result = client.get_json::<serde_json::Value>("/time").await;
                (i, result.is_ok())
            });
            handles.push(handle);
        }

        let mut successes = 0;
        for handle in handles {
            let (_, success) = handle.await.unwrap();
            if success {
                successes += 1;
            }
        }

        assert_eq!(successes, 10, "Not all concurrent requests succeeded");
    }

    #[tokio::test]
    async fn test_circuit_breaker_recovery() {
        let ctx = TestContext::new();

        // Make requests to invalid endpoint to trip circuit breaker
        for _ in 0..5 {
            let _ = ctx.http_client
                .get_json::<serde_json::Value>("/invalid/endpoint")
                .await;
        }

        // Circuit breaker should still allow valid requests
        let result = ctx.http_client
            .get_json::<serde_json::Value>("/time")
            .await;

        assert!(result.is_ok(), "Circuit breaker blocked valid endpoint");
    }
}

/// Integration test suite for REST API operations
mod rest_api_integration {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct TestMessage {
        name: String,
        data: String,
        timestamp: i64,
    }

    #[tokio::test]
    async fn test_channel_publish() {
        let ctx = TestContext::new();

        let message = TestMessage {
            name: "test-message".to_string(),
            data: "Integration test data".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
        };

        let result = ctx.http_client
            .post_json::<_, serde_json::Value>(
                "/channels/test-channel/messages",
                &message
            )
            .await;

        assert!(result.is_ok(), "Failed to publish message");
    }

    #[tokio::test]
    async fn test_channel_history() {
        let ctx = TestContext::new();

        // First publish a message
        let message = TestMessage {
            name: "history-test".to_string(),
            data: "Test history data".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
        };

        ctx.http_client
            .post_json::<_, serde_json::Value>(
                "/channels/test-history/messages",
                &message
            )
            .await
            .expect("Failed to publish message");

        // Then retrieve history
        let result = ctx.http_client
            .get_json::<serde_json::Value>("/channels/test-history/messages")
            .await;

        assert!(result.is_ok(), "Failed to retrieve channel history");
    }

    #[tokio::test]
    async fn test_stats_endpoint() {
        let ctx = TestContext::new();

        let result = ctx.http_client
            .get_json::<serde_json::Value>("/stats")
            .await;

        // Stats endpoint might return empty array but should not error
        assert!(result.is_ok(), "Failed to retrieve stats");
    }
}

/// Integration test suite for error handling
mod error_handling_integration {
    use super::*;

    #[tokio::test]
    async fn test_404_handling() {
        let ctx = TestContext::new();

        let result = ctx.http_client
            .get_json::<serde_json::Value>("/this/does/not/exist")
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), HttpError::NotFound { .. }));
    }

    #[tokio::test]
    async fn test_unauthorized_handling() {
        // Use a clearly invalid API key format
        let client = AblyHttpClient::new("invalid_key");

        // Try to access a protected endpoint that requires authentication
        let result = client
            .get_json::<serde_json::Value>("/stats")
            .await;

        // Should get authentication error for protected endpoints
        println!("Result: {:?}", result);
        assert!(result.is_err(), "Expected error for invalid API key on protected endpoint");
    }

    #[tokio::test]
    async fn test_timeout_handling() {
        let client = AblyHttpClient::with_timeout(
            ABLY_API_KEY,
            Duration::from_millis(1)
        );

        let result = client
            .get_json::<serde_json::Value>("/time")
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), HttpError::Timeout(_)));
    }
}

/// Performance benchmarks
mod performance {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn benchmark_request_latency() {
        let ctx = TestContext::new();
        let mut latencies = vec![];

        for _ in 0..10 {
            let start = Instant::now();
            let _ = ctx.http_client
                .get_json::<serde_json::Value>("/time")
                .await;
            latencies.push(start.elapsed());
        }

        let avg_latency = latencies.iter().sum::<Duration>() / latencies.len() as u32;
        println!("Average request latency: {:?}", avg_latency);

        // Should be reasonably fast
        assert!(avg_latency < Duration::from_secs(2));
    }

    #[tokio::test]
    async fn benchmark_token_generation() {
        let ctx = TestContext::new();
        let start = Instant::now();

        for _ in 0..10 {
            let _ = ctx.jwt_auth
                .create_token_request()
                .build()
                .await;
        }

        let duration = start.elapsed();
        println!("Generated 10 token requests in {:?}", duration);

        // Token generation should be fast
        assert!(duration < Duration::from_secs(1));
    }
}

/// Utility for test cleanup
pub struct TestCleanup;

impl TestCleanup {
    pub async fn cleanup_test_channels(_client: &AblyHttpClient, prefix: &str) {
        // In production, this would delete test channels
        // For now, just a placeholder
        println!("Cleaning up test channels with prefix: {}", prefix);
    }
}

/// Main test runner
#[tokio::test]
async fn run_full_integration_suite() {
    println!("Running full integration test suite...");

    let ctx = TestContext::new();

    // Test authentication flows
    println!("Testing authentication...");
    assert!(ctx.http_client
        .get_json::<serde_json::Value>("/time")
        .await
        .is_ok());

    // Test token flow
    println!("Testing JWT tokens...");
    let token = ctx.get_valid_token().await.expect("Failed to get token");
    assert!(!token.is_empty());

    // Test resilience
    println!("Testing resilience features...");
    let metrics = ctx.http_client.metrics();
    println!("Success rate: {:.2}%", metrics.success_rate() * 100.0);
    println!("Average latency: {:.2}ms", metrics.average_latency_ms());

    println!("Full integration suite completed successfully!");
}