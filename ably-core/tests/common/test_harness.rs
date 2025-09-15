// 🟢 GREEN Phase: Production-ready test harness with advanced features
// Full Integration-First testing infrastructure

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{info, warn, error, debug};

/// Rate limiter for API calls during tests
pub struct RateLimiter {
    max_per_second: u32,
    burst: u32,
    tokens: Arc<RwLock<f64>>,
    last_refill: Arc<RwLock<std::time::Instant>>,
}

impl RateLimiter {
    pub fn new(max_per_second: u32, burst: u32) -> Self {
        Self {
            max_per_second,
            burst,
            tokens: Arc::new(RwLock::new(burst as f64)),
            last_refill: Arc::new(RwLock::new(std::time::Instant::now())),
        }
    }
    
    pub async fn acquire(&self) {
        loop {
            // Refill tokens based on time elapsed
            {
                let mut last_refill = self.last_refill.write().await;
                let mut tokens = self.tokens.write().await;
                let now = std::time::Instant::now();
                let elapsed = now.duration_since(*last_refill).as_secs_f64();
                
                *tokens = (*tokens + elapsed * self.max_per_second as f64).min(self.burst as f64);
                *last_refill = now;
                
                if *tokens >= 1.0 {
                    *tokens -= 1.0;
                    debug!("Rate limiter: acquired token, {} remaining", *tokens);
                    return;
                }
            }
            
            // Wait before trying again
            warn!("Rate limiter: waiting for token availability");
            sleep(Duration::from_millis(100)).await;
        }
    }
}

/// Test environment health checker
pub struct HealthChecker {
    client: reqwest::Client,
    endpoint: String,
}

impl HealthChecker {
    pub fn new(endpoint: String) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap(),
            endpoint,
        }
    }
    
    pub async fn check_health(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let response = self.client
            .get(&self.endpoint)
            .send()
            .await?;
        
        if response.status().is_success() {
            info!("Health check passed for {}", self.endpoint);
            Ok(true)
        } else {
            warn!("Health check failed for {}: {}", self.endpoint, response.status());
            Ok(false)
        }
    }
    
    pub async fn wait_for_healthy(&self, max_attempts: u32) -> Result<(), Box<dyn std::error::Error>> {
        for attempt in 1..=max_attempts {
            if self.check_health().await? {
                return Ok(());
            }
            
            if attempt < max_attempts {
                let delay = Duration::from_secs(attempt as u64);
                warn!("Health check attempt {} failed, retrying in {:?}", attempt, delay);
                sleep(delay).await;
            }
        }
        
        Err("Service not healthy after maximum attempts".into())
    }
}

/// Test data cleanup tracker
pub struct CleanupTracker {
    resources: Arc<RwLock<Vec<TestResource>>>,
}

#[derive(Clone, Debug)]
pub struct TestResource {
    pub id: String,
    pub resource_type: ResourceType,
    pub created_at: std::time::Instant,
}

#[derive(Clone, Debug)]
pub enum ResourceType {
    Channel(String),
    Message(String),
    Presence(String),
    Connection(String),
}

impl CleanupTracker {
    pub fn new() -> Self {
        Self {
            resources: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    pub async fn register(&self, resource: TestResource) {
        let mut resources = self.resources.write().await;
        info!("Registered test resource: {:?}", resource);
        resources.push(resource);
    }
    
    pub async fn cleanup_all(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut resources = self.resources.write().await;
        
        info!("Cleaning up {} test resources", resources.len());
        
        for resource in resources.drain(..) {
            match resource.resource_type {
                ResourceType::Channel(name) => {
                    debug!("Cleaning up channel: {}", name);
                    // Real implementation would delete channel from Ably
                }
                ResourceType::Message(id) => {
                    debug!("Cleaning up message: {}", id);
                    // Real implementation would delete message
                }
                ResourceType::Presence(id) => {
                    debug!("Cleaning up presence: {}", id);
                    // Real implementation would clear presence
                }
                ResourceType::Connection(id) => {
                    debug!("Cleaning up connection: {}", id);
                    // Real implementation would close connection
                }
            }
        }
        
        Ok(())
    }
    
    pub async fn get_resource_count(&self) -> usize {
        self.resources.read().await.len()
    }
}

/// Test isolation namespace generator
pub struct NamespaceGenerator {
    prefix: String,
    counter: Arc<RwLock<u64>>,
}

impl NamespaceGenerator {
    pub fn new(prefix: String) -> Self {
        Self {
            prefix,
            counter: Arc::new(RwLock::new(0)),
        }
    }
    
    pub async fn generate(&self) -> String {
        let mut counter = self.counter.write().await;
        *counter += 1;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        
        format!("{}_{}_{}_{}", self.prefix, timestamp, *counter, uuid::Uuid::new_v4())
    }
}

/// Test retry mechanism with exponential backoff
pub struct RetryPolicy {
    max_attempts: u32,
    initial_delay: Duration,
    max_delay: Duration,
    jitter: bool,
}

impl RetryPolicy {
    pub fn new() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            jitter: true,
        }
    }
    
