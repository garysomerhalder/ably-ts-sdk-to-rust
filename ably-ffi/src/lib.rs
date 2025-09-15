//! C FFI bindings for Ably Rust SDK

pub fn ffi_version() -> &'static str {
    ably_core::version()
}