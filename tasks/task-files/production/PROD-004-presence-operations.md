# PROD-004: Implement Realtime Presence Operations

## 🎯 Objective
Complete the realtime presence system with enter, leave, and update operations for full JS SDK parity.

## 📋 Task Details

**Priority:** 🔴 HIGH (Feature Gap)
**Effort:** 3-4 days
**Assignee:** Senior Rust Engineer
**Dependencies:** PROD-003 (for SYNC handling)

## 🔍 Problem Analysis

Current implementation only has `presence_get()` with a TODO comment. Missing enter/leave/update operations that are critical for collaborative features.

## ✅ Acceptance Criteria

1. [ ] Presence enter() with client ID and data
2. [ ] Presence leave() removes member
3. [ ] Presence update() changes member data
4. [ ] Presence subscribe() for events
5. [ ] Presence sync during connection recovery
6. [ ] Integration tests with multiple clients

## 🛠️ Implementation

```rust
// client/realtime.rs - Complete presence implementation
impl RealtimeChannel {
    pub async fn presence_enter(&self, data: Option<Value>) -> AblyResult<()> {
        let msg = PresenceMessage {
            action: PresenceAction::Enter,
            client_id: Some(self.client_id.clone()),
            data,
            timestamp: Some(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as i64),
            ..Default::default()
        };

        self.send_presence_message(msg).await
    }

    pub async fn presence_leave(&self, data: Option<Value>) -> AblyResult<()> {
        let msg = PresenceMessage {
            action: PresenceAction::Leave,
            client_id: Some(self.client_id.clone()),
            data,
            ..Default::default()
        };

        self.send_presence_message(msg).await
    }

    pub async fn presence_update(&self, data: Option<Value>) -> AblyResult<()> {
        let msg = PresenceMessage {
            action: PresenceAction::Update,
            client_id: Some(self.client_id.clone()),
            data,
            ..Default::default()
        };

        self.send_presence_message(msg).await
    }

    pub async fn presence_subscribe(&self) -> mpsc::Receiver<PresenceEvent> {
        let (tx, rx) = mpsc::channel(100);
        self.presence_subscribers.write().await.push(tx);
        rx
    }

    async fn send_presence_message(&self, msg: PresenceMessage) -> AblyResult<()> {
        let protocol_msg = ProtocolMessage {
            action: Action::Presence,
            channel: Some(self.name.clone()),
            presence: Some(vec![msg]),
            ..Default::default()
        };

        self.transport.send(protocol_msg).await
    }
}

// Presence set management
pub struct PresenceMap {
    members: Arc<RwLock<HashMap<String, PresenceMember>>>,
    sync_serial: Option<String>,
}

impl PresenceMap {
    pub async fn apply_presence_message(&mut self, msg: &PresenceMessage) {
        let client_id = msg.client_id.as_ref().unwrap();

        match msg.action {
            PresenceAction::Enter | PresenceAction::Present => {
                self.members.write().await.insert(
                    client_id.clone(),
                    PresenceMember::from(msg)
                );
            },
            PresenceAction::Leave | PresenceAction::Absent => {
                self.members.write().await.remove(client_id);
            },
            PresenceAction::Update => {
                if let Some(member) = self.members.write().await.get_mut(client_id) {
                    member.data = msg.data.clone();
                    member.timestamp = msg.timestamp;
                }
            },
        }
    }

    pub async fn get_members(&self) -> Vec<PresenceMember> {
        self.members.read().await.values().cloned().collect()
    }
}
```

## 🧪 Testing

```rust
#[tokio::test]
async fn test_presence_operations() {
    let client1 = RealtimeClient::new(API_KEY);
    let client2 = RealtimeClient::new(API_KEY);

    let channel1 = client1.channel("presence-test").await;
    let channel2 = client2.channel("presence-test").await;

    // Client 1 enters
    channel1.presence_enter(Some(json!({"status": "online"}))).await.unwrap();

    // Client 2 should see client 1
    let members = channel2.presence_get().await.unwrap();
    assert_eq!(members.len(), 1);

    // Client 1 updates
    channel1.presence_update(Some(json!({"status": "away"}))).await.unwrap();

    // Client 1 leaves
    channel1.presence_leave(None).await.unwrap();

    // Verify empty presence set
    let members = channel2.presence_get().await.unwrap();
    assert_eq!(members.len(), 0);
}
```

## 📊 Success Metrics

- ✅ All presence operations functional
- ✅ Presence sync during recovery
- ✅ < 100ms operation latency
- ✅ Supports 1000+ presence members