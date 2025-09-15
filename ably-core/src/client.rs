// 🟡 YELLOW Phase: Minimal client implementation
// Basic REST client with error handling

use crate::error::{AblyError, ErrorCode};
use reqwest;
use serde::de::DeserializeOwned;
use serde_json;
use std::time::Duration;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

pub struct RestClient {
    api_key: String,
    http_client: reqwest::Client,
    base_url: String,
    failure_count: Arc<AtomicU32>,
}

impl RestClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            http_client: reqwest::Client::new(),
            base_url: "https://sandbox-rest.ably.io".to_string(),
            failure_count: Arc::new(AtomicU32::new(0)),
        }
    }
    
    pub fn builder() -> RestClientBuilder {
        RestClientBuilder::default()
    }
    
    pub async fn get_server_time(&self) -> Result<i64, AblyError> {
        // Check circuit breaker
        if self.failure_count.load(Ordering::SeqCst) >= 5 {
            return Err(AblyError::CircuitBreakerOpen {
                message: "Too many consecutive failures".to_string(),
            });
        }
        
        let url = format!("{}/time", self.base_url);
        let response = self.http_client
            .get(&url)
            .header("Authorization", format!("Basic {}", base64_encode(&self.api_key)))
            .send()
            .await
            .map_err(|e| AblyError::Network {
                message: e.to_string(),
                source: Some(Box::new(e)),
                retryable: true,
            })?;
        
        if response.status() == 401 {
            self.failure_count.fetch_add(1, Ordering::SeqCst);
            return Err(AblyError::Authentication {
                message: "Authentication failed".to_string(),
                code: Some(ErrorCode::Unauthorized),
            });
        }
        
        if response.status() == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .map(Duration::from_secs);
                
            return Err(AblyError::RateLimited {
                message: "Rate limit exceeded".to_string(),
                retry_after,
            });
        }
        
        if !response.status().is_success() {
            return Err(AblyError::Api {
                code: response.status().as_u16(),
                message: format!("API error: {}", response.status()),
            });
        }
        
        let times: Vec<i64> = response.json().await
            .map_err(|e| AblyError::Decode {
                message: e.to_string(),
                source: Some(Box::new(e)),
            })?;
        
        times.first().copied()
            .ok_or_else(|| AblyError::Decode {
                message: "Empty time response".to_string(),
                source: None,
            })
    }
    
    pub async fn publish_message(&self, channel: &str, message: &str) -> Result<(), AblyError> {
        let url = format!("{}/channels/{}/messages", self.base_url, channel);
        let response = self.http_client
            .post(&url)
            .header("Authorization", format!("Basic {}", base64_encode(&self.api_key)))
            .json(&serde_json::json!({
                "data": message
            }))
            .send()
            .await
            .map_err(|e| AblyError::Network {
                message: format!("Failed to publish to channel: {}", channel),
                source: Some(Box::new(e)),
                retryable: true,
            })?;
        
        if response.status() == 401 {
            return Err(AblyError::Authentication {
                message: format!("Authentication failed for channel: {}", channel),
                code: Some(ErrorCode::Unauthorized),
            });
        }
        
        if !response.status().is_success() {
            return Err(AblyError::Api {
                code: response.status().as_u16(),
                message: format!("Failed to publish to channel: {}", channel),
            });
        }
        
        Ok(())
    }
    
    pub async fn request_raw(&self, path: &str, _method: &str) -> Result<String, AblyError> {
        // Minimal implementation for testing
        let url = format!("{}{}", self.base_url, path);
        let response = self.http_client
            .get(&url)
            .header("Authorization", format!("Basic {}", base64_encode(&self.api_key)))
            .send()
            .await
            .map_err(|e| AblyError::Network {
                message: e.to_string(),
                source: Some(Box::new(e)),
                retryable: true,
            })?;
        
        response.text().await
            .map_err(|e| AblyError::Decode {
                message: e.to_string(),
                source: Some(Box::new(e)),
            })
    }
    
    pub fn parse_as<T: DeserializeOwned>(&self, body: &str) -> Result<T, AblyError> {
        serde_json::from_str(body)
            .map_err(|e| AblyError::Decode {
                message: e.to_string(),
                source: Some(Box::new(e)),
            })
    }
}

pub struct RestClientBuilder {
    api_key: Option<String>,
    timeout_ms: Option<u64>,
}

impl Default for RestClientBuilder {
    fn default() -> Self {
        Self {
            api_key: None,
            timeout_ms: None,
        }
    }
}

impl RestClientBuilder {
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }
    
    pub fn timeout_ms(mut self, timeout: u64) -> Self {
        self.timeout_ms = Some(timeout);
        self
    }
    
    pub fn build(self) -> RestClient {
        let mut client_builder = reqwest::Client::builder();
        
        if let Some(timeout) = self.timeout_ms {
            client_builder = client_builder.timeout(Duration::from_millis(timeout));
        }
        
        RestClient {
            api_key: self.api_key.unwrap_or_default(),
            http_client: client_builder.build().unwrap(),
            base_url: "https://sandbox-rest.ably.io".to_string(),
            failure_count: Arc::new(AtomicU32::new(0)),
        }
    }
}

fn base64_encode(input: &str) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(input)
}