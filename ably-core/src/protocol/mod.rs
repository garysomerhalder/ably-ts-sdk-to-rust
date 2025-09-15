// Protocol messages module
// Comprehensive implementation of Ably protocol v3

pub mod messages;
pub mod encoding;

// Re-export key types
pub use messages::{
    ProtocolMessage, Action, Message, PresenceMessage, PresenceAction,
    ErrorInfo, AuthDetails, ConnectionDetails, flags
};

// Keep compatibility with transport module
pub use crate::transport::{ProtocolMessage as TransportProtocolMessage, Action as TransportAction};