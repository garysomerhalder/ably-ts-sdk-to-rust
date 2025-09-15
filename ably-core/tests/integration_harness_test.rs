// 🟢 GREEN Phase: Advanced integration test using production-ready harness
// Tests the full test infrastructure with real Ably API

mod common;

use common::*;
use common::test_harness::*;
use std::time::{Duration, Instant};

#[tokio::test]
async fn test_rate_limited_api_calls() {
    // Set up rate limiter (10 calls per second, burst of 20)
    let rate_limiter = RateLimiter::new(10, 20);
    let client = create_test_client().await.expect("Should create client");
    
    // Make rapid API calls with rate limiting
    let start = Instant::now();
    for i in 0..25 {
        rate_limiter.acquire().await;
        
        let result = client.get_server_time().await;
        assert!(result.is_ok(), "API call {} should succeed", i);
    }
    let elapsed = start.elapsed();
    
    // Should have taken at least 0.5 seconds due to rate limiting
    // (25 calls - 20 burst = 5 calls that need to wait, at 10/sec = 0.5 sec)
    assert!(elapsed >= Duration::from_millis(400), "Rate limiting should slow down requests");
}

#[tokio::test]
async fn test_health_check_with_retry() {
    let health_checker = HealthChecker::new("https://sandbox-rest.ably.io/time".to_string());
    
    // Should succeed quickly for healthy endpoint
    health_checker.wait_for_healthy(3).await
        .expect("Ably sandbox should be healthy");
}

#[tokio::test]
async fn test_cleanup_tracking() {
    let tracker = CleanupTracker::new();
    let test_id = generate_test_id();
    
    // Register multiple resources
    for i in 0..5 {
        let resource = TestResource {
            id: format!("{}_{}", test_id, i),
            resource_type: ResourceType::Channel(format!("test_channel_{}", i)),
            created_at: Instant::now(),
        };
        tracker.register(resource).await;
    }
    
    assert_eq!(tracker.get_resource_count().await, 5);
    
    // Cleanup should remove all resources
    tracker.cleanup_all().await.expect("Cleanup should succeed");
    assert_eq!(tracker.get_resource_count().await, 0);
}

#[tokio::test]
async fn test_namespace_isolation() {
    let generator = NamespaceGenerator::new("ably_rust_test".to_string());
    
    // Generate multiple namespaces
    let namespaces: Vec<String> = futures::future::join_all(
        (0..10).map(|_| generator.generate())
    ).await;
    
    // All should be unique
    let unique_count = namespaces.iter().collect::<std::collections::HashSet<_>>().len();
    assert_eq!(unique_count, 10, "All namespaces should be unique");
    
    // All should have correct prefix
    for ns in &namespaces {
        assert!(ns.starts_with("ably_rust_test_"), "Namespace should have correct prefix");
    }
}

#[tokio::test]
async fn test_retry_policy() {
    let retry_policy = RetryPolicy::new();
    let mut attempt_count = 0;
    
    // Test successful retry after 2 failures
    let result = retry_policy.execute(|| {
        attempt_count += 1;
        if attempt_count <= 2 {
            Err("Simulated failure")
        } else {
            Ok("Success")
        }
    }).await;
    
    assert_eq!(result, Ok("Success"));
    assert_eq!(attempt_count, 3);
}

#[tokio::test]
async fn test_metrics_collection() {
    let metrics = MetricsCollector::new();
    
    // Simulate test execution
    let test_start = Instant::now();
    
    metrics.increment_api_calls().await;
    metrics.increment_api_calls().await;
    metrics.increment_api_calls().await;
    
    let duration = test_start.elapsed();
    metrics.record_test("test_example".to_string(), duration).await;
    
    // Generate report (check it doesn't panic)
    metrics.report().await;
}

#[tokio::test]
async fn test_full_integration_workflow() {
    // This test demonstrates a complete integration test workflow
    // using all the production-ready features
    
    // 1. Set up infrastructure
    let rate_limiter = RateLimiter::new(10, 20);
    let health_checker = HealthChecker::new("https://sandbox-rest.ably.io/time".to_string());
    let cleanup_tracker = CleanupTracker::new();
    let namespace_gen = NamespaceGenerator::new("integration_test".to_string());
    let metrics = MetricsCollector::new();
    
    let test_start = Instant::now();
    
    // 2. Check service health
    health_checker.wait_for_healthy(3).await
        .expect("Service should be healthy");
    
    // 3. Create test client
    let client = create_test_client().await
        .expect("Should create client");
    
    // 4. Generate isolated namespace
    let namespace = namespace_gen.generate().await;
    
    // 5. Register resources for cleanup
    let resource = TestResource {
        id: namespace.clone(),
        resource_type: ResourceType::Channel(namespace.clone()),
        created_at: Instant::now(),
    };
    cleanup_tracker.register(resource).await;
    
    // 6. Perform rate-limited API calls
    for _ in 0..3 {
        rate_limiter.acquire().await;
        metrics.increment_api_calls().await;
        
        let result = client.get_server_time().await;
        assert!(result.is_ok());
    }
    
    // 7. Clean up resources
    cleanup_tracker.cleanup_all().await
        .expect("Cleanup should succeed");
    
    // 8. Record metrics
    let duration = test_start.elapsed();
    metrics.record_test("full_integration_workflow".to_string(), duration).await;
    metrics.report().await;
}

// Add futures for join_all
use futures;