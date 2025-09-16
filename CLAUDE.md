# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This repository ports the Ably JavaScript/TypeScript SDK (v2.12.0) to Rust while maintaining 100% API compatibility. The project follows strict Traffic-Light Development methodology with Integration-First testing (no mocks). **Current completion: ~85%**.

## Development Commands

### Build & Test

```bash
# Build commands
cargo build                          # Build all workspace crates
cargo build --release                # Build optimized release version
cargo build -p ably-core            # Build specific crate (ably-core, ably-wasm, ably-node, ably-ffi)

# Testing commands (ALWAYS use real Ably API)
export ABLY_API_KEY_SANDBOX="BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA"
cargo test                           # Run all tests
cargo test -p ably-core             # Test specific crate
cargo test --test live_functionality_test  # Run specific integration test
cargo test test_live_ably_connection -- --nocapture  # Run single test with output
cargo test -- --test-threads=1      # Run tests sequentially (helps with API rate limits)

# Code quality
cargo fmt                            # Format all code
cargo clippy -- -D warnings         # Strict linting
cargo doc --open                     # Generate and view documentation
```

### Platform-Specific Builds

```bash
# WASM build
cd ably-wasm
wasm-pack build --target web --out-dir pkg

# Node.js build  
cd ably-node
npm run build

# C FFI build
cargo build -p ably-ffi --release
```

## High-Level Architecture

### Core Design Principles

1. **Integration-First Testing**: ALL tests hit real Ably APIs. Never use mocks or fakes.
2. **Traffic-Light Development**: RED (failing test) → YELLOW (minimal impl) → GREEN (production ready). Commit after each phase.
3. **Protocol Compatibility**: Must match Ably protocol v3 exactly, including all state transitions.
4. **Multi-Platform**: Single core with bindings for WASM, Node.js, and C FFI.

### Module Architecture

```
ably-core/src/
├── auth/           # Authentication (API key & token) - handles auth flow
├── client/         
│   ├── rest.rs     # REST client with builder pattern for queries
│   └── realtime.rs # Realtime client managing WebSocket lifecycle
├── connection/     # Connection state machine (8 states)
├── crypto/         # AES-128/256 CBC encryption with PKCS7 padding
├── error/          # Ably error codes (40000-50000 range) mapping
├── http/           
│   ├── mod.rs      # HTTP client with auth injection
│   └── resilience.rs # Circuit breaker, rate limiter, retry logic
├── plugin/         # Plugin system with lifecycle hooks
├── protocol/       # 22 protocol action types and message structures
├── push/           # Push notifications (FCM, APNS, web)
├── replay/         # Message history replay with positioning
└── transport/      # WebSocket with TLS, heartbeat, reconnection
```

### Key Architectural Patterns

1. **Builder Pattern**: Used extensively for complex queries (stats, history, channels)
   ```rust
   client.stats()
       .unit(StatsUnit::Hour)
       .limit(100)
       .execute()
       .await
   ```

2. **State Machines**: Connection and channel states with strict transitions
   - Connection: initialized → connecting → connected → disconnected → suspended → closing → closed → failed
   - Channel: initialized → attaching → attached → detaching → detached → suspended → failed

3. **Resilience Layer**: All network operations go through resilience wrappers
   - Circuit breaker prevents cascading failures
   - Rate limiter respects Ably's limits
   - Exponential backoff with jitter for retries

4. **Message Flow**: 
   - REST: Client → Channel → HTTP → Ably API
   - Realtime: Client → Channel → WebSocket → Protocol Handler → Ably Realtime

### Critical Implementation Details

1. **Authentication Flow**:
   - API key is split into app ID and key secret
   - Basic auth header: Base64(keyId:keySecret)
   - Token refresh needed for WebSocket reconnection

2. **JSON Response Issues**: 
   - Some Ably responses don't match expected structures
   - History and stats endpoints have parsing issues
   - Channel metadata responses need alignment

3. **WebSocket Connection**:
   - URL: `wss://realtime.ably.io?key={key}&v=3&format=json`
   - Must handle token refresh on reconnect
   - Heartbeat required every 15 seconds

