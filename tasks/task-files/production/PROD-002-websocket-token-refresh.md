# PROD-002: Implement WebSocket Token Refresh Mechanism

## 🎯 Objective
Implement automatic token refresh for WebSocket connections to prevent disconnections when tokens expire, ensuring long-lived realtime connections remain stable.

## 📋 Task Details

**Priority:** 🔴 CRITICAL (Production Blocker)
**Effort:** 2-3 days
**Assignee:** Senior Rust/TypeScript Engineer
**Dependencies:** PROD-001 (for proper AUTH message parsing)

## 🔍 Problem Analysis

WebSocket connections currently drop when tokens expire with no automatic refresh mechanism. This breaks long-lived connections and requires manual reconnection, degrading user experience.

### Current Issue
```rust
// transport/websocket.rs - No token refresh logic
pub async fn connect(&mut self) -> Result<(), TransportError> {
    let url = format!("wss://realtime.ably.io/?key={}&v=3&format=json", self.auth);
    // Connection drops when token expires - no refresh handling
}
```

### Required Behavior (from JS SDK)
```typescript
// JS SDK automatically refreshes tokens before expiry
connection.on('tokenExpiring', async () => {
    const newToken = await auth.requestToken();
    connection.auth.authorize(newToken);
});
```

## ✅ Acceptance Criteria

1. [ ] WebSocket automatically requests new token before expiry
2. [ ] Connection remains stable during token refresh
3. [ ] No message loss during token transition
4. [ ] Graceful fallback if token refresh fails
5. [ ] Token refresh events properly logged

## 🛠️ Implementation Plan

### Phase 1: Token Lifecycle Management (Day 1)

```rust
// auth/token_manager.rs
pub struct TokenManager {
    current_token: Arc<RwLock<Option<TokenDetails>>>,
    auth_callback: Option<Box<dyn Fn() -> BoxFuture<'static, Result<TokenDetails, AblyError>> + Send + Sync>>,
    refresh_margin: Duration, // Refresh 30 seconds before expiry
}

impl TokenManager {
    pub async fn get_valid_token(&self) -> Result<String, AblyError> {
        let token = self.current_token.read().await;

        if let Some(token_details) = token.as_ref() {
            if self.is_token_expiring(token_details) {
                drop(token);
                self.refresh_token().await?;
            } else {
                return Ok(token_details.token.clone());
            }
        } else {
            self.refresh_token().await?;
        }

        self.current_token.read().await
            .as_ref()
            .map(|t| t.token.clone())
            .ok_or_else(|| AblyError::token_expired())
    }

    fn is_token_expiring(&self, token: &TokenDetails) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        token.expires - now < self.refresh_margin.as_millis() as i64
    }

    async fn refresh_token(&self) -> Result<(), AblyError> {
        if let Some(callback) = &self.auth_callback {
            let new_token = callback().await?;
            *self.current_token.write().await = Some(new_token);
            Ok(())
        } else {
            Err(AblyError::no_auth_callback())
        }
    }
}
```

### Phase 2: WebSocket Integration (Day 2)

