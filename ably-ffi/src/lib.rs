// 🔴 RED Phase: C FFI bindings for Ably SDK
// Provides C-compatible interface for use in C/C++ applications

use std::os::raw::{c_char, c_int, c_void};
use std::ffi::{CStr, CString};
use std::ptr;
use std::slice;
use std::sync::Arc;
use std::sync::Mutex;

use ably_core::client::rest::RestClient as CoreRestClient;
use ably_core::client::realtime::RealtimeClient as CoreRealtimeClient;
use ably_core::protocol::messages::{Message, PresenceMessage};
use ably_core::error::{AblyError, ErrorCode};
use serde_json;
use tokio::runtime::Runtime;

/// Opaque handle for REST client
pub struct AblyRestClient {
    client: Arc<CoreRestClient>,
    runtime: Arc<Runtime>,
}

/// Opaque handle for Realtime client
pub struct AblyRealtimeClient {
    client: Arc<Mutex<CoreRealtimeClient>>,
    runtime: Arc<Runtime>,
}

/// Opaque handle for REST channel
pub struct AblyRestChannel {
    client: Arc<CoreRestClient>,
    channel_name: String,
    runtime: Arc<Runtime>,
}

/// Opaque handle for Realtime channel
pub struct AblyRealtimeChannel {
    client: Arc<Mutex<CoreRealtimeClient>>,
    channel_name: String,
    runtime: Arc<Runtime>,
}

/// Error structure for C FFI
#[repr(C)]
pub struct AblyError_FFI {
    pub code: c_int,
    pub message: *mut c_char,
}

/// Result type for FFI functions
#[repr(C)]
pub struct AblyResult {
    pub success: c_int,  // 1 for success, 0 for failure
    pub error: *mut AblyError_FFI,
}

