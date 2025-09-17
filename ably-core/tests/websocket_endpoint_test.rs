// Testing different WebSocket endpoints and paths
use base64::Engine;

#[tokio::test]
async fn test_different_websocket_endpoints() {
    println!("\n🔬 Testing Different WebSocket Endpoints\n");
    
    let api_key = "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA";
    
    // Test different possible endpoints
    let endpoints = vec![
        "wss://realtime.ably.io",
        "wss://realtime.ably.io/",
        "wss://realtime.ably.io/ws",
        "wss://realtime.ably.io/websocket",
        "wss://realtime.ably.io/socket",
        "wss://realtime.ably.io/connect",
        "wss://realtime.ably.io/realtime",
        "wss://ws.ably.io",
        "wss://websocket.ably.io",
        "wss://stream.ably.io",
    ];
    
    for endpoint in endpoints {
        println!("Testing endpoint: {}", endpoint);
        let url = format!("{}?key={}&v=1.2&format=json", endpoint, api_key);
        
        match tokio_tungstenite::connect_async(&url).await {
            Ok((stream, response)) => {
                println!("  ✅ Connected to {}! Status: {:?}", endpoint, response.status());
                drop(stream);
            }
            Err(e) => {
                println!("  ❌ Failed: {}", e);
            }
        }
    }
}

#[tokio::test]
async fn test_websocket_with_environment_subdomain() {
    println!("\n🔬 Testing WebSocket with environment-specific subdomains\n");
    
    let api_key = "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA";
    let app_id = "BGkZHw";
    
    // Try environment-specific subdomains
    let subdomains = vec![
        format!("{}.ably.io", app_id.to_lowercase()),
        format!("{}-realtime.ably.io", app_id.to_lowercase()),
        "realtime-staging.ably.io".to_string(),
        "realtime-production.ably.io".to_string(),
        "us-east-1-a.ably.io".to_string(),
        "us-west-1-a.ably.io".to_string(),
    ];
    
    for subdomain in subdomains {
        println!("Testing subdomain: {}", subdomain);
        let url = format!("wss://{}?key={}&v=1.2&format=json", subdomain, api_key);
        
        match tokio_tungstenite::connect_async(&url).await {
            Ok((stream, response)) => {
                println!("  ✅ Connected to {}! Status: {:?}", subdomain, response.status());
                drop(stream);
            }
            Err(e) => {
                println!("  ❌ Failed: {}", e);
            }
        }
    }
}

#[tokio::test]
async fn test_websocket_with_origin_header() {
    println!("\n🔬 Testing WebSocket with Origin header\n");
    
    let api_key = "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA";
    use tokio_tungstenite::tungstenite::http::Request;
    
    let url = format!("wss://realtime.ably.io?key={}&v=1.2&format=json", api_key);
    
    // Try with Origin header (sometimes required for CORS)
    let request = Request::builder()
        .uri(url)
        .header("Origin", "https://ably.io")
        .header("Host", "realtime.ably.io")
        .header("Upgrade", "websocket")
        .header("Connection", "Upgrade")
        .header("Sec-WebSocket-Key", generate_ws_key())
        .header("Sec-WebSocket-Version", "13")
        .header("User-Agent", "ably-rust/0.1.0")
        .body(())
        .expect("Failed to build request");
    
    match tokio_tungstenite::connect_async(request).await {
        Ok((stream, response)) => {
            println!("✅ Connected with Origin header! Status: {:?}", response.status());
            drop(stream);
        }
        Err(e) => {
            println!("❌ Failed with Origin header: {}", e);
        }
    }
}

fn generate_ws_key() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 16] = rng.gen();
    base64::engine::general_purpose::STANDARD.encode(&bytes)
}