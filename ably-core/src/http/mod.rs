// 🟡 YELLOW Phase: Minimal HTTP client implementation for Ably REST API
// Integration-First - real API calls only!

use crate::auth::AuthMode;
use crate::error::{AblyError, AblyResult};
use reqwest::{Client, RequestBuilder, Response};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Duration;

pub use self::config::HttpConfig;
pub use self::resilience::{CircuitBreaker, RateLimiter, ConnectionMetrics};

mod config;
mod resilience;

/// HTTP methods supported by Ably REST API
#[derive(Debug, Clone, Copy)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

/// Ably HTTP client for REST API operations
pub struct AblyHttpClient {
    client: Client,
    auth_mode: Option<AuthMode>,
    base_url: String,
    default_headers: Vec<(String, String)>,
}

impl AblyHttpClient {
    /// Create new HTTP client with default configuration
    pub fn new(config: HttpConfig) -> Self {
        let client = Client::builder()
            .timeout(config.timeout)
            .connect_timeout(config.connect_timeout)
            .pool_idle_timeout(config.pool_idle_timeout)
            .pool_max_idle_per_host(config.pool_max_idle_per_host)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            auth_mode: None,
            base_url: config.base_url.clone(),
            default_headers: Vec::new(),
        }
    }

    /// Create new HTTP client with authentication
    pub fn with_auth(config: HttpConfig, auth_mode: AuthMode) -> Self {
        let mut client = Self::new(config);
        client.auth_mode = Some(auth_mode);
        client
    }
    
    /// Get the authentication mode
    pub fn auth_mode(&self) -> Option<&AuthMode> {
        self.auth_mode.as_ref()
    }

    /// Add a default header that will be included in all requests
    pub fn add_default_header(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.default_headers.push((key.into(), value.into()));
    }

    /// Create a GET request builder
    pub fn get(&self, url: &str) -> HttpRequestBuilder {
        let full_url = if url.starts_with("http") {
            url.to_string()
        } else {
            format!("{}{}", self.base_url, url)
        };
        HttpRequestBuilder::new(self, HttpMethod::Get, &full_url)
    }

    /// Create a POST request builder
    pub fn post(&self, url: &str) -> HttpRequestBuilder {
        let full_url = if url.starts_with("http") {
            url.to_string()
        } else {
            format!("{}{}", self.base_url, url)
        };
        HttpRequestBuilder::new(self, HttpMethod::Post, &full_url)
    }

    /// Create a PUT request builder
    pub fn put(&self, url: &str) -> HttpRequestBuilder {
        let full_url = if url.starts_with("http") {
            url.to_string()
        } else {
            format!("{}{}", self.base_url, url)
        };
        HttpRequestBuilder::new(self, HttpMethod::Put, &full_url)
    }

    /// Create a DELETE request builder
    pub fn delete(&self, url: &str) -> HttpRequestBuilder {
        let full_url = if url.starts_with("http") {
            url.to_string()
        } else {
            format!("{}{}", self.base_url, url)
        };
        HttpRequestBuilder::new(self, HttpMethod::Delete, &full_url)
    }

    /// Create a PATCH request builder
    pub fn patch(&self, url: &str) -> HttpRequestBuilder {
        let full_url = if url.starts_with("http") {
            url.to_string()
        } else {
            format!("{}{}", self.base_url, url)
        };
        HttpRequestBuilder::new(self, HttpMethod::Patch, &full_url)
    }

    /// Apply authentication to request
    fn apply_auth(&self, request: RequestBuilder) -> RequestBuilder {
        match &self.auth_mode {
            Some(AuthMode::ApiKey(key)) => {
                // Ably uses Basic auth with API key
                use base64::Engine;
                let encoded = base64::engine::general_purpose::STANDARD.encode(key);
                request.header("Authorization", format!("Basic {}", encoded))
            }
            Some(AuthMode::Token(token)) => {
                request.header("Authorization", format!("Bearer {}", token))
            }
            None => request,
        }
    }
}

/// HTTP request builder for fluent API
pub struct HttpRequestBuilder<'a> {
    client: &'a AblyHttpClient,
    method: HttpMethod,
    url: String,
    headers: Vec<(String, String)>,
    query_params: Vec<(String, String)>,
    body: Option<Vec<u8>>,
}

impl<'a> HttpRequestBuilder<'a> {
    fn new(client: &'a AblyHttpClient, method: HttpMethod, url: &str) -> Self {
        let headers = client.default_headers.clone();
        Self {
            client,
            method,
            url: url.to_string(),
            headers,
            query_params: Vec::new(),
            body: None,
        }
    }

