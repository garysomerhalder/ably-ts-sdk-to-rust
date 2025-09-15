// Transport configuration

use std::time::Duration;

/// WebSocket transport configuration
#[derive(Debug, Clone)]
pub struct TransportConfig {
    /// Use binary protocol (MessagePack) instead of JSON
    pub use_binary_protocol: bool,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Enable auto-reconnect
    pub enable_auto_reconnect: bool,
    /// Reconnection delay
    pub reconnect_delay: Duration,
    /// Maximum frame size
    pub max_frame_size: usize,
    /// Keepalive interval
    pub keepalive_interval: Duration,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            use_binary_protocol: false,
            connection_timeout: Duration::from_secs(10),
            enable_auto_reconnect: true,
            reconnect_delay: Duration::from_secs(2),
            max_frame_size: 1024 * 1024, // 1MB
            keepalive_interval: Duration::from_secs(30),
        }
    }
}

impl TransportConfig {
    /// Create configuration builder
    pub fn builder() -> TransportConfigBuilder {
        TransportConfigBuilder::default()
    }
}

/// Transport configuration builder
#[derive(Default)]
pub struct TransportConfigBuilder {
    use_binary_protocol: Option<bool>,
    connection_timeout: Option<Duration>,
    enable_auto_reconnect: Option<bool>,
    reconnect_delay: Option<Duration>,
    max_frame_size: Option<usize>,
    keepalive_interval: Option<Duration>,
}

impl TransportConfigBuilder {
    /// Use binary protocol
    pub fn use_binary_protocol(mut self, use_binary: bool) -> Self {
        self.use_binary_protocol = Some(use_binary);
        self
    }

    /// Set connection timeout
    pub fn connection_timeout(mut self, timeout: Duration) -> Self {
        self.connection_timeout = Some(timeout);
        self
    }

    /// Enable auto-reconnect
    pub fn enable_auto_reconnect(mut self, enable: bool) -> Self {
        self.enable_auto_reconnect = Some(enable);
        self
    }

    /// Set reconnect delay
    pub fn reconnect_delay(mut self, delay: Duration) -> Self {
        self.reconnect_delay = Some(delay);
        self
    }

    /// Set maximum frame size
    pub fn max_frame_size(mut self, size: usize) -> Self {
        self.max_frame_size = Some(size);
        self
    }

    /// Set keepalive interval
    pub fn keepalive_interval(mut self, interval: Duration) -> Self {
        self.keepalive_interval = Some(interval);
        self
    }

    /// Build configuration
    pub fn build(self) -> TransportConfig {
        let default = TransportConfig::default();
        
        TransportConfig {
            use_binary_protocol: self.use_binary_protocol.unwrap_or(default.use_binary_protocol),
            connection_timeout: self.connection_timeout.unwrap_or(default.connection_timeout),
            enable_auto_reconnect: self.enable_auto_reconnect.unwrap_or(default.enable_auto_reconnect),
            reconnect_delay: self.reconnect_delay.unwrap_or(default.reconnect_delay),
            max_frame_size: self.max_frame_size.unwrap_or(default.max_frame_size),
            keepalive_interval: self.keepalive_interval.unwrap_or(default.keepalive_interval),
        }
    }
}