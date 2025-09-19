//! JWT Token Authentication Implementation
//! YELLOW Phase: Minimal JWT implementation for Ably authentication

use super::{AuthMode, TokenDetails, TokenRequest};
use crate::error::{AblyError, AblyResult};
use base64::Engine;
use hmac::{Hmac, Mac};
use rand::Rng;
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

/// JWT authentication handler
pub struct JwtAuth {
    key_name: String,
    key_secret: String,
}

impl JwtAuth {
    /// Create new JWT auth handler from API key
    pub fn new(api_key: &str) -> Self {
        let parts: Vec<&str> = api_key.split(':').collect();
        let key_parts: Vec<&str> = if parts.len() == 2 {
            parts[0].split('.').collect()
        } else {
            vec!["", ""]
        };

        Self {
            key_name: format!("{}.{}", key_parts[0], key_parts.get(1).unwrap_or(&"")),
            key_secret: parts.get(1).unwrap_or(&"").to_string(),
        }
    }

    /// Create a token request builder
    pub fn create_token_request(&self) -> TokenRequestBuilder {
        TokenRequestBuilder::new(self)
    }

    /// Verify MAC signature of a token request
    pub fn verify_mac(&self, request: &TokenRequest) -> AblyResult<bool> {
        let mac = request.mac.as_ref().ok_or_else(|| {
            AblyError::Authentication {
                message: "Missing MAC in token request".to_string(),
                code: None,
            }
        })?;

        let computed_mac = self.compute_mac(request)?;
        Ok(mac == &computed_mac)
    }

    /// Check if a token is expired
    pub fn is_token_expired(&self, token: &TokenDetails) -> bool {
        if let Some(expires) = token.expires {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;
            expires <= now
        } else {
            false
        }
    }

    /// Compute MAC for token request
    fn compute_mac(&self, request: &TokenRequest) -> AblyResult<String> {
        let signing_text = self.build_signing_text(request);

        let mut mac = HmacSha256::new_from_slice(self.key_secret.as_bytes())
            .map_err(|e| AblyError::Authentication {
                message: format!("Failed to create HMAC: {}", e),
                code: None,
            })?;

        mac.update(signing_text.as_bytes());
        let result = mac.finalize();

        Ok(base64::engine::general_purpose::STANDARD.encode(result.into_bytes()))
    }

    /// Build the text to sign for MAC computation
    fn build_signing_text(&self, request: &TokenRequest) -> String {
        let mut parts = vec![];

        if let Some(ref key_name) = request.key_name {
            parts.push(key_name.clone());
        } else {
            parts.push(self.key_name.clone());
        }

        if let Some(ttl) = request.ttl {
            parts.push(ttl.to_string());
        } else {
            parts.push("".to_string());
        }

        if let Some(ref capability) = request.capability {
            parts.push(capability.clone());
        } else {
            parts.push("".to_string());
        }

        if let Some(ref client_id) = request.client_id {
            parts.push(client_id.clone());
        } else {
            parts.push("".to_string());
        }

        if let Some(timestamp) = request.timestamp {
            parts.push(timestamp.to_string());
        } else {
            parts.push("".to_string());
        }

        if let Some(ref nonce) = request.nonce {
            parts.push(nonce.clone());
        } else {
            parts.push("".to_string());
        }

        parts.join("\n")
    }

    /// Generate a random nonce
    fn generate_nonce() -> String {
        let mut rng = rand::thread_rng();
        let nonce: [u8; 16] = rng.gen();
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(nonce)
    }

    /// Get current timestamp in milliseconds
    fn get_timestamp() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
    }
}

/// Builder for token requests
pub struct TokenRequestBuilder<'a> {
    auth: &'a JwtAuth,
    ttl: Option<i64>,
    capability: Option<String>,
    client_id: Option<String>,
}

impl<'a> TokenRequestBuilder<'a> {
    fn new(auth: &'a JwtAuth) -> Self {
        Self {
            auth,
            ttl: None,
            capability: None,
            client_id: None,
        }
    }

    /// Set TTL (time to live) in seconds
    pub fn with_ttl(mut self, ttl: i64) -> Self {
        self.ttl = Some(ttl * 1000); // Convert to milliseconds
        self
    }

    /// Set capability JSON string
    pub fn with_capability(mut self, capability: &str) -> Self {
        self.capability = Some(capability.to_string());
        self
    }

