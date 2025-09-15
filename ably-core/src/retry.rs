// 🟡 YELLOW Phase: Minimal retry implementation
// Basic retry policy with exponential backoff

use crate::error::AblyError;
use std::time::Duration;
use tokio::time::sleep;

pub struct RetryPolicy {
    max_attempts: u32,
    initial_delay: Duration,
    max_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
        }
    }
}

impl RetryPolicy {
    pub async fn execute_async<F, T, Fut>(&self, mut operation: F) -> Result<T, AblyError>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, AblyError>>,
    {
        let mut attempt = 1;
        
        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(err) if attempt >= self.max_attempts => return Err(err),
                Err(err) if !err.is_retryable() => return Err(err),
                Err(_) => {
                    let delay = self.calculate_delay(attempt);
                    sleep(delay).await;
                    attempt += 1;
                }
            }
        }
    }
    
    fn calculate_delay(&self, attempt: u32) -> Duration {
        let exponential = self.initial_delay * 2u32.pow(attempt - 1);
        exponential.min(self.max_delay)
    }
}