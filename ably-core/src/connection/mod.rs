// Connection management module

pub mod state_machine;

pub use state_machine::{
    ConnectionStateMachine, ConnectionState, ConnectionEvent,
    ChannelStateMachine, ChannelState, ChannelEvent
};