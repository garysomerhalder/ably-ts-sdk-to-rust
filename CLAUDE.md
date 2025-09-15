# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This repository is for porting the Ably JavaScript/TypeScript SDK (v2.12.0) to Rust while maintaining 100% API compatibility. The `ably-js/` directory contains the reference TypeScript implementation with 164 TypeScript source files.

## Repository Structure

```
/root/repo/
├── ably-js/                 # Reference TypeScript SDK
│   ├── src/
│   │   ├── common/lib/      # Core library (client, transport, types, util)
│   │   ├── platform/        # Platform-specific code (nodejs, web, react-native, nativescript)
│   │   └── plugins/         # Plugin system (push, objects)
│   ├── test/                # Test suites
│   │   ├── realtime/        # Realtime client tests
│   │   ├── rest/            # REST client tests
│   │   ├── browser/         # Browser-specific tests
│   │   └── unit/            # Unit tests
│   └── build/               # Build outputs
└── [future directories for Rust implementation]
```

## Development Commands

### Ably JavaScript SDK (Reference Implementation)

```bash
# Setup and build
cd ably-js
npm install
npm run build                # Build all targets
npm run build:node           # Node.js build only
npm run build:browser        # Browser build only
npm run build:react          # React hooks build

# Testing
npm test                     # Run Node.js tests (realtime, rest, unit)
npm run test:playwright      # Run browser tests via Playwright
npm run test:react           # Run React tests with Vitest
npm run test:grep <pattern>  # Run specific tests by pattern

# Code quality
npm run lint                 # ESLint
npm run lint:fix             # Auto-fix linting issues
npm run format               # Prettier formatting
npm run format:check         # Check formatting

# Development
npm run grunt -- test:webserver  # Start test webserver
npm run sourcemap            # Analyze bundle with source-map-explorer
npm run modulereport         # Generate module size report
```

### Future Rust SDK Commands

```bash
# Build
cargo build --release
cargo build --features "push objects"  # With optional features

# Testing
cargo test                   # All tests
cargo test --test integration -- --test-threads=1  # Integration tests serially
cargo test <test_name>       # Single test

# Documentation
cargo doc --open             # Generate and view docs
```

## High-Level Architecture

### Core Components (from ably-js)

#### 1. Client Architecture (`src/common/lib/client/`)
- **BaseClient** (7.5KB): Core functionality shared between REST and Realtime
- **Auth** (42KB): Handles API key auth, JWT tokens, token renewal, and client ID validation
- **BaseRealtime** (8.9KB): Extends BaseClient with realtime capabilities
- **DefaultRealtime** (3.3KB): Full-featured client with all plugins

#### 2. Transport Layer (`src/common/lib/transport/`)
- **ConnectionManager** (73KB): Manages connection lifecycle, state machine, reconnection logic
- **WebSocketTransport** (8.3KB): Primary realtime transport
- **CometTransport** (13.8KB): HTTP streaming fallback
- **Protocol** (3.4KB): Wire protocol handling, message queuing
- **MessageQueue** (2.4KB): Handles message acknowledgments and retries

#### 3. Protocol Messages (`src/common/lib/types/`)
The SDK implements 22 protocol action types:
- Connection: CONNECT, CONNECTED, DISCONNECT, DISCONNECTED, CLOSE, CLOSED
- Channel: ATTACH, ATTACHED, DETACH, DETACHED
- Messaging: MESSAGE, PRESENCE, SYNC
- Control: HEARTBEAT, ACK, NACK, ERROR, AUTH, ACTIVATE
- Objects: OBJECT, OBJECT_SYNC, ANNOTATION

Channel flags control capabilities like PRESENCE, PUBLISH, SUBSCRIBE, and OBJECT operations.

#### 4. Channel System (`src/common/lib/client/`)
- **RealtimeChannel** (34KB): Core channel functionality with state machine
- **RealtimePresence** (15KB): Presence set synchronization
- **FilteredSubscriptions** (4.1KB): Message filtering support

#### 5. Platform Abstraction
The SDK abstracts platform differences through:
- Separate implementations for Node.js, Web, React Native, NativeScript
- Platform-specific defaults, crypto, and transport selection
- Modular build system producing platform-specific bundles

### Connection State Machine

States: initialized → connecting → connected → disconnected → suspended → closing → closed → failed

Key transitions:
- Auto-reconnect with exponential backoff on disconnect
- Suspend after prolonged disconnection
- Failed state requires manual intervention

### Channel State Machine

States: initialized → attaching → attached → detaching → detached → suspended → failed

Operations are queued when channel is not attached.

## Rust Port Implementation Strategy

### Phase Structure

1. **Core Infrastructure**: HTTP client, auth, error handling
2. **Protocol Layer**: ProtocolMessage types, WebSocket transport, state machines
3. **Client Implementation**: REST client, Realtime client, channels
4. **Feature Parity**: Presence, push, encryption, plugins
5. **Bindings**: Node.js (napi-rs), WASM, C FFI

### Critical Implementation Requirements

- **Protocol Version**: Must implement Ably protocol v3
- **Encoding**: Support both MessagePack (preferred) and JSON
- **Error Codes**: Use Ably error codes (40000-50000 range)
- **Retry Logic**: Exponential backoff with jitter
- **State Machines**: Must match JavaScript SDK state transitions exactly
- **Testing**: Integration tests against real Ably sandbox (no mocks)

### Key Files to Study

1. **Authentication**: `ably-js/src/common/lib/client/auth.ts`
2. **Connection Management**: `ably-js/src/common/lib/transport/connectionmanager.ts`
3. **Channel Operations**: `ably-js/src/common/lib/client/realtimechannel.ts`
4. **Protocol Definition**: `ably-js/src/common/lib/types/protocolmessagecommon.ts`
5. **Transport Layer**: `ably-js/src/common/lib/transport/websockettransport.ts`

## Testing Approach

The JavaScript SDK has comprehensive tests in:
- `test/realtime/`: Realtime client tests
- `test/rest/`: REST API tests
- `test/browser/`: Browser-specific tests
- `test/unit/`: Unit tests

Tests use real Ably sandbox environments. The Rust port should:
1. Port test cases maintaining the same structure
2. Use integration tests against real Ably services
3. Never use mocks (Integration-First approach)
4. Match test coverage of JavaScript SDK

## Performance Targets

Based on JavaScript SDK capabilities:
- Connection establishment: <200ms
- Message latency: <50ms regional
- Throughput: >10,000 messages/second per connection
- Binary size: WASM bundle <200KB gzipped

## Recommended Rust Dependencies

```toml
# Core
tokio = "1.40"              # Async runtime
reqwest = "0.12"            # HTTP client
tokio-tungstenite = "0.24"  # WebSocket client

# Serialization
serde = "1.0"
serde_json = "1.0"
rmp-serde = "1.3"           # MessagePack

# Utilities
thiserror = "2.0"           # Error handling
tracing = "0.1"             # Logging
dashmap = "6.0"             # Concurrent collections
base64 = "0.22"

# Crypto
ring = "0.17"               # Cryptography
aes-gcm = "0.10"            # AES encryption

# Bindings
napi = "3.0"                # Node.js bindings
wasm-bindgen = "0.2"        # WebAssembly
```