// Authentication module for Ably SDK

/// Authentication modes supported by Ably
#[derive(Debug, Clone)]
pub enum AuthMode {
    /// API key authentication
    ApiKey(String),
    /// Token authentication
    Token(String),
}

impl AuthMode {
    /// Create API key authentication from key string
    pub fn api_key(key: impl Into<String>) -> Self {
        Self::ApiKey(key.into())
    }

    /// Create token authentication
    pub fn token(token: impl Into<String>) -> Self {
        Self::Token(token.into())
    }
}