    /// Set client ID
    pub fn with_client_id(mut self, client_id: &str) -> Self {
        self.client_id = Some(client_id.to_string());
        self
    }

    /// Build the token request
    pub async fn build(self) -> AblyResult<TokenRequest> {
        let timestamp = JwtAuth::get_timestamp();
        let nonce = JwtAuth::generate_nonce();

        let mut request = TokenRequest {
            key_name: Some(self.auth.key_name.clone()),
            ttl: self.ttl,
            capability: self.capability,
            client_id: self.client_id,
            timestamp: Some(timestamp),
            nonce: Some(nonce),
            mac: None,
        };

        // Compute MAC
        let mac = self.auth.compute_mac(&request)?;
        request.mac = Some(mac);

        Ok(request)
    }
}

/// Token renewal handler
pub struct TokenRenewalHandler {
    jwt_auth: JwtAuth,
    current_token: Option<TokenDetails>,
    renewal_threshold_ms: i64,
}

impl TokenRenewalHandler {
    /// Create new token renewal handler
    pub fn new(api_key: &str) -> Self {
        Self {
            jwt_auth: JwtAuth::new(api_key),
            current_token: None,
            renewal_threshold_ms: 60_000, // Renew 1 minute before expiry
        }
    }

    /// Set the renewal threshold (milliseconds before expiry)
    pub fn set_renewal_threshold(&mut self, threshold_ms: i64) {
        self.renewal_threshold_ms = threshold_ms;
    }

    /// Check if token needs renewal
    pub fn needs_renewal(&self) -> bool {
        if let Some(ref token) = self.current_token {
            if let Some(expires) = token.expires {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64;

                return expires - now <= self.renewal_threshold_ms;
            }
        }
        true // Need token if we don't have one
    }

    /// Set current token
    pub fn set_token(&mut self, token: TokenDetails) {
        self.current_token = Some(token);
    }

    /// Get current token if valid
    pub fn get_valid_token(&self) -> Option<&TokenDetails> {
        if !self.needs_renewal() {
            self.current_token.as_ref()
        } else {
            None
        }
    }

    /// Create token request for renewal
    pub async fn create_renewal_request(&self) -> AblyResult<TokenRequest> {
        self.jwt_auth
            .create_token_request()
            .with_ttl(3600) // 1 hour default
            .build()
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_auth_initialization() {
        let jwt_auth = JwtAuth::new("app.key:secret");
        assert_eq!(jwt_auth.key_name, "app.key");
        assert_eq!(jwt_auth.key_secret, "secret");
    }

    #[tokio::test]
    async fn test_token_request_builder() {
        let jwt_auth = JwtAuth::new("app.key:secret");
        let request = jwt_auth
            .create_token_request()
            .with_ttl(3600)
            .with_client_id("test-client")
            .build()
            .await
            .unwrap();

        assert_eq!(request.key_name.as_deref(), Some("app.key"));
        assert_eq!(request.ttl, Some(3600000));
        assert_eq!(request.client_id.as_deref(), Some("test-client"));
        assert!(request.mac.is_some());
    }

    #[test]
    fn test_token_expiry_check() {
        let jwt_auth = JwtAuth::new("app.key:secret");

        let expired_token = TokenDetails {
            token: "test".to_string(),
            expires: Some(1000), // Way in the past
            issued: Some(500),
            capability: None,
            client_id: None,
        };

        assert!(jwt_auth.is_token_expired(&expired_token));

        let valid_token = TokenDetails {
            token: "test".to_string(),
            expires: Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64
                    + 3600000,
            ),
            issued: Some(0),
            capability: None,
            client_id: None,
        };

        assert!(!jwt_auth.is_token_expired(&valid_token));
    }

    #[test]
    fn test_renewal_handler() {
        let mut handler = TokenRenewalHandler::new("app.key:secret");

        // Should need renewal when no token
        assert!(handler.needs_renewal());

        // Set a valid token
        let valid_token = TokenDetails {
            token: "test".to_string(),
            expires: Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64
                    + 7200000, // 2 hours from now
            ),
            issued: Some(0),
            capability: None,
            client_id: None,
        };

        handler.set_token(valid_token);
        assert!(!handler.needs_renewal());
        assert!(handler.get_valid_token().is_some());
    }
}