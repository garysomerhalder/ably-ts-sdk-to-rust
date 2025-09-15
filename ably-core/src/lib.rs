//! Ably Rust SDK Core Library
//! 
//! This crate provides the core functionality for the Ably Rust SDK,
//! including REST and Realtime clients, authentication, and transport layers.

pub mod error;
pub mod client;
pub mod retry;

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