// Client module organization

pub mod rest;
pub mod realtime;

// Re-export main types
pub use rest::{RestClient, Channel};
pub use crate::protocol::messages::Message;

// Keep legacy client for backward compatibility
mod legacy;
pub use legacy::RestClient as LegacyRestClient;