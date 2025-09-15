# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This repository ports the Ably JavaScript/TypeScript SDK (v2.12.0) to Rust while maintaining 100% API compatibility. The project follows strict Traffic-Light Development methodology with Integration-First testing (no mocks).

## Development Commands

### Rust SDK (Main Project)

```bash
# Build commands
cargo build                          # Build all workspace crates
cargo build --release                # Build optimized release version
cargo build -p ably-core            # Build specific crate

# Testing commands
cargo test                           # Run all tests
cargo test -p ably-core             # Test specific crate
cargo test --test <test_name>       # Run specific test file
cargo test test_basic_get_request   # Run test by name pattern
cargo test -- --nocapture           # Show println! output during tests

# Documentation
cargo doc --open                     # Generate and view documentation
cargo doc --no-deps                  # Document only this project

# Code quality
cargo fmt                            # Format all code
cargo clippy                         # Run linter
cargo check                          # Quick compilation check
```

### JavaScript Reference SDK (ably-js/)

```bash
cd ably-js
npm install                          # Initial setup
npm run build                        # Build all targets
npm test                            # Run tests
npm run test:grep <pattern>         # Run specific tests
```

## Repository Structure

```
/root/repo/
├── ably-core/                      # Core SDK implementation
│   ├── src/
│   │   ├── auth/                   # Authentication module
│   │   ├── client/                 # REST and Realtime clients
│   │   ├── error/                  # Error handling with Ably codes
│   │   ├── http/                   # HTTP client with resilience
│   │   ├── logging/                # Structured logging/tracing
│   │   ├── protocol/               # Protocol messages (pending)
│   │   └── transport/              # WebSocket transport (pending)
│   └── tests/                      # Integration tests (real API only)
├── ably-node/                      # Node.js bindings (future)
├── ably-wasm/                      # WebAssembly bindings (future)
├── ably-ffi/                       # C FFI bindings (future)
├── ably-js/                        # Reference TypeScript SDK
└── tasks/                          # Task tracking and progress
```

## High-Level Architecture

### Current Implementation Status

#### ✅ Completed Components
- **HTTP Client** (`ably-core/src/http/`): Production-ready with circuit breaker, rate limiting, and metrics
- **Error System** (`ably-core/src/error/`): Ably protocol error codes (40000-50000 range)
- **Logging** (`ably-core/src/logging/`): Structured logging with OpenTelemetry readiness
- **Authentication** (`ably-core/src/auth/`): API key and token support

#### 🔴 In Progress
- **WebSocket Transport**: Tests written, implementation pending
- **Protocol Messages**: 22 action types to implement
- **Connection State Machine**: Full state transitions needed

### Critical Implementation Requirements

1. **Protocol Compatibility**
   - Must implement Ably protocol v3
   - Support both MessagePack and JSON encoding
   - Maintain exact state machine transitions from JS SDK

2. **Testing Philosophy**
   - ALL tests must use real Ably sandbox API
   - Never use mocks or fakes (Integration-First)
   - Test API key: `BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA`

3. **Traffic-Light Development**
   - 🔴 RED: Write failing tests against real API
   - 🟡 YELLOW: Minimal implementation to pass
   - 🟢 GREEN: Add resilience and production features
   - Commit after EACH phase

### State Machines

#### Connection States
```
initialized → connecting → connected → disconnected → suspended → closing → closed → failed
```
- Auto-reconnect with exponential backoff
- Suspend after prolonged disconnection
- Failed requires manual intervention

#### Channel States
```
initialized → attaching → attached → detaching → detached → suspended → failed
```
- Operations queue when not attached
- Automatic reattachment on reconnection

## Key Reference Files

When implementing new features, study these JavaScript SDK files:

1. **Auth System**: `ably-js/src/common/lib/client/auth.ts` (42KB)
2. **Connection Manager**: `ably-js/src/common/lib/transport/connectionmanager.ts` (73KB)
3. **WebSocket Transport**: `ably-js/src/common/lib/transport/websockettransport.ts` (8.3KB)
4. **Protocol Messages**: `ably-js/src/common/lib/types/protocolmessagecommon.ts`
5. **Realtime Channel**: `ably-js/src/common/lib/client/realtimechannel.ts` (34KB)

## Task Management

Tasks are tracked in `/tasks/` directory with current status in:
- `/tasks/INFRASTRUCTURE_PHASE_STATUS.md` - Current phase progress
- `/tasks/task-files/` - Individual task specifications

Current Progress:
- Foundation Phase: ✅ Complete (5/5 tasks)
- Infrastructure Phase: 🟡 In Progress (1/5 tasks)
- Overall: ~15% complete

## Testing Guidelines

1. **Environment Setup**
   ```bash
   export ABLY_API_KEY_SANDBOX="BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA"
   ```

2. **Test Organization**
   - Unit tests: In `src/` modules with `#[cfg(test)]`
   - Integration tests: In `tests/` directory
   - All tests must connect to real Ably sandbox

3. **Test Patterns**
   ```rust
   // Always use real endpoints
   let response = client.get("https://sandbox-rest.ably.io/time").await;
   
   // Never mock
   // ❌ let mock_server = MockServer::start();
   // ✅ let real_api = "https://sandbox-rest.ably.io";
   ```

## Performance Targets

Based on JavaScript SDK benchmarks:
- Connection establishment: <200ms
- Message latency: <50ms regional
- Throughput: >10,000 messages/second
- WASM bundle: <200KB gzipped

## Workspace Dependencies

Key dependencies are managed at workspace level in root `Cargo.toml`:
```toml
tokio = "1.40"                      # Async runtime
reqwest = "0.12"                    # HTTP client
tokio-tungstenite = "0.24"          # WebSocket
serde = "1.0"                       # Serialization
rmp-serde = "1.3"                   # MessagePack
thiserror = "2.0"                   # Error handling
tracing = "0.1"                     # Logging
```

## Protocol Action Types

The SDK must implement these 22 protocol actions:
- **Connection**: CONNECT, CONNECTED, DISCONNECT, DISCONNECTED, CLOSE, CLOSED
- **Channel**: ATTACH, ATTACHED, DETACH, DETACHED  
- **Messaging**: MESSAGE, PRESENCE, SYNC
- **Control**: HEARTBEAT, ACK, NACK, ERROR, AUTH, ACTIVATE
- **Objects**: OBJECT, OBJECT_SYNC, ANNOTATION

## Common Development Patterns

### Adding a New Module
1. Create tests first (RED phase) in `tests/`
2. Implement minimal code (YELLOW phase) in `src/`
3. Add resilience features (GREEN phase)
4. Commit after each phase

### Error Handling
```rust
use ably_core::error::{AblyError, AblyResult};

// Use Ably-specific error codes
AblyError::from_ably_code(40400, "Not found")
```

### Real API Integration
```rust
// Always use real endpoints
let client = AblyHttpClient::with_auth(
    HttpConfig::default(),
    AuthMode::ApiKey(api_key)
);
```