// Helper function to create C string
fn to_c_string(s: &str) -> *mut c_char {
    match CString::new(s) {
        Ok(cs) => cs.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

// Helper function to convert C string to Rust string
unsafe fn from_c_string(s: *const c_char) -> Result<String, String> {
    if s.is_null() {
        return Err("Null pointer".to_string());
    }
    
    match CStr::from_ptr(s).to_str() {
        Ok(str) => Ok(str.to_string()),
        Err(_) => Err("Invalid UTF-8".to_string()),
    }
}

// Helper function to create error
fn create_error(err: AblyError) -> *mut AblyError_FFI {
    let code = match err.code() {
        Some(ErrorCode::Custom(code)) => code as c_int,
        Some(ErrorCode::Unauthorized) => 401,
        Some(ErrorCode::Forbidden) => 403,
        Some(ErrorCode::NotFound) => 404,
        Some(ErrorCode::RateLimit) => 429,
        Some(ErrorCode::Internal) => 500,
        None => 50000,
    };
    
    let error = Box::new(AblyError_FFI {
        code,
        message: to_c_string(&err.to_string()),
    });
    
    Box::into_raw(error)
}

/// Get SDK version
#[no_mangle]
pub extern "C" fn ably_version() -> *const c_char {
    static VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");
    VERSION.as_ptr() as *const c_char
}

/// Create a new REST client
#[no_mangle]
pub extern "C" fn ably_rest_client_new(api_key: *const c_char) -> *mut AblyRestClient {
    unsafe {
        match from_c_string(api_key) {
            Ok(key) => {
                let runtime = Runtime::new().expect("Failed to create runtime");
                let client = CoreRestClient::new(&key);
                
                let handle = Box::new(AblyRestClient {
                    client: Arc::new(client),
                    runtime: Arc::new(runtime),
                });
                
                Box::into_raw(handle)
            }
            Err(_) => ptr::null_mut(),
        }
    }
}

/// Free REST client
#[no_mangle]
pub extern "C" fn ably_rest_client_free(client: *mut AblyRestClient) {
    if !client.is_null() {
        unsafe {
            let _ = Box::from_raw(client);
        }
    }
}

/// Get a REST channel
#[no_mangle]
pub extern "C" fn ably_rest_client_channel(
    client: *mut AblyRestClient,
    name: *const c_char,
) -> *mut AblyRestChannel {
    if client.is_null() {
        return ptr::null_mut();
    }
    
    unsafe {
        let client = &*client;
        match from_c_string(name) {
            Ok(channel_name) => {
                let handle = Box::new(AblyRestChannel {
                    client: Arc::clone(&client.client),
                    channel_name,
                    runtime: Arc::clone(&client.runtime),
                });
                
                Box::into_raw(handle)
            }
            Err(_) => ptr::null_mut(),
        }
    }
}

/// Free REST channel
#[no_mangle]
pub extern "C" fn ably_rest_channel_free(channel: *mut AblyRestChannel) {
    if !channel.is_null() {
        unsafe {
            let _ = Box::from_raw(channel);
        }
    }
}

/// Publish a message to REST channel
#[no_mangle]
pub extern "C" fn ably_rest_channel_publish(
    channel: *mut AblyRestChannel,
    name: *const c_char,
    data: *const c_char,
) -> AblyResult {
    if channel.is_null() {
        return AblyResult {
            success: 0,
            error: create_error(AblyError::from_ably_code(50000, "Null channel")),
        };
    }
    
    unsafe {
        let channel = &*channel;
        
        let message_name = if name.is_null() {
            None
        } else {
            match from_c_string(name) {
                Ok(n) => Some(n),
                Err(_) => None,
            }
        };
        
        let message_data = if data.is_null() {
            None
        } else {
            match from_c_string(data) {
                Ok(d) => serde_json::from_str(&d).ok(),
                Err(_) => None,
            }
        };
        
        let message = Message {
            name: message_name,
            data: message_data,
            ..Default::default()
        };
        
        let rest_channel = channel.client.channel(&channel.channel_name);
        
        let result = channel.runtime.block_on(async {
            rest_channel.publish(message).await
        });
        
        match result {
            Ok(_) => AblyResult {
                success: 1,
                error: ptr::null_mut(),
            },
            Err(e) => AblyResult {
                success: 0,
                error: create_error(e),
            },
        }
    }
}

/// Get message history from REST channel
#[no_mangle]
pub extern "C" fn ably_rest_channel_history(
    channel: *mut AblyRestChannel,
    limit: c_int,
    out_json: *mut *mut c_char,
) -> AblyResult {
    if channel.is_null() || out_json.is_null() {
        return AblyResult {
            success: 0,
            error: create_error(AblyError::from_ably_code(50000, "Invalid parameters")),
        };
    }
    
    unsafe {
        let channel = &*channel;
        let rest_channel = channel.client.channel(&channel.channel_name);
        
        let result = channel.runtime.block_on(async {
            let mut query = rest_channel.history();
            if limit > 0 {
                query = query.limit(limit as u32);
            }
            query.execute().await
        });
        
        match result {
            Ok(history) => {
                match serde_json::to_string(&history.items) {
                    Ok(json) => {
                        *out_json = to_c_string(&json);
                        AblyResult {
                            success: 1,
                            error: ptr::null_mut(),
                        }
                    }
                    Err(_) => AblyResult {
                        success: 0,
                        error: create_error(AblyError::from_ably_code(50000, "Serialization failed")),
                    }
                }
            }
            Err(e) => AblyResult {
                success: 0,
                error: create_error(e),
            },
        }
    }
}

/// Create a new Realtime client
#[no_mangle]
pub extern "C" fn ably_realtime_client_new(api_key: *const c_char) -> *mut AblyRealtimeClient {
    unsafe {
        match from_c_string(api_key) {
            Ok(key) => {
                let runtime = Runtime::new().expect("Failed to create runtime");
                
                let client = runtime.block_on(async {
                    CoreRealtimeClient::new(&key).await
                });
                
                match client {
                    Ok(c) => {
                        let handle = Box::new(AblyRealtimeClient {
                            client: Arc::new(Mutex::new(c)),
                            runtime: Arc::new(runtime),
                        });
                        
                        Box::into_raw(handle)
                    }
                    Err(_) => ptr::null_mut(),
                }
            }
            Err(_) => ptr::null_mut(),
        }
    }
}

/// Free Realtime client
#[no_mangle]
pub extern "C" fn ably_realtime_client_free(client: *mut AblyRealtimeClient) {
    if !client.is_null() {
        unsafe {
            let _ = Box::from_raw(client);
        }
    }
}

/// Connect Realtime client
#[no_mangle]
pub extern "C" fn ably_realtime_client_connect(client: *mut AblyRealtimeClient) -> AblyResult {
    if client.is_null() {
        return AblyResult {
            success: 0,
            error: create_error(AblyError::from_ably_code(50000, "Null client")),
        };
    }
    
    unsafe {
        let client = &*client;
        
        let result = client.runtime.block_on(async {
            let mut c = client.client.lock().unwrap();
            c.connect().await
        });
        
        match result {
            Ok(_) => AblyResult {
                success: 1,
                error: ptr::null_mut(),
            },
            Err(e) => AblyResult {
                success: 0,
                error: create_error(e),
            },
        }
    }
}

/// Disconnect Realtime client
#[no_mangle]
pub extern "C" fn ably_realtime_client_disconnect(client: *mut AblyRealtimeClient) -> AblyResult {
    if client.is_null() {
        return AblyResult {
            success: 0,
            error: create_error(AblyError::from_ably_code(50000, "Null client")),
        };
    }
    
    unsafe {
        let client = &*client;
        
        let result = client.runtime.block_on(async {
            let mut c = client.client.lock().unwrap();
            c.disconnect().await
        });
        
        match result {
            Ok(_) => AblyResult {
                success: 1,
                error: ptr::null_mut(),
            },
            Err(e) => AblyResult {
                success: 0,
                error: create_error(e),
            },
        }
    }
}

/// Get a Realtime channel
#[no_mangle]
pub extern "C" fn ably_realtime_client_channel(
    client: *mut AblyRealtimeClient,
    name: *const c_char,
) -> *mut AblyRealtimeChannel {
    if client.is_null() {
        return ptr::null_mut();
    }
    
    unsafe {
        let client = &*client;
        match from_c_string(name) {
            Ok(channel_name) => {
                let handle = Box::new(AblyRealtimeChannel {
                    client: Arc::clone(&client.client),
                    channel_name,
                    runtime: Arc::clone(&client.runtime),
                });
                
                Box::into_raw(handle)
            }
            Err(_) => ptr::null_mut(),
        }
    }
}

/// Free Realtime channel
#[no_mangle]
pub extern "C" fn ably_realtime_channel_free(channel: *mut AblyRealtimeChannel) {
    if !channel.is_null() {
        unsafe {
            let _ = Box::from_raw(channel);
        }
    }
}

/// Attach to Realtime channel
#[no_mangle]
pub extern "C" fn ably_realtime_channel_attach(channel: *mut AblyRealtimeChannel) -> AblyResult {
    if channel.is_null() {
        return AblyResult {
            success: 0,
            error: create_error(AblyError::from_ably_code(50000, "Null channel")),
        };
    }
    
    unsafe {
        let channel = &*channel;
        
        let result = channel.runtime.block_on(async {
            let mut client = channel.client.lock().unwrap();
            let ch = client.channel(&channel.channel_name).await;
            ch.attach().await
        });
        
        match result {
            Ok(_) => AblyResult {
                success: 1,
                error: ptr::null_mut(),
            },
            Err(e) => AblyResult {
                success: 0,
                error: create_error(e),
            },
        }
    }
}

/// Detach from Realtime channel
#[no_mangle]
pub extern "C" fn ably_realtime_channel_detach(channel: *mut AblyRealtimeChannel) -> AblyResult {
    if channel.is_null() {
        return AblyResult {
            success: 0,
            error: create_error(AblyError::from_ably_code(50000, "Null channel")),
        };
    }
    
    unsafe {
        let channel = &*channel;
        
        let result = channel.runtime.block_on(async {
            let mut client = channel.client.lock().unwrap();
            let ch = client.channel(&channel.channel_name).await;
            ch.detach().await
        });
        
        match result {
            Ok(_) => AblyResult {
                success: 1,
                error: ptr::null_mut(),
            },
            Err(e) => AblyResult {
                success: 0,
                error: create_error(e),
            },
        }
    }
}

/// Publish a message to Realtime channel
#[no_mangle]
pub extern "C" fn ably_realtime_channel_publish(
    channel: *mut AblyRealtimeChannel,
    name: *const c_char,
    data: *const c_char,
) -> AblyResult {
    if channel.is_null() {
        return AblyResult {
            success: 0,
            error: create_error(AblyError::from_ably_code(50000, "Null channel")),
        };
    }
    
    unsafe {
        let channel = &*channel;
        
        let message_name = if name.is_null() {
            None
        } else {
            match from_c_string(name) {
                Ok(n) => Some(n),
                Err(_) => None,
            }
        };
        
        let message_data = if data.is_null() {
            None
        } else {
            match from_c_string(data) {
                Ok(d) => serde_json::from_str(&d).ok(),
                Err(_) => None,
            }
        };
        
        let message = Message {
            name: message_name,
            data: message_data,
            ..Default::default()
        };
        
        let result = channel.runtime.block_on(async {
            let mut client = channel.client.lock().unwrap();
            let ch = client.channel(&channel.channel_name).await;
            ch.publish(message).await
        });
        
        match result {
            Ok(_) => AblyResult {
                success: 1,
                error: ptr::null_mut(),
            },
            Err(e) => AblyResult {
                success: 0,
                error: create_error(e),
            },
        }
    }
}

/// Free an error structure
#[no_mangle]
pub extern "C" fn ably_error_free(error: *mut AblyError_FFI) {
    if !error.is_null() {
        unsafe {
            let error = Box::from_raw(error);
            if !error.message.is_null() {
                let _ = CString::from_raw(error.message);
            }
        }
    }
}

/// Free a C string
#[no_mangle]
pub extern "C" fn ably_string_free(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

// Crypto utilities

/// Generate random key
#[no_mangle]
pub extern "C" fn ably_crypto_generate_random_key(
    bits: c_int,
    out_key: *mut u8,
    out_len: *mut c_int,
) -> AblyResult {
    use ably_core::crypto::{generate_random_key, CipherAlgorithm};
    
    if out_key.is_null() || out_len.is_null() {
        return AblyResult {
            success: 0,
            error: create_error(AblyError::from_ably_code(50000, "Invalid parameters")),
        };
    }
    
    let algorithm = if bits == 128 {
        CipherAlgorithm::Aes128
    } else if bits == 256 {
        CipherAlgorithm::Aes256
    } else {
        return AblyResult {
            success: 0,
            error: create_error(AblyError::from_ably_code(50000, "Key size must be 128 or 256 bits")),
        };
    };
    
    let key = generate_random_key(algorithm);
    let key_len = key.len() as c_int;
    
    unsafe {
        ptr::copy_nonoverlapping(key.as_ptr(), out_key, key.len());
        *out_len = key_len;
    }
    
    AblyResult {
        success: 1,
        error: ptr::null_mut(),
    }
}

// Tests for FFI bindings
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_version() {
        let version = ably_version();
        assert!(!version.is_null());
    }
    
    #[test]
    fn test_rest_client_creation() {
        let api_key = CString::new("test_key").unwrap();
        let client = ably_rest_client_new(api_key.as_ptr());
        assert!(!client.is_null());
        ably_rest_client_free(client);
    }
}