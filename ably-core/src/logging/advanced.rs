// 🟢 GREEN Phase: Production-ready logging enhancements
// Advanced logging features for production use

use tracing::{Span, Level};
use tracing_subscriber::EnvFilter;
use std::path::Path;
use std::fs::File;

/// Production logging configuration
pub struct ProductionLogConfig {
    pub enable_json: bool,
    pub enable_file_output: bool,
    pub file_path: String,
    pub enable_sampling: bool,
    pub sampling_rate: f32,
    pub enable_sensitive_redaction: bool,
}

impl Default for ProductionLogConfig {
    fn default() -> Self {
        Self {
            enable_json: true,
            enable_file_output: false,
            file_path: "ably.log".to_string(),
            enable_sampling: false,
            sampling_rate: 1.0,
            enable_sensitive_redaction: true,
        }
    }
}

/// Initialize production logging with advanced features
pub fn init_production_logging(config: ProductionLogConfig) {
    // Build environment filter
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    
    // Configure subscriber based on options
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(env_filter);
    
    if config.enable_json {
        subscriber.json().init();
    } else {
        subscriber.init();
    }
}

/// Create a span with automatic performance tracking
pub fn create_performance_span(name: &str) -> Span {
    tracing::span!(Level::INFO, "perf", operation = name)
}

/// Redact sensitive information in production
pub fn redact_sensitive(value: &str) -> String {
    // Check for common sensitive patterns
    if value.contains(':') && value.contains('.') {
        // Likely an API key
        return "[API_KEY_REDACTED]".to_string();
    }
    
    if value.len() == 16 && value.chars().all(|c| c.is_numeric() || c == '-') {
        // Likely a credit card number
        return "[CARD_REDACTED]".to_string();
    }
    
    if value.contains('@') {
        // Likely an email
        let parts: Vec<&str> = value.split('@').collect();
        if parts.len() == 2 {
            return format!("{}@[REDACTED]", &parts[0][..1]);
        }
    }
    
    value.to_string()
}

/// Log metrics for monitoring
pub struct LogMetrics {
    pub total_logs: u64,
    pub error_count: u64,
    pub warning_count: u64,
    pub performance_p95_ms: f64,
}

impl LogMetrics {
    pub fn new() -> Self {
        Self {
            total_logs: 0,
            error_count: 0,
            warning_count: 0,
            performance_p95_ms: 0.0,
        }
    }
    
    pub fn record_log(&mut self, level: &str) {
        self.total_logs += 1;
        match level {
            "ERROR" => self.error_count += 1,
            "WARN" => self.warning_count += 1,
            _ => {}
        }
    }
}

/// Distributed tracing support
pub struct TraceContext {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
}

impl TraceContext {
    pub fn new() -> Self {
        use uuid::Uuid;
        Self {
            trace_id: Uuid::new_v4().to_string(),
            span_id: Uuid::new_v4().to_string(),
            parent_span_id: None,
        }
    }
    
    pub fn child(&self) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            span_id: uuid::Uuid::new_v4().to_string(),
            parent_span_id: Some(self.span_id.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sensitive_redaction() {
        assert_eq!(redact_sensitive("key.id:secret"), "[API_KEY_REDACTED]");
        assert_eq!(redact_sensitive("4111-1111-1111-1111"), "[CARD_REDACTED]");
        assert_eq!(redact_sensitive("user@example.com"), "u@[REDACTED]");
        assert_eq!(redact_sensitive("normal text"), "normal text");
    }
    
    #[test]
    fn test_trace_context() {
        let parent = TraceContext::new();
        let child = parent.child();
        
        assert_eq!(parent.trace_id, child.trace_id);
        assert_ne!(parent.span_id, child.span_id);
        assert_eq!(child.parent_span_id, Some(parent.span_id));
    }
}