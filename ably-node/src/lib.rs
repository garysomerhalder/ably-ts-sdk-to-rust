//! Node.js bindings for Ably Rust SDK

pub fn node_version() -> &'static str {
    ably_core::version()
}