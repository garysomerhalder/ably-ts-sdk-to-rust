# PROD-003: Complete Missing Protocol Message Handlers

## 🎯 Objective
Implement the missing protocol message handlers (SYNC, AUTH, ACTIVATE) to ensure complete WebSocket message flow compatibility with Ably protocol v3.

## 📋 Task Details

**Priority:** 🔴 CRITICAL (Protocol Compliance)
**Effort:** 5-6 days
**Assignee:** Senior Rust Engineer
**Dependencies:** None

## 🔍 Problem Analysis

The WebSocket transport currently doesn't handle all 22 protocol action types, specifically missing SYNC, AUTH, and ACTIVATE handlers which are critical for:
- **SYNC**: Channel state synchronization after connection recovery
- **AUTH**: Dynamic authentication updates during connection
- **ACTIVATE**: Channel activation confirmation

### Current Gap
```rust
// connection/state_machine.rs - Incomplete handler
match msg.action {
    Action::Connected => { /* handled */ },
    Action::Message => { /* handled */ },
    Action::Sync => {
        // TODO: Implement SYNC handling
        warn!("SYNC action not implemented");
    },
    Action::Auth => {
        // TODO: Implement AUTH handling
        warn!("AUTH action not implemented");
    },
    Action::Activate => {
        // TODO: Implement ACTIVATE handling
        warn!("ACTIVATE action not implemented");
    },
    // ... other handlers
}
```

## ✅ Acceptance Criteria

1. [ ] SYNC handler processes channel state synchronization
2. [ ] AUTH handler manages dynamic authentication
3. [ ] ACTIVATE handler confirms channel activation
4. [ ] All handlers integrate with state machines
5. [ ] Protocol compliance tests pass
6. [ ] Message ordering preserved during SYNC

## 🛠️ Implementation Plan

### Phase 1: SYNC Handler Implementation (Day 1-2)

```rust
// protocol/sync_handler.rs
pub struct SyncHandler {
    channel_states: Arc<RwLock<HashMap<String, ChannelSyncState>>>,
}

#[derive(Debug, Clone)]
pub struct ChannelSyncState {
    pub channel: String,
    pub presence_sync_serial: Option<String>,
    pub message_serial: Option<i64>,
    pub sync_complete: bool,
}

impl SyncHandler {
    pub async fn handle_sync(&mut self, msg: &ProtocolMessage) -> Result<(), ProtocolError> {
        let channel_name = msg.channel
            .as_ref()
            .ok_or_else(|| ProtocolError::missing_channel())?;

        let mut states = self.channel_states.write().await;
        let state = states.entry(channel_name.clone())
            .or_insert_with(|| ChannelSyncState::new(channel_name));

        // Process based on sync state
        match (msg.sync_sequence, msg.presence, msg.messages) {
            // Initial SYNC - start of sequence
            (Some(seq), _, _) if seq.starts_with("0:") => {
                info!("Starting SYNC for channel {}", channel_name);
                state.sync_complete = false;
                self.process_sync_start(state, msg).await?;
            },

            // Intermediate SYNC - continuing sequence
            (Some(seq), _, _) if !seq.ends_with(":") => {
                self.process_sync_continuation(state, msg).await?;
            },

            // Final SYNC - sequence complete
            (Some(seq), _, _) if seq.ends_with(":") => {
                info!("Completing SYNC for channel {}", channel_name);
                self.process_sync_end(state, msg).await?;
                state.sync_complete = true;

                // Notify channel it's synchronized
                self.notify_channel_synced(channel_name).await;
            },

            _ => {
                warn!("Invalid SYNC message format");
            }
        }

        Ok(())
    }

    async fn process_sync_messages(&self, state: &mut ChannelSyncState, messages: &[Message]) {
        // Sort messages by serial and apply to channel
        let mut sorted_messages = messages.to_vec();
        sorted_messages.sort_by_key(|m| m.msg_serial);

        for msg in sorted_messages {
            // Update state serial
            if let Some(serial) = msg.msg_serial {
                state.message_serial = Some(serial);
            }

            // Queue message for delivery
            self.queue_synced_message(msg).await;
        }
    }

    async fn process_presence_sync(&self, state: &mut ChannelSyncState, presence: &[PresenceMessage]) {
        // Synchronize presence set
        for presence_msg in presence {
            match presence_msg.action {
                PresenceAction::Present | PresenceAction::Enter => {
                    self.add_presence_member(presence_msg).await;
                },
                PresenceAction::Leave | PresenceAction::Absent => {
                    self.remove_presence_member(presence_msg).await;
                },
                _ => {}
            }
        }
    }
}
```

