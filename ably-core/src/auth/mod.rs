// Authentication module for Ably SDK

use serde::{Deserialize, Serialize};

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

/// Token details returned from token request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenDetails {
    pub token: String,
    pub expires: Option<i64>,
    pub issued: Option<i64>,
    pub capability: Option<String>,
    #[serde(rename = "clientId")]
    pub client_id: Option<String>,
}

/// Token request for authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRequest {
    #[serde(rename = "keyName")]
    pub key_name: Option<String>,
    pub ttl: Option<i64>,
    pub capability: Option<String>,
    #[serde(rename = "clientId")]
    pub client_id: Option<String>,
    pub timestamp: Option<i64>,
    pub nonce: Option<String>,
    pub mac: Option<String>,
}