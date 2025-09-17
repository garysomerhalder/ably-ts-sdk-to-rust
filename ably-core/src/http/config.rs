// HTTP client configuration

use std::time::Duration;

/// HTTP client configuration
#[derive(Debug, Clone)]
pub struct HttpConfig {
    /// Request timeout
    pub timeout: Duration,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Pool idle timeout
    pub pool_idle_timeout: Option<Duration>,
    /// Maximum idle connections per host
    pub pool_max_idle_per_host: usize,
    /// Maximum number of retries
    pub max_retries: u32,
    /// Base URL for API requests
    pub base_url: String,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(10),
            connect_timeout: Duration::from_secs(5),
            pool_idle_timeout: Some(Duration::from_secs(90)),
            pool_max_idle_per_host: 32,
            max_retries: 3,
            base_url: "https://rest.ably.io".to_string(),
        }
    }
}

impl HttpConfig {
    /// Create a new configuration builder
    pub fn builder() -> HttpConfigBuilder {
        HttpConfigBuilder::default()
    }
}

/// HTTP configuration builder
#[derive(Default)]
pub struct HttpConfigBuilder {
    timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
    pool_idle_timeout: Option<Duration>,
    pool_max_idle_per_host: Option<usize>,
    max_retries: Option<u32>,
    base_url: Option<String>,
}

impl HttpConfigBuilder {
    /// Set request timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set connection timeout
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }

    /// Set pool idle timeout
    pub fn pool_idle_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.pool_idle_timeout = timeout;
        self
    }

    /// Set maximum idle connections per host
    pub fn pool_max_idle_per_host(mut self, max: usize) -> Self {
        self.pool_max_idle_per_host = Some(max);
        self
    }

    /// Set max retries
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = Some(retries);
        self
    }

    /// Set base URL
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Build the configuration
    pub fn build(self) -> HttpConfig {
        let default = HttpConfig::default();
        HttpConfig {
            timeout: self.timeout.unwrap_or(default.timeout),
            connect_timeout: self.connect_timeout.unwrap_or(default.connect_timeout),
            pool_idle_timeout: self.pool_idle_timeout.or(default.pool_idle_timeout),
            pool_max_idle_per_host: self.pool_max_idle_per_host.unwrap_or(default.pool_max_idle_per_host),
            max_retries: self.max_retries.unwrap_or(default.max_retries),
            base_url: self.base_url.unwrap_or(default.base_url),
        }
    }
}