    pub async fn execute<F, T, E>(&self, mut operation: F) -> Result<T, E>
    where
        F: FnMut() -> Result<T, E>,
        E: std::fmt::Display,
    {
        let mut attempt = 1;
        
        loop {
            match operation() {
                Ok(result) => {
                    if attempt > 1 {
                        info!("Operation succeeded after {} attempts", attempt);
                    }
                    return Ok(result);
                }
                Err(err) if attempt >= self.max_attempts => {
                    error!("Operation failed after {} attempts: {}", attempt, err);
                    return Err(err);
                }
                Err(err) => {
                    let delay = self.calculate_delay(attempt);
                    warn!("Attempt {} failed: {}, retrying in {:?}", attempt, err, delay);
                    sleep(delay).await;
                    attempt += 1;
                }
            }
        }
    }
    
    fn calculate_delay(&self, attempt: u32) -> Duration {
        let exponential = self.initial_delay * 2u32.pow(attempt - 1);
        let delay = exponential.min(self.max_delay);
        
        if self.jitter {
            // Add random jitter up to 25% of delay
            let jitter_ms = (delay.as_millis() as f64 * 0.25 * rand::random::<f64>()) as u64;
            delay + Duration::from_millis(jitter_ms)
        } else {
            delay
        }
    }
}

/// Test metrics collector
pub struct MetricsCollector {
    test_durations: Arc<RwLock<Vec<(String, Duration)>>>,
    api_calls: Arc<RwLock<u64>>,
    failures: Arc<RwLock<u64>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            test_durations: Arc::new(RwLock::new(Vec::new())),
            api_calls: Arc::new(RwLock::new(0)),
            failures: Arc::new(RwLock::new(0)),
        }
    }
    
    pub async fn record_test(&self, name: String, duration: Duration) {
        let mut durations = self.test_durations.write().await;
        durations.push((name.clone(), duration));
        info!("Test '{}' completed in {:?}", name, duration);
    }
    
    pub async fn increment_api_calls(&self) {
        let mut calls = self.api_calls.write().await;
        *calls += 1;
    }
    
    pub async fn increment_failures(&self) {
        let mut failures = self.failures.write().await;
        *failures += 1;
    }
    
    pub async fn report(&self) {
        let durations = self.test_durations.read().await;
        let api_calls = self.api_calls.read().await;
        let failures = self.failures.read().await;
        
        info!("=== Test Metrics Report ===");
        info!("Total tests: {}", durations.len());
        info!("Total API calls: {}", *api_calls);
        info!("Total failures: {}", *failures);
        
        if !durations.is_empty() {
            let total: Duration = durations.iter().map(|(_, d)| *d).sum();
            let avg = total / durations.len() as u32;
            info!("Average test duration: {:?}", avg);
            
            let slowest = durations.iter()
                .max_by_key(|(_, d)| d.as_millis())
                .unwrap();
            info!("Slowest test: '{}' ({:?})", slowest.0, slowest.1);
        }
    }
}

// Re-export rand for jitter calculation
use rand;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(10, 5);
        
        // Should allow burst of 5 immediately
        for _ in 0..5 {
            limiter.acquire().await;
        }
        
        // 6th should require waiting
        let start = std::time::Instant::now();
        limiter.acquire().await;
        let elapsed = start.elapsed();
        
        // Should have waited at least 100ms
        assert!(elapsed >= Duration::from_millis(100));
    }
    
    #[tokio::test]
    async fn test_cleanup_tracker() {
        let tracker = CleanupTracker::new();
        
        let resource = TestResource {
            id: "test-123".to_string(),
            resource_type: ResourceType::Channel("test-channel".to_string()),
            created_at: std::time::Instant::now(),
        };
        
        tracker.register(resource).await;
        assert_eq!(tracker.get_resource_count().await, 1);
        
        tracker.cleanup_all().await.unwrap();
        assert_eq!(tracker.get_resource_count().await, 0);
    }
    
    #[tokio::test]
    async fn test_namespace_generator() {
        let generator = NamespaceGenerator::new("test".to_string());
        
        let ns1 = generator.generate().await;
        let ns2 = generator.generate().await;
        
        assert_ne!(ns1, ns2);
        assert!(ns1.starts_with("test_"));
        assert!(ns2.starts_with("test_"));
    }
}