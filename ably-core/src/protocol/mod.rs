// Protocol messages module
// Comprehensive implementation of Ably protocol v3

pub mod messages;
pub mod encoding;
pub mod messagepack;

// Re-export key types
pub use messages::{
    ProtocolMessage, Action, Message, PresenceMessage, PresenceAction,
    ErrorInfo, AuthDetails, ConnectionDetails, flags,
    MessageFlags, ChannelDetails, ChannelStatus, ChannelOccupancy,
    ChannelMetrics, MessageData
};

pub use messagepack::{
    MessagePackEncoder, MessagePackDecoder, MessagePackExt
};