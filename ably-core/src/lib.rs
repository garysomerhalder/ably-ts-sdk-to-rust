//! Ably Rust SDK Core Library
//! 
//! This crate provides the core functionality for the Ably Rust SDK,
//! including REST and Realtime clients, authentication, and transport layers.

pub mod auth;
pub mod client;
pub mod error;
pub mod http;
pub mod logging;
pub mod protocol;
pub mod retry;
pub mod transport;

pub fn version() -> &'static str {
    "0.1.0"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(version(), "0.1.0");
    }
}