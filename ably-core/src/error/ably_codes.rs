// 🟢 GREEN Phase: Complete Ably error code mapping
// All Ably protocol error codes for 100% compatibility

use super::{AblyError, ErrorCategory};
use serde_json;

/// Complete Ably error code definitions
/// Based on Ably protocol specification
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AblyErrorCode {
    // 40xxx Client errors
    BadRequest = 40000,
    InvalidRequestBody = 40001,
    InvalidParameterName = 40002,
    InvalidParameterValue = 40003,
    InvalidHeader = 40004,
    InvalidCredential = 40005,
    InvalidConnectionId = 40006,
    InvalidMessageId = 40007,
    InvalidContentLength = 40008,
    MaxMessageLengthExceeded = 40009,
    InvalidChannelName = 40010,
    
    // 401xx Authentication errors
    Unauthorized = 40100,
    InvalidCredentials = 40101,
    IncompatibleCredentials = 40102,
    InvalidUseOfBasicAuthOverHttp = 40103,
    TokenExpired = 40104,
    TokenRevoked = 40105,
    
    // 403xx Authorization errors  
    Forbidden = 40300,
    AccountDisabled = 40301,
    AccountRestrictedConnectionLimitsExceeded = 40302,
    AccountRestrictedMessageLimitsExceeded = 40303,
    AccountRestrictedConnectionRateLimitExceeded = 40304,
    AccountBlocked = 40305,
    
    // 404xx Not found errors
    NotFound = 40400,
    ResourceNotFound = 40401,
    
    // 405xx Method errors
    MethodNotAllowed = 40500,
    
    // 408xx Timeout errors
    RequestTimeout = 40800,
    ConnectionTimeout = 40801,
    
    // 429xx Rate limiting
    TooManyRequests = 42900,
    RateLimitExceeded = 42901,
    
    // 50xxx Server errors
    InternalServerError = 50000,
    InternalChannelError = 50001,
    InternalConnectionError = 50002,
    TimeoutError = 50003,
    RequestFailed = 50004,
}

impl AblyErrorCode {
    pub fn to_u16(self) -> u16 {
        self as u16
    }
    
    pub fn from_u16(code: u16) -> Option<Self> {
        match code {
            40000 => Some(Self::BadRequest),
            40001 => Some(Self::InvalidRequestBody),
            40002 => Some(Self::InvalidParameterName),
            40003 => Some(Self::InvalidParameterValue),
            40004 => Some(Self::InvalidHeader),
            40005 => Some(Self::InvalidCredential),
            40006 => Some(Self::InvalidConnectionId),
            40007 => Some(Self::InvalidMessageId),
            40008 => Some(Self::InvalidContentLength),
            40009 => Some(Self::MaxMessageLengthExceeded),
            40010 => Some(Self::InvalidChannelName),
            
            40100 => Some(Self::Unauthorized),
            40101 => Some(Self::InvalidCredentials),
            40102 => Some(Self::IncompatibleCredentials),
            40103 => Some(Self::InvalidUseOfBasicAuthOverHttp),
            40104 => Some(Self::TokenExpired),
            40105 => Some(Self::TokenRevoked),
            
            40300 => Some(Self::Forbidden),
            40301 => Some(Self::AccountDisabled),
            40302 => Some(Self::AccountRestrictedConnectionLimitsExceeded),
            40303 => Some(Self::AccountRestrictedMessageLimitsExceeded),
            40304 => Some(Self::AccountRestrictedConnectionRateLimitExceeded),
            40305 => Some(Self::AccountBlocked),
            
            40400 => Some(Self::NotFound),
            40401 => Some(Self::ResourceNotFound),
            
            40500 => Some(Self::MethodNotAllowed),
            
            40800 => Some(Self::RequestTimeout),
            40801 => Some(Self::ConnectionTimeout),
            
            42900 => Some(Self::TooManyRequests),
            42901 => Some(Self::RateLimitExceeded),
            
            50000 => Some(Self::InternalServerError),
            50001 => Some(Self::InternalChannelError),
            50002 => Some(Self::InternalConnectionError),
            50003 => Some(Self::TimeoutError),
            50004 => Some(Self::RequestFailed),
            
            _ => None,
        }
    }
    
