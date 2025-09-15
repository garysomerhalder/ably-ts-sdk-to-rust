// 🟢 GREEN Phase: Production-ready resilience features for HTTP client

use crate::error::{AblyError, AblyResult};
// use crate::retry::{RetryPolicy, ExponentialBackoff}; // TODO: Implement when needed
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, warn, error, instrument};

/// Circuit breaker for HTTP requests
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    failure_threshold: u32,
    recovery_timeout: Duration,
    half_open_requests: u32,
    state: Arc<RwLock<CircuitBreakerState>>,
    failure_count: Arc<AtomicU32>,
    success_count: Arc<AtomicU32>,
    last_failure_time: Arc<RwLock<Option<Instant>>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, recovery_timeout: Duration) -> Self {
        Self {
            failure_threshold,
            recovery_timeout,
            half_open_requests: 3,
            state: Arc::new(RwLock::new(CircuitBreakerState::Closed)),
            failure_count: Arc::new(AtomicU32::new(0)),
            success_count: Arc::new(AtomicU32::new(0)),
            last_failure_time: Arc::new(RwLock::new(None)),
        }
    }

    #[instrument(skip(self, f))]
    pub async fn call<F, T>(&self, f: F) -> AblyResult<T>
    where
        F: std::future::Future<Output = AblyResult<T>>,
    {
        {
            let state = self.state.read().await;
            
            match *state {
                CircuitBreakerState::Open => {
                    // Check if we should transition to half-open
                    if let Some(last_failure) = *self.last_failure_time.read().await {
                        if last_failure.elapsed() >= self.recovery_timeout {
                            drop(state);
                            let mut state = self.state.write().await;
                            *state = CircuitBreakerState::HalfOpen;
                            debug!("Circuit breaker transitioning to half-open");
                        } else {
                            return Err(AblyError::CircuitBreakerOpen {
                                message: "Circuit breaker is open".to_string(),
                            });
                        }
                    } else {
                        return Err(AblyError::CircuitBreakerOpen {
                            message: "Circuit breaker is open".to_string(),
                        });
                    }
                }
                CircuitBreakerState::HalfOpen => {
                    // Allow limited requests through
                    let success_count = self.success_count.load(Ordering::SeqCst);
                    if success_count >= self.half_open_requests {
                        drop(state);
                        let mut state = self.state.write().await;
                        *state = CircuitBreakerState::Closed;
                        self.failure_count.store(0, Ordering::SeqCst);
                        self.success_count.store(0, Ordering::SeqCst);
                        debug!("Circuit breaker closed after successful half-open period");
                    }
                }
                CircuitBreakerState::Closed => {}
            }
        }

        // Execute the request
        match f.await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(e) => {
                self.on_failure().await;
                Err(e)
            }
        }
    }

    async fn on_success(&self) {
        let state = self.state.read().await;
        match *state {
            CircuitBreakerState::HalfOpen => {
                self.success_count.fetch_add(1, Ordering::SeqCst);
            }
            CircuitBreakerState::Closed => {
                // Reset failure count on success in closed state
                self.failure_count.store(0, Ordering::SeqCst);
            }
            _ => {}
        }
    }

    async fn on_failure(&self) {
        let failures = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
        
        let mut last_failure = self.last_failure_time.write().await;
        *last_failure = Some(Instant::now());
        drop(last_failure);

        if failures >= self.failure_threshold {
            let mut state = self.state.write().await;
            if *state != CircuitBreakerState::Open {
                *state = CircuitBreakerState::Open;
                warn!("Circuit breaker opened after {} failures", failures);
            }
        }
    }
}

/// Rate limiter for HTTP requests
#[derive(Debug, Clone)]
pub struct RateLimiter {
    max_requests: u32,
    window: Duration,
    requests: Arc<RwLock<Vec<Instant>>>,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            requests: Arc::new(RwLock::new(Vec::new())),
        }
    }

    #[instrument(skip(self))]
    pub async fn check_rate_limit(&self) -> AblyResult<()> {
        let now = Instant::now();
        let mut requests = self.requests.write().await;
        
        // Remove old requests outside the window
        requests.retain(|&req_time| now.duration_since(req_time) < self.window);
        
        if requests.len() >= self.max_requests as usize {
            let oldest = requests.first().copied();
            if let Some(oldest_time) = oldest {
                let wait_time = self.window - now.duration_since(oldest_time);
                return Err(AblyError::RateLimited {
                    message: format!("Rate limit exceeded: {} requests in {:?}", self.max_requests, self.window),
                    retry_after: Some(wait_time),
                });
            }
        }
        
        requests.push(now);
        Ok(())
    }
}

