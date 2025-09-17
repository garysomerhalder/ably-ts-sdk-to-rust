// UltraThink: Deep debugging of WebSocket 400 error
// Testing multiple connection parameter combinations

use base64::Engine;

#[tokio::test]
async fn test_websocket_parameter_combinations() {
    println!("\n🔬 UltraThink: WebSocket Parameter Analysis\n");
    
    let api_key = "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA";
    
    // Parse API key components
    let parts: Vec<&str> = api_key.split('.').collect();
    let app_id = parts[0];
    let key_parts: Vec<&str> = parts[1].split(':').collect();
    let key_id = format!("{}.{}", app_id, key_parts[0]);
    let key_secret = key_parts[1];
    
    println!("📊 API Key Components:");
    println!("   App ID: {}", app_id);
    println!("   Key ID: {}", key_id);
    println!("   Key Secret: {} chars", key_secret.len());
    
    // Test 1: Try with just app ID
    println!("\n🧪 Test 1: URL with just app ID");
    test_url_format(&format!(
        "wss://realtime.ably.io?key={}&v=1.2&format=json",
        app_id
    )).await;
    
    // Test 2: Try with key ID only (no secret)
    println!("\n🧪 Test 2: URL with key ID only");
    test_url_format(&format!(
        "wss://realtime.ably.io?key={}&v=1.2&format=json",
        key_id
    )).await;
    
    // Test 3: Try with different parameter order
    println!("\n🧪 Test 3: Different parameter order");
    test_url_format(&format!(
        "wss://realtime.ably.io?format=json&v=1.2&key={}",
        api_key
    )).await;
    
    // Test 4: Try without version
    println!("\n🧪 Test 4: Without version parameter");
    test_url_format(&format!(
        "wss://realtime.ably.io?key={}&format=json",
        api_key
    )).await;
    
    // Test 5: Try with client library identification
    println!("\n🧪 Test 5: With library identification");
    test_url_format(&format!(
        "wss://realtime.ably.io?key={}&v=1.2&format=json&lib=rust-0.1.0",
        api_key
    )).await;
    
    // Test 6: Try with timestamp
    println!("\n🧪 Test 6: With timestamp");
    let timestamp = chrono::Utc::now().timestamp_millis();
    test_url_format(&format!(
        "wss://realtime.ably.io?key={}&v=1.2&format=json&timestamp={}",
        api_key, timestamp
    )).await;
}

async fn test_url_format(url: &str) {
    println!("   URL: {}", url);
    
    // Try direct connection with URL
    match tokio_tungstenite::connect_async(url).await {
        Ok((stream, response)) => {
            println!("   ✅ Connected! Response: {:?}", response.status());
            // Close the connection
            drop(stream);
        }
        Err(e) => {
            println!("   ❌ Failed: {}", e);
            
            // Try to extract more details from the error
            if let tokio_tungstenite::tungstenite::Error::Http(response) = &e {
                println!("      HTTP Status: {:?}", response.status());
                if let Some(body) = response.body() {
                    println!("      Body: {:?}", String::from_utf8_lossy(body));
                }
            }
        }
    }
}

#[tokio::test]
async fn test_websocket_with_basic_auth() {
    println!("\n🔬 Testing WebSocket with Basic Auth Header\n");
    
    let api_key = "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA";
    
    // Create request with Basic auth header
    use tokio_tungstenite::tungstenite::http::Request;
    use base64::Engine;
    
    let encoded_key = base64::engine::general_purpose::STANDARD.encode(api_key);
    let url = "wss://realtime.ably.io?v=1.2&format=json";
    
    println!("URL: {}", url);
    println!("Authorization: Basic {}", encoded_key);
    
    let request = Request::builder()
        .uri(url)
        .header("Authorization", format!("Basic {}", encoded_key))
        .header("Host", "realtime.ably.io")
        .header("Upgrade", "websocket")
        .header("Connection", "Upgrade")
        .header("Sec-WebSocket-Key", generate_ws_key())
        .header("Sec-WebSocket-Version", "13")
        .body(())
        .expect("Failed to build request");
    
    match tokio_tungstenite::connect_async(request).await {
        Ok((stream, response)) => {
            println!("✅ Connected with Basic Auth! Response: {:?}", response.status());
            drop(stream);
        }
        Err(e) => {
            println!("❌ Failed with Basic Auth: {}", e);
        }
    }
}

fn generate_ws_key() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 16] = rng.gen();
    base64::engine::general_purpose::STANDARD.encode(&bytes)
}

#[tokio::test] 
async fn test_websocket_subprotocols() {
    println!("\n🔬 Testing WebSocket with Ably subprotocols\n");
    
    let api_key = "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA";
    
    use tokio_tungstenite::tungstenite::http::Request;
    
    let url = format!("wss://realtime.ably.io?key={}&v=1.2&format=json", api_key);
    
    // Try with ably-protocol subprotocol
    let request = Request::builder()
        .uri(url)
        .header("Host", "realtime.ably.io")
        .header("Upgrade", "websocket")
        .header("Connection", "Upgrade")
        .header("Sec-WebSocket-Key", generate_ws_key())
        .header("Sec-WebSocket-Version", "13")
        .header("Sec-WebSocket-Protocol", "ably-v1.2")
        .body(())
        .expect("Failed to build request");
    
    match tokio_tungstenite::connect_async(request).await {
        Ok((stream, response)) => {
            println!("✅ Connected with ably-v1.2 protocol! Response: {:?}", response.status());
            drop(stream);
        }
        Err(e) => {
            println!("❌ Failed with ably-v1.2 protocol: {}", e);
        }
    }
}