### Phase 2: AUTH Handler Implementation (Day 3-4)

```rust
// protocol/auth_handler.rs
pub struct AuthHandler {
    auth_manager: Arc<TokenManager>,
    connection_state: Arc<RwLock<ConnectionState>>,
}

impl AuthHandler {
    pub async fn handle_auth(&mut self, msg: &ProtocolMessage) -> Result<ProtocolMessage, ProtocolError> {
        match msg.auth.as_ref() {
            // Server requesting authentication
            Some(auth_details) if auth_details.access_token.is_none() => {
                info!("Server requesting authentication");
                self.provide_authentication().await
            },

            // Server providing new token
            Some(auth_details) if auth_details.access_token.is_some() => {
                info!("Server providing new token");
                self.update_authentication(auth_details).await
            },

            // Authentication challenge
            _ if msg.connection_details.is_some() => {
                info!("Authentication challenge received");
                self.handle_auth_challenge(msg).await
            },

            _ => {
                Err(ProtocolError::invalid_auth_message())
            }
        }
    }

    async fn provide_authentication(&self) -> Result<ProtocolMessage, ProtocolError> {
        let token = self.auth_manager.get_valid_token().await?;

        Ok(ProtocolMessage {
            action: Action::Auth,
            auth: Some(AuthDetails {
                access_token: Some(token),
                ..Default::default()
            }),
            ..Default::default()
        })
    }

    async fn update_authentication(&self, auth: &AuthDetails) -> Result<ProtocolMessage, ProtocolError> {
        if let Some(token) = &auth.access_token {
            // Update stored token
            self.auth_manager.update_token(token.clone()).await?;

            // Send acknowledgment
            Ok(ProtocolMessage {
                action: Action::Ack,
                ..Default::default()
            })
        } else {
            Err(ProtocolError::missing_token())
        }
    }

    async fn handle_auth_challenge(&self, msg: &ProtocolMessage) -> Result<ProtocolMessage, ProtocolError> {
        // Extract nonce from connection details
        let nonce = msg.connection_details
            .as_ref()
            .and_then(|d| d.nonce.as_ref())
            .ok_or_else(|| ProtocolError::missing_nonce())?;

        // Generate signed response
        let signature = self.auth_manager.sign_nonce(nonce).await?;

        Ok(ProtocolMessage {
            action: Action::Auth,
            auth: Some(AuthDetails {
                signature: Some(signature),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}
```

### Phase 3: ACTIVATE Handler Implementation (Day 5)

