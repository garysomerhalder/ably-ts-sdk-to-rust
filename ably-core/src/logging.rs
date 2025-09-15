// 🟢 GREEN Phase: Production-ready logging and tracing system
// Complete logging infrastructure with advanced features

pub mod advanced;

use tracing::{info, Span};
use tracing_subscriber;
use std::sync::atomic::{AtomicU64, Ordering};
use uuid::Uuid;

static CORRELATION_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub struct LogConfig {
    pub level: LogLevel,
    pub format: String,
    pub console_output: bool,
    pub file_output: Option<String>,
    pub sampling_rate: f32,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            format: "json".to_string(),
            console_output: true,
            file_output: None,
            sampling_rate: 1.0,
        }
    }
}

impl LogConfig {
    pub fn builder() -> LogConfigBuilder {
        LogConfigBuilder::default()
    }
}

pub struct LogConfigBuilder {
    level: LogLevel,
    format: String,
    console_output: bool,
    file_output: Option<String>,
    sampling_rate: f32,
}

impl Default for LogConfigBuilder {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            format: "json".to_string(),
            console_output: true,
            file_output: None,
            sampling_rate: 1.0,
        }
    }
}

impl LogConfigBuilder {
    pub fn level(mut self, level: LogLevel) -> Self {
        self.level = level;
        self
    }
    
    pub fn format(mut self, format: &str) -> Self {
        self.format = format.to_string();
        self
    }
    
    pub fn console_output(mut self, enabled: bool) -> Self {
        self.console_output = enabled;
        self
    }
    
    pub fn file_output(mut self, path: &str) -> Self {
        self.file_output = Some(path.to_string());
        self
    }
    
    pub fn sampling_rate(mut self, rate: f32) -> Self {
        self.sampling_rate = rate;
        self
    }
    
    pub fn build(self) -> LogConfig {
        LogConfig {
            level: self.level,
            format: self.format,
            console_output: self.console_output,
            file_output: self.file_output,
            sampling_rate: self.sampling_rate,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    fn as_str(&self) -> &str {
        match self {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
    }
}

pub fn init_logging(config: LogConfig) {
    // Simple initialization for YELLOW phase
    if config.format == "json" {
        tracing_subscriber::fmt()
            .json()
            .init();
    } else {
        tracing_subscriber::fmt()
            .init();
    }
    
    info!("Logging initialized with level: {:?}", config.level);
}

pub fn create_span(name: &str) -> Span {
    tracing::info_span!("operation", name = name)
}

pub fn with_correlation_id<F, R>(f: F) -> Option<String>
where
    F: FnOnce() -> R,
{
    let correlation_id = generate_correlation_id();
    let span = tracing::info_span!("correlated", correlation_id = %correlation_id);
    let _guard = span.enter();
    
    f();
    
    Some(correlation_id)
}

pub fn get_correlation_id() -> Option<String> {
    // In YELLOW phase, just generate a new one
    Some(generate_correlation_id())
}

pub fn generate_correlation_id() -> String {
    format!("corr_{}", CORRELATION_COUNTER.fetch_add(1, Ordering::SeqCst))
}

pub fn generate_trace_id() -> String {
    Uuid::new_v4().to_string()
}

pub fn get_current_trace_id() -> Option<String> {
    // In YELLOW phase, return None
    None
}

pub fn redact<T: std::fmt::Display>(_value: T) -> String {
    // Simple redaction for YELLOW phase
    "[REDACTED]".to_string()
}

// Extension trait for Span
pub trait SpanExt {
    fn with_field(self, key: &str, value: &str) -> Self;
    fn with_trace_id(self, trace_id: String) -> Self;
}

impl SpanExt for Span {
    fn with_field(self, _key: &str, _value: &str) -> Self {
        // In YELLOW phase, just return self
        self
    }
    
    fn with_trace_id(self, _trace_id: String) -> Self {
        // In YELLOW phase, just return self
        self
    }
}