/// Request interceptor for adding common headers and logging
pub struct RequestInterceptor {
    client_id: String,
    sdk_version: String,
}

impl RequestInterceptor {
    pub fn new(client_id: String) -> Self {
        Self {
            client_id,
            sdk_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    pub fn add_headers(&self, mut headers: reqwest::header::HeaderMap) -> reqwest::header::HeaderMap {
        headers.insert(
            "X-Ably-Version",
            "3".parse().unwrap(),
        );
        headers.insert(
            "X-Ably-Lib",
            format!("rust-{}", self.sdk_version).parse().unwrap(),
        );
        headers.insert(
            "X-Ably-ClientId",
            self.client_id.parse().unwrap_or_else(|_| "unknown".parse().unwrap()),
        );
        headers
    }
}

/// Response validator for checking Ably-specific error codes
pub struct ResponseValidator;

impl ResponseValidator {
    #[instrument(skip(response))]
    pub async fn validate(response: reqwest::Response) -> AblyResult<reqwest::Response> {
        let status = response.status();
        
        if status.is_success() {
            return Ok(response);
        }

        // Try to parse Ably error response
        if let Ok(error_body) = response.json::<AblyErrorResponse>().await {
            let error = AblyError::from_ably_code(
                error_body.error.code,
                &error_body.error.message,
            );
            error!("Ably API error: {} - {}", error_body.error.code, error_body.error.message);
            return Err(error);
        }

        // Generic HTTP error
        match status.as_u16() {
            429 => Err(AblyError::RateLimited {
                message: "Rate limit exceeded".to_string(),
                retry_after: None,
            }),
            401 | 403 => Err(AblyError::Authentication {
                message: format!("Authentication failed: {}", status),
                code: None,
            }),
            500..=599 => Err(AblyError::Internal {
                message: format!("Server error: {}", status),
            }),
            _ => Err(AblyError::Api {
                code: status.as_u16(),
                message: format!("HTTP error: {}", status),
            }),
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct AblyErrorResponse {
    error: AblyErrorDetail,
}

#[derive(Debug, serde::Deserialize)]
struct AblyErrorDetail {
    code: u16,
    message: String,
    #[allow(dead_code)]
    href: Option<String>,
}

/// Connection pool metrics
#[derive(Debug, Default)]
pub struct ConnectionMetrics {
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,
    total_latency_ms: AtomicU64,
}

impl ConnectionMetrics {
    pub fn record_request(&self, success: bool, latency: Duration) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        
        if success {
            self.successful_requests.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_requests.fetch_add(1, Ordering::Relaxed);
        }
        
        self.total_latency_ms.fetch_add(
            latency.as_millis() as u64,
            Ordering::Relaxed
        );
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.total_requests.load(Ordering::Relaxed);
        if total == 0 {
            return 1.0;
        }
        
        let successful = self.successful_requests.load(Ordering::Relaxed);
        successful as f64 / total as f64
    }

    pub fn average_latency_ms(&self) -> f64 {
        let total = self.total_requests.load(Ordering::Relaxed);
        if total == 0 {
            return 0.0;
        }
        
        let total_latency = self.total_latency_ms.load(Ordering::Relaxed);
        total_latency as f64 / total as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_opens_after_threshold() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(1));
        
        // Simulate failures
        for _ in 0..3 {
            let _ = cb.call(async {
                Err::<(), _>(AblyError::network("Network error"))
            }).await;
        }
        
        // Circuit should be open now
        let result = cb.call(async {
            Ok::<_, AblyError>(())
        }).await;
        
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AblyError::CircuitBreakerOpen { .. }));
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(2, Duration::from_secs(1));
        
        // First two requests should succeed
        assert!(limiter.check_rate_limit().await.is_ok());
        assert!(limiter.check_rate_limit().await.is_ok());
        
        // Third request should fail
        let result = limiter.check_rate_limit().await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AblyError::RateLimited { .. }));
    }
}