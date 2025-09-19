// 🟢 GREEN Phase: Production-ready retry logic with exponential backoff

use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn, instrument};
use rand::Rng;

/// Retry policy with exponential backoff and jitter
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial backoff duration
    pub initial_backoff: Duration,
    /// Maximum backoff duration
    pub max_backoff: Duration,
    /// Backoff multiplier (typically 2.0)
    pub backoff_multiplier: f64,
    /// Whether to add jitter to backoff
    pub use_jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            use_jitter: true,
        }
    }
}

impl RetryPolicy {
    /// Create a new retry policy with custom settings
    pub fn new(max_retries: u32, initial_backoff: Duration) -> Self {
        Self {
            max_retries,
            initial_backoff,
            ..Default::default()
        }
    }

    /// Calculate backoff duration for a given attempt
    pub fn calculate_backoff(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::ZERO;
        }

        let mut backoff = self.initial_backoff.as_millis() as f64;

        // Apply exponential backoff
        for _ in 1..attempt {
            backoff *= self.backoff_multiplier;
        }

        // Cap at maximum backoff
        backoff = backoff.min(self.max_backoff.as_millis() as f64);

        // Add jitter if enabled (0-25% random variation)
        if self.use_jitter {
            let mut rng = rand::thread_rng();
            let jitter = rng.gen_range(0.0..0.25);
            backoff *= 1.0 + jitter;
        }

        Duration::from_millis(backoff as u64)
    }

    /// Execute a future with retry logic
    #[instrument(skip(self, f, should_retry))]
    pub async fn execute_with_retry<F, Fut, T, E>(
        &self,
        f: F,
        should_retry: impl Fn(&E) -> bool,
    ) -> Result<T, E>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: std::fmt::Debug,
    {
        let mut attempt = 0;

        loop {
            match f().await {
                Ok(result) => {
                    if attempt > 0 {
                        debug!("Request succeeded after {} retries", attempt);
                    }
                    return Ok(result);
                }
                Err(err) => {
                    if attempt >= self.max_retries || !should_retry(&err) {
                        warn!("Request failed after {} attempts: {:?}", attempt + 1, err);
                        return Err(err);
                    }

                    let backoff = self.calculate_backoff(attempt + 1);
                    warn!(
                        "Request failed (attempt {}), retrying in {:?}: {:?}",
                        attempt + 1,
                        backoff,
                        err
                    );

                    sleep(backoff).await;
                    attempt += 1;
                }
            }
        }
    }
}

/// Builder for retry policy
pub struct RetryPolicyBuilder {
    policy: RetryPolicy,
}

impl RetryPolicyBuilder {
    pub fn new() -> Self {
        Self {
            policy: RetryPolicy::default(),
        }
    }

    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.policy.max_retries = max_retries;
        self
    }

    pub fn initial_backoff(mut self, duration: Duration) -> Self {
        self.policy.initial_backoff = duration;
        self
    }

    pub fn max_backoff(mut self, duration: Duration) -> Self {
        self.policy.max_backoff = duration;
        self
    }

    pub fn backoff_multiplier(mut self, multiplier: f64) -> Self {
        self.policy.backoff_multiplier = multiplier;
        self
    }

    pub fn use_jitter(mut self, use_jitter: bool) -> Self {
        self.policy.use_jitter = use_jitter;
        self
    }

    pub fn build(self) -> RetryPolicy {
        self.policy
    }
}

/// Determine if an error is retryable
pub trait RetryableError {
    fn is_retryable(&self) -> bool;
}

impl RetryableError for reqwest::Error {
    fn is_retryable(&self) -> bool {
        // Network errors, timeouts, and connection errors are retryable
        self.is_timeout() || self.is_connect() || self.is_request()
    }
}

impl RetryableError for crate::http::HttpError {
    fn is_retryable(&self) -> bool {
        use crate::http::HttpError;
        match self {
            HttpError::Timeout(_) => true,
            HttpError::Network(_) => true,
            HttpError::ServerError { status, .. } => {
                // Retry on 502, 503, 504 (server errors that might be temporary)
                matches!(*status, 502 | 503 | 504)
            }
            HttpError::RateLimited { .. } => false, // Don't retry rate limits immediately
            HttpError::AuthenticationFailed { .. } => false, // Don't retry auth failures
            HttpError::NotFound { .. } => false, // Don't retry 404s
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exponential_backoff_calculation() {
        let policy = RetryPolicy {
            max_retries: 5,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            use_jitter: false, // Disable jitter for predictable testing
        };

        assert_eq!(policy.calculate_backoff(0), Duration::ZERO);
        assert_eq!(policy.calculate_backoff(1), Duration::from_millis(100));
        assert_eq!(policy.calculate_backoff(2), Duration::from_millis(200));
        assert_eq!(policy.calculate_backoff(3), Duration::from_millis(400));
        assert_eq!(policy.calculate_backoff(4), Duration::from_millis(800));
        assert_eq!(policy.calculate_backoff(5), Duration::from_millis(1600));
    }

    #[test]
    fn test_max_backoff_cap() {
        let policy = RetryPolicy {
            max_retries: 10,
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(5),
            backoff_multiplier: 2.0,
            use_jitter: false,
        };

        // Should cap at 5 seconds
        assert_eq!(policy.calculate_backoff(10), Duration::from_secs(5));
    }

    #[tokio::test]
    async fn test_retry_execution() {
        let policy = RetryPolicy::new(3, Duration::from_millis(10));
        let mut attempt_count = 0;

        let result = policy
            .execute_with_retry(
                || {
                    attempt_count += 1;
                    async move {
                        if attempt_count < 3 {
                            Err("Temporary error")
                        } else {
                            Ok("Success")
                        }
                    }
                },
                |_| true, // Always retry
            )
            .await;

        assert_eq!(result, Ok("Success"));
        assert_eq!(attempt_count, 3);
    }

    #[tokio::test]
    async fn test_no_retry_on_non_retryable_error() {
        let policy = RetryPolicy::new(3, Duration::from_millis(10));
        let mut attempt_count = 0;

        let result = policy
            .execute_with_retry(
                || {
                    attempt_count += 1;
                    async move { Err("Non-retryable error") }
                },
                |_| false, // Never retry
            )
            .await;

        assert_eq!(result, Err("Non-retryable error"));
        assert_eq!(attempt_count, 1); // Should not retry
    }
}