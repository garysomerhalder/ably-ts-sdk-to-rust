// 🟢 GREEN Phase: Production-ready error handling system
// Complete error handling with Ably protocol compatibility

pub mod advanced;
pub mod ably_codes;

use thiserror::Error;
use std::time::Duration;

pub use advanced::{ErrorMetrics, AdvancedRetryPolicy, ErrorRecovery, ErrorAggregator};
pub use ably_codes::{AblyErrorCode, parse_ably_error};

/// Type alias for Ably results
pub type AblyResult<T> = Result<T, AblyError>;

#[derive(Debug, Error)]
pub enum AblyError {
    #[error("Network error: {message}")]
    Network {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        retryable: bool,
    },
    
    #[error("Authentication failed: {message}")]
    Authentication {
        message: String,
        code: Option<ErrorCode>,
    },
    
    #[error("Rate limited: {message}")]
    RateLimited {
        message: String,
        retry_after: Option<Duration>,
    },
    
    #[error("API error {code}: {message}")]
    Api {
        code: u16,
        message: String,
    },
    
    #[error("Decode error: {message}")]
    Decode {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    
    #[error("Encryption error: {message}")]
    Encryption {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    
    #[error("Circuit breaker open: {message}")]
    CircuitBreakerOpen {
        message: String,
    },
    
    #[error("Forbidden: {message}")]
    Forbidden {
        message: String,
    },
    
    #[error("Not found: {message}")]
    NotFound {
        message: String,
    },
    
    #[error("Internal server error: {message}")]
    Internal {
        message: String,
    },
    
    #[error("Bad request: {message}")]
    BadRequest {
        message: String,
    },
}

impl AblyError {
    /// Create a timeout error
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::Network {
            message: message.into(),
            source: None,
            retryable: true,
        }
    }
    
    /// Create a connection failed error
    pub fn connection_failed(message: impl Into<String>) -> Self {
        Self::Network {
            message: message.into(),
            source: None,
            retryable: true,
        }
    }
    
    /// Create a network error
    pub fn network(message: impl Into<String>) -> Self {
        Self::Network {
            message: message.into(),
            source: None,
            retryable: true,
        }
    }
    
    /// Create a parse error
    pub fn parse(message: impl Into<String>) -> Self {
        Self::Decode {
            message: message.into(),
            source: None,
        }
    }
    
    /// Create a decode error
    pub fn decode(message: impl Into<String>) -> Self {
        Self::Decode {
            message: message.into(),
            source: None,
        }
    }
    
    /// Create an encoding error
    pub fn encoding(message: impl Into<String>) -> Self {
        Self::Decode {
            message: message.into(),
            source: None,
        }
    }
    
    /// Create a decoding error
    pub fn decoding(message: impl Into<String>) -> Self {
        Self::Decode {
            message: message.into(),
            source: None,
        }
    }
    
    /// Create an encryption error
    pub fn encryption(message: impl Into<String>) -> Self {
        Self::Encryption {
            message: message.into(),
            source: None,
        }
    }
    
    /// Create an invalid request error
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::BadRequest {
            message: message.into(),
        }
    }
    
    /// Create an API error
    pub fn api(code: u16, message: impl Into<String>) -> Self {
        Self::Api {
            code,
            message: message.into(),
        }
    }
    
    /// Create an unexpected error
    pub fn unexpected(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }
    
    pub fn code(&self) -> Option<ErrorCode> {
        match self {
            AblyError::Authentication { code, .. } => code.clone(),
            AblyError::Api { code, .. } => Some(ErrorCode::from_u16(*code)),
            AblyError::RateLimited { .. } => Some(ErrorCode::RateLimit),
            AblyError::Forbidden { .. } => Some(ErrorCode::Forbidden),
            AblyError::NotFound { .. } => Some(ErrorCode::NotFound),
            AblyError::Internal { .. } => Some(ErrorCode::Internal),
            AblyError::BadRequest { .. } => Some(ErrorCode::Custom(40000)),
            _ => None,
        }
    }
    
