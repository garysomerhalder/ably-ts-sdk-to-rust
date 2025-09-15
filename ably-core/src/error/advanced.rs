// 🟢 GREEN Phase: Production-ready error handling enhancements
// Advanced error features for production use

use super::{AblyError, ErrorCategory};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{error, warn, info};

/// Error metrics collector for observability
pub struct ErrorMetrics {
    counters: Arc<RwLock<HashMap<ErrorCategory, u64>>>,
    last_errors: Arc<RwLock<Vec<ErrorRecord>>>,
    start_time: Instant,
}

#[derive(Debug, Clone)]
pub struct ErrorRecord {
    pub timestamp: Instant,
    pub category: ErrorCategory,
    pub message: String,
    pub retryable: bool,
}

impl ErrorMetrics {
    pub fn new() -> Self {
        Self {
            counters: Arc::new(RwLock::new(HashMap::new())),
            last_errors: Arc::new(RwLock::new(Vec::with_capacity(100))),
            start_time: Instant::now(),
        }
    }
    
    pub async fn record_error(&self, error: &AblyError) {
        let category = error.category();
        
        // Update counter
        let mut counters = self.counters.write().await;
        *counters.entry(category.clone()).or_insert(0) += 1;
        
        // Record error details
        let mut errors = self.last_errors.write().await;
        errors.push(ErrorRecord {
            timestamp: Instant::now(),
            category,
            message: error.context(),
            retryable: error.is_retryable(),
        });
        
        // Keep only last 100 errors
        if errors.len() > 100 {
            errors.remove(0);
        }
        
        // Log the error
        match error.category() {
            ErrorCategory::Network | ErrorCategory::Internal => {
                error!("Critical error: {:?}", error);
            }
            ErrorCategory::Auth | ErrorCategory::Forbidden => {
                warn!("Auth error: {:?}", error);
            }
            _ => {
                info!("Error occurred: {:?}", error);
            }
        }
    }
    
    pub async fn get_stats(&self) -> ErrorStats {
        let counters = self.counters.read().await;
        let errors = self.last_errors.read().await;
        
        let total_errors: u64 = counters.values().sum();
        let uptime = self.start_time.elapsed();
        let error_rate = if uptime.as_secs() > 0 {
            total_errors as f64 / uptime.as_secs() as f64
        } else {
            0.0
        };
        
        ErrorStats {
            total_errors,
            errors_by_category: counters.clone(),
            error_rate,
            uptime,
            recent_errors: errors.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ErrorStats {
    pub total_errors: u64,
    pub errors_by_category: HashMap<ErrorCategory, u64>,
    pub error_rate: f64,
    pub uptime: Duration,
    pub recent_errors: Vec<ErrorRecord>,
}

/// Advanced retry policy with jitter and circuit breaking
pub struct AdvancedRetryPolicy {
    max_attempts: u32,
    initial_delay: Duration,
    max_delay: Duration,
    jitter_factor: f32,
    circuit_breaker_threshold: u32,
    failure_count: Arc<RwLock<u32>>,
    last_failure: Arc<RwLock<Option<Instant>>>,
}

impl AdvancedRetryPolicy {
    pub fn new() -> Self {
        Self {
            max_attempts: 5,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            jitter_factor: 0.25,
            circuit_breaker_threshold: 10,
            failure_count: Arc::new(RwLock::new(0)),
            last_failure: Arc::new(RwLock::new(None)),
        }
    }
    
    pub async fn should_retry(&self, attempt: u32, error: &AblyError) -> bool {
        // Check if error is retryable
        if !error.is_retryable() {
            return false;
        }
        
        // Check max attempts
        if attempt >= self.max_attempts {
            return false;
        }
        
        // Check circuit breaker
        let failures = self.failure_count.read().await;
        if *failures >= self.circuit_breaker_threshold {
            let last_failure = self.last_failure.read().await;
            if let Some(last) = *last_failure {
                // Allow retry after cooldown period
                if last.elapsed() < Duration::from_secs(60) {
                    warn!("Circuit breaker open, denying retry");
                    return false;
                }
            }
        }
        
        true
    }
    
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let exponential = self.initial_delay.as_millis() as u64 * 2u64.pow(attempt - 1);
        let base_delay = Duration::from_millis(exponential.min(self.max_delay.as_millis() as u64));
        
        // Add jitter
        let jitter_ms = (base_delay.as_millis() as f32 * self.jitter_factor * rand::random::<f32>()) as u64;
        base_delay + Duration::from_millis(jitter_ms)
    }
    
    pub async fn record_success(&self) {
        let mut failures = self.failure_count.write().await;
        *failures = 0;
        
        let mut last = self.last_failure.write().await;
        *last = None;
    }
    
    pub async fn record_failure(&self) {
        let mut failures = self.failure_count.write().await;
        *failures += 1;
        
        let mut last = self.last_failure.write().await;
        *last = Some(Instant::now());
    }
}

/// Error recovery strategies
#[derive(Debug, Clone)]
pub enum RecoveryStrategy {
    /// Retry with exponential backoff
    Retry(Duration),
    /// Fallback to alternative service
    Fallback(String),
    /// Graceful degradation
    Degrade,
    /// Fail fast
    FailFast,
    /// Queue for later processing
    Queue,
}

pub struct ErrorRecovery {
    strategies: HashMap<ErrorCategory, RecoveryStrategy>,
}

impl ErrorRecovery {
    pub fn new() -> Self {
        let mut strategies = HashMap::new();
        
        // Default strategies per error category
        strategies.insert(ErrorCategory::Network, RecoveryStrategy::Retry(Duration::from_secs(1)));
        strategies.insert(ErrorCategory::RateLimit, RecoveryStrategy::Retry(Duration::from_secs(30)));
        strategies.insert(ErrorCategory::Auth, RecoveryStrategy::FailFast);
        strategies.insert(ErrorCategory::Internal, RecoveryStrategy::Retry(Duration::from_secs(5)));
        strategies.insert(ErrorCategory::BadRequest, RecoveryStrategy::FailFast);
        
        Self { strategies }
    }
    
    pub fn get_strategy(&self, error: &AblyError) -> RecoveryStrategy {
        self.strategies
            .get(&error.category())
            .cloned()
            .unwrap_or(RecoveryStrategy::FailFast)
    }
}

/// Error aggregator for batch operations
pub struct ErrorAggregator {
    errors: Vec<AblyError>,
    max_errors: usize,
}

impl ErrorAggregator {
    pub fn new(max_errors: usize) -> Self {
        Self {
            errors: Vec::new(),
            max_errors,
        }
    }
    
    pub fn add_error(&mut self, error: AblyError) -> Result<(), AblyError> {
        self.errors.push(error);
        
        if self.errors.len() >= self.max_errors {
            Err(AblyError::Api {
                code: 50001,
                message: format!("Too many errors: {} errors accumulated", self.errors.len()),
            })
        } else {
            Ok(())
        }
    }
    
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
    
    pub fn get_summary(&self) -> Option<String> {
        if self.errors.is_empty() {
            return None;
        }
        
        let mut summary = format!("{} errors occurred:\n", self.errors.len());
        for (i, error) in self.errors.iter().enumerate().take(5) {
            summary.push_str(&format!("  {}. {}\n", i + 1, error.context()));
        }
        
        if self.errors.len() > 5 {
            summary.push_str(&format!("  ... and {} more errors\n", self.errors.len() - 5));
        }
        
        Some(summary)
    }
}

// Re-export rand for jitter
use rand;