```rust
// transport/websocket.rs
pub struct WebSocketTransport {
    token_manager: Arc<TokenManager>,
    refresh_handle: Option<JoinHandle<()>>,
    // ... existing fields
}

impl WebSocketTransport {
    pub async fn connect(&mut self) -> Result<(), TransportError> {
        let token = self.token_manager.get_valid_token().await?;
        let url = format!("wss://realtime.ably.io/?token={}&v=3&format=json", token);

        // Start token refresh timer
        self.start_token_refresh_timer().await;

        // ... existing connection logic
    }

    async fn start_token_refresh_timer(&mut self) {
        let token_manager = Arc::clone(&self.token_manager);
        let tx = self.sender.clone();

        self.refresh_handle = Some(tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;

                if token_manager.needs_refresh().await {
                    match token_manager.refresh_token().await {
                        Ok(new_token) => {
                            // Send AUTH protocol message with new token
                            let auth_msg = ProtocolMessage {
                                action: Action::Auth,
                                auth: Some(AuthDetails {
                                    access_token: Some(new_token),
                                    ..Default::default()
                                }),
                                ..Default::default()
                            };

                            if let Err(e) = tx.send(Message::Text(
                                serde_json::to_string(&auth_msg).unwrap()
                            )).await {
                                error!("Failed to send AUTH message: {}", e);
                                break;
                            }
                        },
                        Err(e) => {
                            error!("Token refresh failed: {}", e);
                            // Trigger reconnection
                            break;
                        }
                    }
                }
            }
        }));
    }

    async fn handle_auth_response(&mut self, msg: &ProtocolMessage) {
        if msg.action == Action::Connected || msg.action == Action::Ack {
            info!("Token refresh successful");
            // Update connection details if needed
        } else if msg.action == Action::Error {
            error!("Token refresh rejected: {:?}", msg.error);
            // Trigger reconnection with new token
            self.reconnect_with_new_token().await;
        }
    }
}
```

### Phase 3: Integration & Testing (Day 3)

```rust
// client/realtime.rs
impl RealtimeClient {
    pub fn new_with_token_callback<F>(callback: F) -> Self
    where
        F: Fn() -> BoxFuture<'static, Result<TokenDetails, AblyError>> + Send + Sync + 'static
    {
        let token_manager = Arc::new(TokenManager::new_with_callback(Box::new(callback)));

        Self {
            transport: WebSocketTransport::new_with_token_manager(token_manager),
            // ... other fields
        }
    }
}

// Integration test
#[tokio::test]
async fn test_token_refresh_during_connection() {
    let token_requests = Arc::new(AtomicU32::new(0));
    let requests_clone = Arc::clone(&token_requests);

    let client = RealtimeClient::new_with_token_callback(move || {
        let count = requests_clone.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            // Simulate token request
            let token = request_token_from_server().await?;
            Ok(TokenDetails {
                token,
                expires: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64 + 60000, // 1 minute expiry
                ..Default::default()
            })
        })
    });

    client.connect().await.unwrap();

    // Wait for token to expire and refresh
    tokio::time::sleep(Duration::from_secs(90)).await;

    // Verify token was refreshed
    assert!(token_requests.load(Ordering::SeqCst) >= 2);

    // Verify connection still active
    assert!(client.is_connected().await);
}
```

## 🚦 Traffic-Light Development

### 🔴 RED Phase
- Write tests showing connection drops on token expiry
- Document current failure modes
- Capture token expiry scenarios from production

### 🟡 YELLOW Phase
- Basic token refresh before expiry
- Send AUTH message to server
- Handle AUTH response

### 🟢 GREEN Phase
- Message queuing during refresh
- Exponential backoff on refresh failure
- Comprehensive error handling
- Performance monitoring

## 🧪 Testing Strategy

1. **Unit Tests**: Token expiry detection, refresh timing
2. **Integration Tests**: Full token lifecycle with real API
3. **Long-running Tests**: 24-hour connection stability
4. **Failure Scenarios**: Network issues during refresh

## 📚 References

- [Ably Token Authentication](https://ably.com/docs/core-features/authentication#token-authentication)
- [JS SDK Token Refresh](https://github.com/ably/ably-js/blob/main/src/common/lib/client/auth.ts)
- [WebSocket AUTH Protocol](https://ably.com/docs/realtime/connection#authentication)

## ⚠️ Risks & Mitigations

- **Risk**: Token refresh fails repeatedly
- **Mitigation**: Exponential backoff and reconnection

- **Risk**: Message loss during refresh
- **Mitigation**: Queue messages during AUTH exchange

## 📊 Success Metrics

- ✅ Zero disconnections due to token expiry
- ✅ Token refresh < 100ms latency
- ✅ 99.9% connection uptime over 24 hours
- ✅ Successful refresh rate > 99%