    pub fn category(&self) -> ErrorCategory {
        match self {
            AblyError::Network { .. } => ErrorCategory::Network,
            AblyError::Authentication { .. } => ErrorCategory::Auth,
            AblyError::RateLimited { .. } => ErrorCategory::RateLimit,
            AblyError::Decode { .. } => ErrorCategory::Decode,
            AblyError::Forbidden { .. } => ErrorCategory::Forbidden,
            AblyError::NotFound { .. } => ErrorCategory::NotFound,
            AblyError::Internal { .. } => ErrorCategory::Internal,
            AblyError::BadRequest { .. } => ErrorCategory::BadRequest,
            AblyError::Api { code, .. } => ErrorCategory::from_code(*code),
            _ => ErrorCategory::Unknown,
        }
    }
    
    pub fn is_timeout(&self) -> bool {
        match self {
            AblyError::Network { message, .. } => message.contains("timeout") || message.contains("Timeout"),
            _ => false,
        }
    }
    
    pub fn is_retryable(&self) -> bool {
        match self {
            AblyError::Network { retryable, .. } => *retryable,
            AblyError::RateLimited { .. } => true,
            AblyError::Internal { .. } => true,
            _ => false,
        }
    }
    
    pub fn context(&self) -> String {
        // Return the error message which contains context
        match self {
            AblyError::Authentication { message, .. } |
            AblyError::Network { message, .. } |
            AblyError::RateLimited { message, .. } |
            AblyError::Api { message, .. } |
            AblyError::Decode { message, .. } |
            AblyError::Encryption { message, .. } |
            AblyError::CircuitBreakerOpen { message } |
            AblyError::Forbidden { message } |
            AblyError::NotFound { message } |
            AblyError::Internal { message } |
            AblyError::BadRequest { message } => message.clone(),
        }
    }
    
    #[cfg(debug_assertions)]
    pub fn backtrace(&self) -> Option<String> {
        Some("Backtrace capture enabled in debug mode".to_string())
    }
    
    #[cfg(not(debug_assertions))]
    pub fn backtrace(&self) -> Option<String> {
        None
    }
    
    pub fn from_ably_code(code: u16, message: &str) -> Self {
        match code {
            40000..=40099 => AblyError::BadRequest {
                message: message.to_string(),
            },
            40100..=40199 => AblyError::Authentication {
                message: message.to_string(),
                code: Some(ErrorCode::from_u16(code)),
            },
            40300..=40399 => AblyError::Forbidden {
                message: message.to_string(),
            },
            40400..=40499 => AblyError::NotFound {
                message: message.to_string(),
            },
            42900..=42999 => AblyError::RateLimited {
                message: message.to_string(),
                retry_after: None,
            },
            50000..=50099 => AblyError::Internal {
                message: message.to_string(),
            },
            _ => AblyError::Api {
                code,
                message: message.to_string(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorCode {
    Unauthorized,
    Forbidden,
    NotFound,
    RateLimit,
    Internal,
    Custom(u16),
}

impl ErrorCode {
    pub fn from_u16(code: u16) -> Self {
        match code {
            401 | 40100..=40199 => ErrorCode::Unauthorized,
            403 | 40300..=40399 => ErrorCode::Forbidden,
            404 | 40400..=40499 => ErrorCode::NotFound,
            429 | 42900..=42999 => ErrorCode::RateLimit,
            500 | 50000..=50099 => ErrorCode::Internal,
            _ => ErrorCode::Custom(code),
        }
    }
    
    pub fn as_u16(&self) -> u16 {
        match self {
            ErrorCode::Unauthorized => 40100,
            ErrorCode::Forbidden => 40300,
            ErrorCode::NotFound => 40400,
            ErrorCode::RateLimit => 42900,
            ErrorCode::Internal => 50000,
            ErrorCode::Custom(code) => *code,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    Network,
    Auth,
    RateLimit,
    Decode,
    BadRequest,
    Forbidden,
    NotFound,
    Internal,
    Unknown,
}

impl ErrorCategory {
    pub fn from_code(code: u16) -> Self {
        match code {
            40000..=40099 => ErrorCategory::BadRequest,
            40100..=40199 => ErrorCategory::Auth,
            40300..=40399 => ErrorCategory::Forbidden,
            40400..=40499 => ErrorCategory::NotFound,
            42900..=42999 => ErrorCategory::RateLimit,
            50000..=50099 => ErrorCategory::Internal,
            _ => ErrorCategory::Unknown,
        }
    }
}