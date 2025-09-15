// Protocol message types implementation
// Comprehensive support for all 22 Ably protocol actions

use serde::{Serialize, Deserialize};
use serde_repr::{Serialize_repr, Deserialize_repr};
use serde_json::Value;
use std::collections::HashMap;

/// Complete protocol message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolMessage {
    pub action: Action,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorInfo>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_serial: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_key: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_serial: Option<i64>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msg_serial: Option<i64>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<Message>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence: Option<Vec<PresenceMessage>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthDetails>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_details: Option<ConnectionDetails>,
}

/// Action types (all 22 protocol actions)
#[derive(Debug, Clone, Copy, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum Action {
    Heartbeat = 0,
    Ack = 1,
    Nack = 2,
    Connect = 3,
    Connected = 4,
    Disconnect = 5,
    Disconnected = 6,
    Close = 7,
    Closed = 8,
    Error = 9,
    Attach = 10,
    Attached = 11,
    Detach = 12,
    Detached = 13,
    Presence = 14,
    Message = 15,
    Sync = 16,
    Auth = 17,
    Activate = 18,
    Object = 19,
    ObjectSync = 20,
    Annotation = 21,
}

/// Message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_key: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, Value>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
}

/// Presence message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresenceMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    
    pub action: PresenceAction,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
}

/// Presence action types
#[derive(Debug, Clone, Copy, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum PresenceAction {
    Absent = 0,
    Present = 1,
    Enter = 2,
    Leave = 3,
    Update = 4,
}

/// Error information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorInfo {
    pub code: u16,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cause: Option<Box<ErrorInfo>>,
}

/// Authentication details
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
}

/// Connection details
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_key: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_message_size: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_frame_size: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_inbound_rate: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_state_ttl: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
}

/// Channel flags
pub mod flags {
    pub const PRESENCE: u32 = 1 << 0;
    pub const PUBLISH: u32 = 1 << 1;
    pub const SUBSCRIBE: u32 = 1 << 2;
    pub const PRESENCE_SUBSCRIBE: u32 = 1 << 3;
    pub const HAS_PRESENCE: u32 = 1 << 16;
    pub const HAS_BACKLOG: u32 = 1 << 17;
    pub const RESUMED: u32 = 1 << 18;
    pub const TRANSIENT: u32 = 1 << 19;
    pub const ATTACH_RESUME: u32 = 1 << 20;
}

impl ProtocolMessage {
    /// Create a connect message
    pub fn connect(client_id: Option<String>, auth: Option<AuthDetails>) -> Self {
        Self {
            action: Action::Connect,
            auth,
            connection_details: client_id.map(|id| ConnectionDetails {
                client_id: Some(id),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    /// Create an attach message for a channel
    pub fn attach(channel: String, flags: Option<u32>) -> Self {
        Self {
            action: Action::Attach,
            channel: Some(channel),
            flags,
            ..Default::default()
        }
    }

    /// Create a detach message for a channel
    pub fn detach(channel: String) -> Self {
        Self {
            action: Action::Detach,
            channel: Some(channel),
            ..Default::default()
        }
    }

    /// Create a message to publish
    pub fn message(channel: String, messages: Vec<Message>) -> Self {
        Self {
            action: Action::Message,
            channel: Some(channel),
            messages: Some(messages),
            ..Default::default()
        }
    }

    /// Create a presence message
    pub fn presence(channel: String, presence: Vec<PresenceMessage>) -> Self {
        Self {
            action: Action::Presence,
            channel: Some(channel),
            presence: Some(presence),
            ..Default::default()
        }
    }

    /// Create a heartbeat message
    pub fn heartbeat() -> Self {
        Self {
            action: Action::Heartbeat,
            ..Default::default()
        }
    }

    /// Create an ack message
    pub fn ack(msg_serial: i64, count: Option<u32>) -> Self {
        Self {
            action: Action::Ack,
            msg_serial: Some(msg_serial),
            count,
            ..Default::default()
        }
    }

    /// Create a nack message
    pub fn nack(msg_serial: i64, count: Option<u32>, error: Option<ErrorInfo>) -> Self {
        Self {
            action: Action::Nack,
            msg_serial: Some(msg_serial),
            count,
            error,
            ..Default::default()
        }
    }

    /// Create an error message
    pub fn error(error: ErrorInfo, channel: Option<String>) -> Self {
        Self {
            action: Action::Error,
            error: Some(error),
            channel,
            ..Default::default()
        }
    }
}

impl Default for ProtocolMessage {
    fn default() -> Self {
        Self {
            action: Action::Heartbeat,
            flags: None,
            count: None,
            error: None,
            id: None,
            channel: None,
            channel_serial: None,
            connection_id: None,
            connection_key: None,
            connection_serial: None,
            msg_serial: None,
            timestamp: None,
            messages: None,
            presence: None,
            auth: None,
            connection_details: None,
        }
    }
}

impl Default for ConnectionDetails {
    fn default() -> Self {
        Self {
            client_id: None,
            connection_key: None,
            max_message_size: None,
            max_frame_size: None,
            max_inbound_rate: None,
            connection_state_ttl: None,
            server_id: None,
        }
    }
}