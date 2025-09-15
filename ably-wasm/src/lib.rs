//! WebAssembly bindings for Ably Rust SDK

pub fn wasm_version() -> &'static str {
    ably_core::version()
}