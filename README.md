# Ably Rust SDK

A Rust implementation of the Ably Realtime SDK, providing 100% API compatibility with the JavaScript/TypeScript SDK.

## Project Status

🚧 **Under Development** - Porting from ably-js v2.12.0

## Project Structure

```
.
├── ably-core/       # Core SDK functionality
├── ably-node/       # Node.js bindings (via napi-rs)
├── ably-wasm/       # WebAssembly bindings
├── ably-ffi/        # C FFI bindings
├── ably-js/         # Reference TypeScript implementation
└── tasks/           # Task tracking and management
```

## Development

### Prerequisites

- Rust 1.75+ (latest stable)
- Node.js 18+ (for reference SDK and bindings)
- pkg-config and libssl-dev (for OpenSSL)

### Building

```bash
# Build all crates
cargo build

# Build release version
cargo build --release

# Run tests
cargo test

# Generate documentation
cargo doc --open
```

### Traffic-Light Development

This project follows Traffic-Light Development methodology:

1. 🔴 **RED**: Write failing tests against real Ably API
2. 🟡 **YELLOW**: Minimal implementation to pass tests
3. 🟢 **GREEN**: Production hardening with full features

## Architecture

The SDK is structured as a Rust workspace with multiple crates:

- **ably-core**: Core functionality including REST and Realtime clients
- **ably-node**: Node.js bindings using napi-rs
- **ably-wasm**: WebAssembly compilation for browser usage
- **ably-ffi**: C FFI for integration with other languages

## Testing

All tests run against real Ably sandbox environments. No mocks or fakes are used (Integration-First approach).

## License

Apache 2.0 - See LICENSE file for details