    /// Add a header to the request
    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.push((key.to_string(), value.to_string()));
        self
    }

    /// Add query parameters
    pub fn query<T: Serialize + ?Sized>(mut self, params: &T) -> Self {
        if let Ok(serialized) = serde_urlencoded::to_string(params) {
            for pair in serialized.split('&') {
                if let Some((key, value)) = pair.split_once('=') {
                    self.query_params.push((key.to_string(), value.to_string()));
                }
            }
        }
        self
    }

    /// Set JSON body
    pub fn json<T: Serialize>(mut self, body: &T) -> Self {
        if let Ok(json) = serde_json::to_vec(body) {
            self.body = Some(json);
            self.headers.push(("Content-Type".to_string(), "application/json".to_string()));
        }
        self
    }

    /// Send the request and parse response as JSON
    pub async fn send_json<T: DeserializeOwned>(self) -> AblyResult<T> {
        let mut request = match self.method {
            HttpMethod::Get => self.client.client.get(&self.url),
            HttpMethod::Post => self.client.client.post(&self.url),
            HttpMethod::Put => self.client.client.put(&self.url),
            HttpMethod::Delete => self.client.client.delete(&self.url),
            HttpMethod::Patch => self.client.client.patch(&self.url),
        };

        // Apply authentication
        request = self.client.apply_auth(request);

        // Add headers
        for (key, value) in self.headers {
            request = request.header(&key, &value);
        }

        // Add query parameters
        if !self.query_params.is_empty() {
            request = request.query(&self.query_params);
        }

        // Add body if present
        if let Some(body) = self.body {
            request = request.body(body);
        }

        // Send request
        let response = request.send().await.map_err(|e| {
            if e.is_timeout() {
                AblyError::timeout(format!("Request timeout: {}", e))
            } else if e.is_connect() {
                AblyError::connection_failed(format!("Connection failed: {}", e))
            } else {
                AblyError::network(format!("Network error: {}", e))
            }
        })?;

        // Parse response
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AblyError::api(status.as_u16(), error_text));
        }

        response.json::<T>().await.map_err(|e| {
            AblyError::decode(format!("Failed to parse response: {}", e))
        })
    }

    /// Send the request and get raw response
    pub async fn send(self) -> AblyResult<HttpResponse> {
        let mut request = match self.method {
            HttpMethod::Get => self.client.client.get(&self.url),
            HttpMethod::Post => self.client.client.post(&self.url),
            HttpMethod::Put => self.client.client.put(&self.url),
            HttpMethod::Delete => self.client.client.delete(&self.url),
            HttpMethod::Patch => self.client.client.patch(&self.url),
        };

        // Apply authentication
        request = self.client.apply_auth(request);

        // Add headers
        for (key, value) in self.headers {
            request = request.header(&key, &value);
        }

        // Add query parameters
        if !self.query_params.is_empty() {
            request = request.query(&self.query_params);
        }

        // Add body if present
        if let Some(body) = self.body {
            request = request.body(body);
        }

        // Send request
        let response = request.send().await.map_err(|e| {
            if e.is_timeout() {
                AblyError::timeout(format!("Request timeout: {}", e))
            } else if e.is_connect() {
                AblyError::connection_failed(format!("Connection failed: {}", e))
            } else {
                AblyError::network(format!("Network error: {}", e))
            }
        })?;

        Ok(HttpResponse { inner: response })
    }
}

/// HTTP response wrapper
#[derive(Debug)]
pub struct HttpResponse {
    inner: Response,
}

impl HttpResponse {
    /// Get response status code  
    pub fn status(&self) -> reqwest::StatusCode {
        self.inner.status()
    }

    /// Get response headers
    pub fn headers(&self) -> &reqwest::header::HeaderMap {
        self.inner.headers()
    }

    /// Parse response as JSON
    pub async fn json<T: DeserializeOwned>(self) -> AblyResult<T> {
        self.inner
            .json()
            .await
            .map_err(|e| AblyError::parse(format!("Failed to parse JSON: {}", e)))
    }

    /// Get response as text
    pub async fn text(self) -> AblyResult<String> {
        self.inner
            .text()
            .await
            .map_err(|e| AblyError::network(format!("Failed to read response: {}", e)))
    }

    /// Get response as bytes
    pub async fn bytes(self) -> AblyResult<Vec<u8>> {
        self.inner
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| AblyError::network(format!("Failed to read response: {}", e)))
    }
}