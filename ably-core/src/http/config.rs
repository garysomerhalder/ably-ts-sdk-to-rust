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
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(10),
            connect_timeout: Duration::from_secs(5),
            pool_idle_timeout: Some(Duration::from_secs(90)),
            pool_max_idle_per_host: 32,
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

    /// Build the configuration
    pub fn build(self) -> HttpConfig {
        let default = HttpConfig::default();
        HttpConfig {
            timeout: self.timeout.unwrap_or(default.timeout),
            connect_timeout: self.connect_timeout.unwrap_or(default.connect_timeout),
            pool_idle_timeout: self.pool_idle_timeout.or(default.pool_idle_timeout),
            pool_max_idle_per_host: self.pool_max_idle_per_host.unwrap_or(default.pool_max_idle_per_host),
        }
    }
}