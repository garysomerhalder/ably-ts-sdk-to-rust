// Protocol messages module
// Comprehensive implementation of Ably protocol v3

pub mod messages;
pub mod encoding;

// Re-export key types
pub use messages::{
    ProtocolMessage, Action, Message, PresenceMessage, PresenceAction,
    ErrorInfo, AuthDetails, ConnectionDetails, flags
};