4. **Encryption**:
   - Messages encrypted with channel-specific cipher params
   - IV generated per message
   - Base64 encoding for wire format

## Known Issues & Workarounds

### JSON Parsing Failures
**Issue**: History, stats, and channel list endpoints fail to parse.
**Workaround**: Check actual API responses and update structs in `protocol/messages.rs`.

### WebSocket Token Refresh
**Issue**: WebSocket disconnects when token expires.
**TODO**: Implement token refresh in `transport/websocket.rs` before token expiry.

### Rate Limiting in Tests
**Issue**: Running all tests in parallel hits rate limits.
**Workaround**: Use `cargo test -- --test-threads=1` for sequential execution.

## Testing Strategy

### Test Categories
1. **Unit Tests** (`#[cfg(test)]` in modules): Core logic without I/O
2. **Integration Tests** (`tests/` directory): Real API interactions
3. **Live Tests** (`live_functionality_test.rs`): End-to-end verification

### Key Test Files
- `tests/rest_client_test.rs` - REST API operations
- `tests/websocket_transport_test.rs` - WebSocket connection
- `tests/encryption_test.rs` - Message encryption
- `tests/live_functionality_test.rs` - Full integration test

### Running Specific Test Scenarios
```bash
# Test REST client only
cargo test --test rest_client_test

# Test with specific API key
ABLY_API_KEY_SANDBOX="your-key" cargo test

# Debug failing test
RUST_BACKTRACE=1 cargo test failing_test_name -- --nocapture
```

## Reference Implementation

When implementing new features, consult these JavaScript SDK files:

1. **Connection Management**: `ably-js/src/common/lib/transport/connectionmanager.ts` (73KB)
   - State machine implementation
   - Reconnection logic
   - Error recovery

2. **Protocol Handling**: `ably-js/src/common/lib/transport/protocol.ts`
   - Message encoding/decoding
   - Action type handling
   - Protocol negotiation

3. **Channel Operations**: `ably-js/src/common/lib/client/realtimechannel.ts` (34KB)
   - Message queuing
   - Presence handling
   - State recovery

## Performance Benchmarks

Target metrics (from JS SDK):
- Connection establishment: <200ms
- Message latency: <50ms regional
- Throughput: >10,000 messages/second
- Memory usage: <50MB for 1000 channels

Current Rust SDK (preliminary):
- Connection: ~150ms ✅
- Message latency: ~40ms ✅  
- Throughput: Untested
- Memory: ~30MB for 1000 channels ✅

## Common Development Tasks

### Adding a New Protocol Message Type
1. Add action to `protocol/mod.rs` Action enum
2. Update `ProtocolMessage` struct if needed
3. Add handler in `connection/state_machine.rs`
4. Write integration test against real API

### Implementing a New REST Endpoint
1. Add method to `RestClient` in `client/rest.rs`
2. Create builder struct for complex queries
3. Map response to protocol types
4. Test with real API response

### Adding Platform Bindings
1. Define C-compatible structs in `ably-ffi/src/lib.rs`
2. Create safe wrappers in target platform module
3. Handle memory management carefully
4. Test cross-platform compatibility

## Debugging Tips

### Connection Issues
```bash
# Enable debug logging
RUST_LOG=ably_core=debug cargo run

# Test raw WebSocket connection
wscat -c "wss://realtime.ably.io?key=BGkZHw.WUtzEQ:..."
```

### API Response Mismatches
```bash
# Capture actual response
curl -H "Authorization: Basic $(echo -n 'key' | base64)" \
     "https://rest.ably.io/channels/test/messages"

# Compare with expected struct
cargo test -- --nocapture 2>&1 | grep "Failed to parse"
```

## Project Status Files

- `/tasks/PROJECT_COMPLETION_STATUS.md` - Overall progress (~85% complete)
- `/tasks/INFRASTRUCTURE_PHASE_STATUS.md` - Current development phase
- `/FUNCTIONALITY_REPORT.md` - Working features and known issues
- `/reference/ably-api-credentials.md` - API key for testing