```rust
// protocol/activate_handler.rs
pub struct ActivateHandler {
    channels: Arc<RwLock<HashMap<String, ChannelState>>>,
}

impl ActivateHandler {
    pub async fn handle_activate(&mut self, msg: &ProtocolMessage) -> Result<(), ProtocolError> {
        let channel_name = msg.channel
            .as_ref()
            .ok_or_else(|| ProtocolError::missing_channel())?;

        let mut channels = self.channels.write().await;
        let channel = channels.get_mut(channel_name)
            .ok_or_else(|| ProtocolError::channel_not_found())?;

        // Update channel state to active
        channel.state = ChannelState::Active;
        channel.activated_at = Some(SystemTime::now());

        // Extract channel parameters if provided
        if let Some(params) = &msg.params {
            channel.params = Some(params.clone());
        }

        // Process any flags
        if let Some(flags) = msg.flags {
            self.process_activation_flags(channel, flags).await;
        }

        info!("Channel {} activated successfully", channel_name);

        // Notify listeners of activation
        self.notify_channel_activated(channel_name).await;

        Ok(())
    }

    async fn process_activation_flags(&self, channel: &mut ChannelState, flags: u32) {
        // Bit flags from Ably protocol
        const HAS_PRESENCE: u32 = 1 << 0;
        const HAS_BACKLOG: u32 = 1 << 1;
        const RESUMED: u32 = 1 << 2;
        const HAS_LOCAL_PRESENCE: u32 = 1 << 3;
        const TRANSIENT: u32 = 1 << 4;

        if flags & HAS_PRESENCE != 0 {
            channel.has_presence = true;
        }

        if flags & HAS_BACKLOG != 0 {
            channel.has_backlog = true;
            // Request backlog messages
            self.request_backlog(channel.name.clone()).await;
        }

        if flags & RESUMED != 0 {
            channel.resumed = true;
            info!("Channel {} resumed from previous state", channel.name);
        }

        if flags & TRANSIENT != 0 {
            channel.transient = true;
        }
    }
}
```

### Phase 4: Integration & Testing (Day 6)

```rust
// connection/protocol_dispatcher.rs
pub struct ProtocolDispatcher {
    sync_handler: SyncHandler,
    auth_handler: AuthHandler,
    activate_handler: ActivateHandler,
    // ... other handlers
}

impl ProtocolDispatcher {
    pub async fn dispatch(&mut self, msg: ProtocolMessage) -> Result<Option<ProtocolMessage>, ProtocolError> {
        match msg.action {
            Action::Sync => {
                self.sync_handler.handle_sync(&msg).await?;
                Ok(None)
            },
            Action::Auth => {
                let response = self.auth_handler.handle_auth(&msg).await?;
                Ok(Some(response))
            },
            Action::Activate => {
                self.activate_handler.handle_activate(&msg).await?;
                Ok(None)
            },
            // ... other actions
        }
    }
}

// Integration test
#[tokio::test]
async fn test_complete_protocol_flow() {
    let client = RealtimeClient::new(ABLY_API_KEY);

    // Connect and trigger various protocol messages
    client.connect().await.unwrap();

    let channel = client.channel("test-channel").await;
    channel.attach().await.unwrap();

    // Should trigger ATTACH -> ATTACHED -> ACTIVATE flow
    assert_eq!(channel.state().await, ChannelState::Active);

    // Disconnect and reconnect to trigger SYNC
    client.disconnect().await;
    client.connect().await.unwrap();

    // Should trigger SYNC messages for channel recovery
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(channel.state().await, ChannelState::Active);
}
```

## 🚦 Traffic-Light Development

### 🔴 RED Phase
- Write tests for each missing handler
- Document expected protocol flows
- Capture real protocol messages from JS SDK

### 🟡 YELLOW Phase
- Basic handler implementation
- State updates working
- Response generation correct

### 🟢 GREEN Phase
- Full error handling
- Performance optimization
- Comprehensive logging
- Edge case handling

## 🧪 Testing Strategy

1. **Unit Tests**: Each handler in isolation
2. **Integration Tests**: Complete protocol flows
3. **Compliance Tests**: Match JS SDK behavior
4. **Stress Tests**: High message volume handling

## 📚 References

- [Ably Protocol Specification](https://ably.com/docs/client-lib/protocol)
- [Protocol Action Types](https://github.com/ably/ably-common/blob/main/protocol/README.md)
- JS SDK implementation: `src/common/lib/transport/protocol.ts`

## ⚠️ Risks & Mitigations

- **Risk**: Protocol version incompatibility
- **Mitigation**: Implement version negotiation

- **Risk**: State corruption during SYNC
- **Mitigation**: Transactional state updates

## 📊 Success Metrics

- ✅ All 22 protocol actions handled
- ✅ Zero unhandled protocol messages in logs
- ✅ Protocol compliance test suite passes
- ✅ Message ordering preserved during recovery