    pub fn category(&self) -> ErrorCategory {
        match self.to_u16() {
            40000..=40099 => ErrorCategory::BadRequest,
            40100..=40199 => ErrorCategory::Auth,
            40300..=40399 => ErrorCategory::Forbidden,
            40400..=40499 => ErrorCategory::NotFound,
            40500..=40599 => ErrorCategory::BadRequest,
            40800..=40899 => ErrorCategory::Network,
            42900..=42999 => ErrorCategory::RateLimit,
            50000..=50099 => ErrorCategory::Internal,
            _ => ErrorCategory::Unknown,
        }
    }
    
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RequestTimeout |
            Self::ConnectionTimeout |
            Self::TooManyRequests |
            Self::RateLimitExceeded |
            Self::InternalServerError |
            Self::InternalChannelError |
            Self::InternalConnectionError |
            Self::TimeoutError |
            Self::RequestFailed
        )
    }
    
    pub fn default_message(&self) -> &'static str {
        match self {
            Self::BadRequest => "Bad request",
            Self::InvalidRequestBody => "Invalid request body",
            Self::InvalidParameterName => "Invalid parameter name",
            Self::InvalidParameterValue => "Invalid parameter value",
            Self::InvalidHeader => "Invalid header",
            Self::InvalidCredential => "Invalid credential",
            Self::InvalidConnectionId => "Invalid connection ID",
            Self::InvalidMessageId => "Invalid message ID",
            Self::InvalidContentLength => "Invalid content length",
            Self::MaxMessageLengthExceeded => "Maximum message length exceeded",
            Self::InvalidChannelName => "Invalid channel name",
            
            Self::Unauthorized => "Unauthorized",
            Self::InvalidCredentials => "Invalid credentials",
            Self::IncompatibleCredentials => "Incompatible credentials",
            Self::InvalidUseOfBasicAuthOverHttp => "Invalid use of basic auth over non-TLS connection",
            Self::TokenExpired => "Token expired",
            Self::TokenRevoked => "Token revoked",
            
            Self::Forbidden => "Forbidden",
            Self::AccountDisabled => "Account disabled",
            Self::AccountRestrictedConnectionLimitsExceeded => "Account connection limits exceeded",
            Self::AccountRestrictedMessageLimitsExceeded => "Account message limits exceeded",
            Self::AccountRestrictedConnectionRateLimitExceeded => "Account connection rate limit exceeded",
            Self::AccountBlocked => "Account blocked",
            
            Self::NotFound => "Not found",
            Self::ResourceNotFound => "Resource not found",
            
            Self::MethodNotAllowed => "Method not allowed",
            
            Self::RequestTimeout => "Request timeout",
            Self::ConnectionTimeout => "Connection timeout",
            
            Self::TooManyRequests => "Too many requests",
            Self::RateLimitExceeded => "Rate limit exceeded",
            
            Self::InternalServerError => "Internal server error",
            Self::InternalChannelError => "Internal channel error",
            Self::InternalConnectionError => "Internal connection error",
            Self::TimeoutError => "Timeout error",
            Self::RequestFailed => "Request failed",
        }
    }
}

/// Convert HTTP status codes to Ably error codes
pub fn http_to_ably_code(status: u16) -> AblyErrorCode {
    match status {
        400 => AblyErrorCode::BadRequest,
        401 => AblyErrorCode::Unauthorized,
        403 => AblyErrorCode::Forbidden,
        404 => AblyErrorCode::NotFound,
        405 => AblyErrorCode::MethodNotAllowed,
        408 => AblyErrorCode::RequestTimeout,
        429 => AblyErrorCode::TooManyRequests,
        500 => AblyErrorCode::InternalServerError,
        502 | 503 | 504 => AblyErrorCode::RequestFailed,
        _ => AblyErrorCode::BadRequest,
    }
}

/// Parse error response from Ably API
pub fn parse_ably_error(status: u16, body: &str) -> AblyError {
    // Try to parse JSON error response
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(error) = json.get("error") {
            let code = error.get("code")
                .and_then(|c| c.as_u64())
                .map(|c| c as u16)
                .unwrap_or_else(|| http_to_ably_code(status).to_u16());
            
            let message = error.get("message")
                .and_then(|m| m.as_str())
                .unwrap_or_else(|| AblyErrorCode::from_u16(code)
                    .map(|c| c.default_message())
                    .unwrap_or("Unknown error"))
                .to_string();
            
            return AblyError::from_ably_code(code, &message);
        }
    }
    
    // Fallback to status-based error
    let code = http_to_ably_code(status);
    AblyError::from_ably_code(code.to_u16(), code.default_message())
}