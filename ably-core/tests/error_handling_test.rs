// 🔴 RED Phase: Error handling tests that MUST fail initially
// Testing real Ably API error scenarios

use ably_core::error::{AblyError, ErrorCode, ErrorCategory};
use ably_core::client::RestClient;

#[tokio::test]
async fn test_authentication_error_401() {
    // Invalid API key should trigger 401
    let invalid_key = "invalid.key:secret";
    let client = RestClient::new(invalid_key);
    
    let result = client.get_server_time().await;
    
    assert!(result.is_err());
    let err = result.unwrap_err();
    
    // Should be categorized as Auth error
    assert!(matches!(err, AblyError::Authentication { .. }));
    assert_eq!(err.code(), Some(ErrorCode::Unauthorized));
    assert_eq!(err.category(), ErrorCategory::Auth);
}

#[tokio::test]
async fn test_rate_limiting_error_429() {
    // Test that rate limiting error is properly handled
    // In YELLOW phase, we'll test the error type conversion
    let error = AblyError::from_ably_code(42900, "Rate limit exceeded");
    assert!(matches!(error, AblyError::RateLimited { .. }));
    assert_eq!(error.category(), ErrorCategory::RateLimit);
    assert!(error.is_retryable());
}

#[tokio::test]
async fn test_network_timeout_error() {
    // Create client with very short timeout
    let client = RestClient::builder()
        .api_key(get_test_api_key())
        .timeout_ms(1) // 1ms timeout will definitely fail
        .build();
    
    let result = client.get_server_time().await;
    
    assert!(result.is_err());
    let err = result.unwrap_err();
    
    assert!(matches!(err, AblyError::Network { .. }));
    assert_eq!(err.category(), ErrorCategory::Network);
    assert!(err.is_retryable());
}

#[tokio::test]
async fn test_malformed_response_handling() {
    // Test handling of unexpected response format
    let client = RestClient::new(get_test_api_key());
    
    // Try to parse channel stats as server time (wrong type)
    let result = client.request_raw("/stats", "GET").await
        .and_then(|body| client.parse_as::<i64>(&body));
    
    assert!(result.is_err());
    let err = result.unwrap_err();
    
    assert!(matches!(err, AblyError::Decode { .. }));
    assert!(!err.is_retryable());
}

#[tokio::test]
async fn test_error_context_preservation() {
    let client = RestClient::new("invalid.key:secret");
    
    let result = client.publish_message("test-channel", "test-message").await;
    
    assert!(result.is_err());
    let err = result.unwrap_err();
    
    // Error should preserve context
    assert!(err.context().contains("channel"));
    assert!(err.context().contains("test-channel"));
    
    // Should have backtrace in debug mode
    #[cfg(debug_assertions)]
    assert!(err.backtrace().is_some());
}

#[tokio::test]
async fn test_retry_with_exponential_backoff() {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    
    let attempt_count = Arc::new(AtomicU32::new(0));
    let count_clone = attempt_count.clone();
    
    // Simulate operation that fails twice then succeeds
    let operation = move || {
        let count = count_clone.clone();
        async move {
            let c = count.fetch_add(1, Ordering::SeqCst);
            if c < 2 {
                Err(AblyError::Network { 
                    message: "Transient error".into(),
                    source: None,
                    retryable: true,
                })
            } else {
                Ok("Success")
            }
        }
    };
    
    let retry_policy = ably_core::retry::RetryPolicy::default();
    let result = retry_policy.execute_async(operation).await;
    
    assert!(result.is_ok());
    assert_eq!(attempt_count.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn test_circuit_breaker_activation() {
    let client = RestClient::new("invalid.key:secret");
    
    // Make multiple failing requests
    for _ in 0..5 {
        let _ = client.get_server_time().await;
    }
    
    // Circuit breaker should be open
    let result = client.get_server_time().await;
    assert!(result.is_err());
    
    let err = result.unwrap_err();
    assert!(matches!(err, AblyError::CircuitBreakerOpen { .. }));
}

#[tokio::test]
async fn test_error_code_mapping() {
    // Test that Ably error codes are properly mapped
    let test_cases = vec![
        (40000, ErrorCategory::BadRequest),
        (40100, ErrorCategory::Auth),
        (40300, ErrorCategory::Forbidden),
        (40400, ErrorCategory::NotFound),
        (42900, ErrorCategory::RateLimit),
        (50000, ErrorCategory::Internal),
    ];
    
    for (code, expected_category) in test_cases {
        let error = AblyError::from_ably_code(code, "Test error");
        assert_eq!(error.category(), expected_category);
        assert_eq!(error.code().unwrap().as_u16(), code);
    }
}

// Helper function - would be in common module
fn get_test_api_key() -> String {
    std::env::var("ABLY_API_KEY_SANDBOX")
        .unwrap_or_else(|_| "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA".to_string())
}