# PROD-009: Add Connection Recovery with Exponential Backoff

## 🎯 Objective
Implement automatic connection recovery with exponential backoff for resilient WebSocket connections.

## 📋 Task Details

**Priority:** 🟡 MEDIUM
**Effort:** 2 days
**Assignee:** Rust Engineer
**Dependencies:** PROD-002

## 🔍 Problem

No automatic reconnection on connection failures, requiring manual intervention.

## ✅ Acceptance Criteria

1. [ ] Automatic reconnection on disconnect
2. [ ] Exponential backoff with jitter
3. [ ] Maximum retry limits
4. [ ] Connection state preservation
5. [ ] Message queuing during reconnection

## 🛠️ Implementation

```rust
// transport/reconnect_manager.rs
pub struct ReconnectManager {
    max_retries: u32,
    base_delay: Duration,
    max_delay: Duration,
    current_attempt: AtomicU32,
}

impl ReconnectManager {
    pub async fn execute_with_retry<F, T>(&self, operation: F) -> Result<T, TransportError>
    where
        F: Fn() -> BoxFuture<'static, Result<T, TransportError>>,
    {
        let mut attempt = 0;

        loop {
            match operation().await {
                Ok(result) => {
                    self.current_attempt.store(0, Ordering::SeqCst);
                    return Ok(result);
                }
                Err(e) if attempt >= self.max_retries => {
                    return Err(e);
                }
                Err(e) => {
                    attempt += 1;
                    let delay = self.calculate_backoff(attempt);

                    warn!("Connection failed (attempt {}/{}): {}. Retrying in {:?}",
                          attempt, self.max_retries, e, delay);

                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    fn calculate_backoff(&self, attempt: u32) -> Duration {
        let exponential = self.base_delay * 2u32.pow(attempt - 1);
        let capped = exponential.min(self.max_delay);

        // Add jitter (±25%)
        let jitter = rand::random::<f64>() * 0.5 - 0.25;
        let jittered = capped.as_secs_f64() * (1.0 + jitter);

        Duration::from_secs_f64(jittered)
    }
}

// Integration with WebSocketTransport
impl WebSocketTransport {
    async fn maintain_connection(&mut self) {
        let reconnect_manager = ReconnectManager::new();

        loop {
            if !self.is_connected() {
                match reconnect_manager.execute_with_retry(|| {
                    Box::pin(self.connect())
                }).await {
                    Ok(_) => {
                        info!("Successfully reconnected");
                        self.recover_state().await;
                    }
                    Err(e) => {
                        error!("Failed to reconnect after max retries: {}", e);
                        self.set_state(ConnectionState::Failed).await;
                        break;
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    async fn recover_state(&mut self) {
        // Re-attach channels
        for channel in self.channels.read().await.values() {
            self.reattach_channel(channel).await;
        }

        // Resume message flow
        self.resume_connection().await;
    }
}
```

## 📊 Success Metrics

- ✅ 99.9% connection uptime
- ✅ Recovery within 30 seconds
- ✅ No message loss during recovery
- ✅ Graceful degradation on persistent failures