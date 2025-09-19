// Enhanced HTTP client methods with full production resilience
// This will be integrated into the main mod.rs file

use super::*;

impl AblyHttpClient {
    /// Execute GET request with resilience features
    async fn execute_get_request<T: DeserializeOwned>(
        &self,
        path: &str,
        request_id: &Uuid,
    ) -> Result<T, HttpError> {
        let full_url = if path.starts_with("http") {
            path.to_string()
        } else {
            format!("{}{}", self.base_url, path)
        };

        let mut request = self.client.get(&full_url);
        request = self.apply_auth(request);
        request = request
            .header("User-Agent", "ably-rust-sdk/0.1.0")
            .header("X-Request-ID", request_id.to_string())
            .header("X-Ably-Version", "3")
            .header("X-Ably-Lib", format!("rust-{}", env!("CARGO_PKG_VERSION")));

        let response = request.send().await?;
        let status = response.status();

        if status == 401 {
            return Err(HttpError::AuthenticationFailed {
                status: status.as_u16(),
            });
        } else if status == 404 {
            return Err(HttpError::NotFound {
                message: response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Not found".to_string()),
            });
        } else if status == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok());
            return Err(HttpError::RateLimited { retry_after });
        } else if !status.is_success() {
            return Err(HttpError::ServerError {
                status: status.as_u16(),
                message: response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string()),
            });
        }

        response
            .json::<T>()
            .await
            .map_err(|e| HttpError::Network(format!("Failed to parse JSON: {}", e)))
    }

    /// Execute POST request with resilience features
    async fn execute_post_request<S: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &S,
        request_id: &Uuid,
    ) -> Result<T, HttpError> {
        let full_url = if path.starts_with("http") {
            path.to_string()
        } else {
            format!("{}{}", self.base_url, path)
        };

        let mut request = self.client.post(&full_url);
        request = self.apply_auth(request);
        request = request
            .header("User-Agent", "ably-rust-sdk/0.1.0")
            .header("Content-Type", "application/json")
            .header("X-Request-ID", request_id.to_string())
            .header("X-Ably-Version", "3")
            .header("X-Ably-Lib", format!("rust-{}", env!("CARGO_PKG_VERSION")))
            .json(body);

        let response = request.send().await?;
        let status = response.status();

        if status == 401 {
            return Err(HttpError::AuthenticationFailed {
                status: status.as_u16(),
            });
        } else if status == 404 {
            return Err(HttpError::NotFound {
                message: response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Not found".to_string()),
            });
        } else if status == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok());
            return Err(HttpError::RateLimited { retry_after });
        } else if !status.is_success() {
            return Err(HttpError::ServerError {
                status: status.as_u16(),
                message: response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string()),
            });
        }

        response
            .json::<T>()
            .await
            .map_err(|e| HttpError::Network(format!("Failed to parse JSON: {}", e)))
    }

    /// Enhanced GET with full production features
    #[instrument(skip(self), fields(request_id = %Uuid::new_v4()))]
    pub async fn get_json_enhanced<T: DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<T, HttpError> {
        let request_id = Uuid::new_v4();
        debug!("Starting GET request to {} with ID {}", path, request_id);

        // Check rate limiting
        if let Some(limiter) = &self.rate_limiter {
            if let Err(_) = limiter.check_rate_limit().await {
                warn!("Rate limit exceeded for GET {}", path);
                return Err(HttpError::RateLimited { retry_after: Some(1) });
            }
        }

        // Execute with retry and circuit breaker
        let result = self
            .retry_policy
            .execute_with_retry(
                || async {
                    let start = Instant::now();

                    // Check circuit breaker
                    let result = if let Some(cb) = &self.circuit_breaker {
                        cb.call(self.execute_get_request(path, &request_id))
                            .await
                            .map_err(|e| HttpError::Network(format!("Circuit breaker: {:?}", e)))
                    } else {
                        self.execute_get_request(path, &request_id).await
                    };

                    // Record metrics
                    match &result {
                        Ok(_) => self.metrics.record_request(true, start.elapsed()),
                        Err(_) => self.metrics.record_request(false, start.elapsed()),
                    }

                    result
                },
                |e| e.is_retryable(),
            )
            .await;

        match &result {
            Ok(_) => info!("Request {} completed successfully", request_id),
            Err(e) => error!("Request {} failed: {:?}", request_id, e),
        }

        result
    }

    /// Enhanced POST with full production features
    #[instrument(skip(self, body), fields(request_id = %Uuid::new_v4()))]
    pub async fn post_json_enhanced<S: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &S,
    ) -> Result<T, HttpError> {
        let request_id = Uuid::new_v4();
        debug!("Starting POST request to {} with ID {}", path, request_id);

        // Check rate limiting
        if let Some(limiter) = &self.rate_limiter {
            if let Err(_) = limiter.check_rate_limit().await {
                warn!("Rate limit exceeded for POST {}", path);
                return Err(HttpError::RateLimited { retry_after: Some(1) });
            }
        }

        // Execute with retry and circuit breaker
        let result = self
            .retry_policy
            .execute_with_retry(
                || async {
                    let start = Instant::now();

                    // Check circuit breaker
                    let result = if let Some(cb) = &self.circuit_breaker {
                        cb.call(self.execute_post_request(path, body, &request_id))
                            .await
                            .map_err(|e| HttpError::Network(format!("Circuit breaker: {:?}", e)))
                    } else {
                        self.execute_post_request(path, body, &request_id).await
                    };

                    // Record metrics
                    match &result {
                        Ok(_) => self.metrics.record_request(true, start.elapsed()),
                        Err(_) => self.metrics.record_request(false, start.elapsed()),
                    }

                    result
                },
                |e| e.is_retryable(),
            )
            .await;

        match &result {
            Ok(_) => info!("Request {} completed successfully", request_id),
            Err(e) => error!("Request {} failed: {:?}", request_id, e),
        }

        result
    }
}