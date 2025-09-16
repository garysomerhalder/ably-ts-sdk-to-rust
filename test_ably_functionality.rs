use ably_core::{
    client::rest::RestClient,
    auth::AuthMode,
    http::HttpConfig,
};
use std::env;

#[tokio::main]
async fn main() {
    println!("🔧 Testing Ably SDK Functionality\n");
    
    // Get API key
    let api_key = env::var("ABLY_API_KEY_SANDBOX")
        .unwrap_or_else(|_| "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA".to_string());
    
    println!("✅ Using API key: {}...", &api_key[..15]);
    
    // 1. Test basic connectivity
    println!("\n1️⃣ Testing REST Client Creation...");
    let client = match RestClient::new(
        HttpConfig::default(),
        AuthMode::ApiKey(api_key.clone())
    ) {
        Ok(c) => {
            println!("   ✅ REST client created successfully");
            c
        },
        Err(e) => {
            println!("   ❌ Failed to create REST client: {}", e);
            return;
        }
    };
    
    // 2. Test time endpoint
    println!("\n2️⃣ Testing Time Endpoint...");
    match client.time().await {
        Ok(time) => println!("   ✅ Server time: {} ms", time),
        Err(e) => println!("   ❌ Failed to get time: {}", e),
    }
    
    // 3. Test channel publish
    println!("\n3️⃣ Testing Channel Publish...");
    let channel_name = format!("test-channel-{}", chrono::Utc::now().timestamp());
    match client.publish(&channel_name, "test-event", "Hello from Rust SDK!").await {
        Ok(_) => println!("   ✅ Message published to channel: {}", channel_name),
        Err(e) => println!("   ❌ Failed to publish: {}", e),
    }
    
    // 4. Test channel history
    println!("\n4️⃣ Testing Channel History...");
    match client.get_channel_history(&channel_name, None).await {
        Ok(history) => {
            println!("   ✅ Retrieved {} messages from history", history.items.len());
            for msg in &history.items {
                if let Some(data) = &msg.data {
                    println!("      - Message: {}", data);
                }
            }
        },
        Err(e) => println!("   ❌ Failed to get history: {}", e),
    }
    
    // 5. Test stats
    println!("\n5️⃣ Testing Stats Endpoint...");
    match client.stats(None).await {
        Ok(stats) => println!("   ✅ Retrieved {} stat entries", stats.items.len()),
        Err(e) => println!("   ❌ Failed to get stats: {}", e),
    }
    
    // 6. Test channel list
    println!("\n6️⃣ Testing Channel List...");
    match client.list_channels(None).await {
        Ok(channels) => println!("   ✅ Found {} active channels", channels.len()),
        Err(e) => println!("   ❌ Failed to list channels: {}", e),
    }
    
    // 7. Test encrypted channel
    println!("\n7️⃣ Testing Encrypted Channel...");
    use ably_core::crypto::{CipherParams, Algorithm};
    use rand::Rng;
    
    let mut key = [0u8; 32];
    rand::thread_rng().fill(&mut key);
    let cipher_params = CipherParams::new(Algorithm::Aes256Cbc, key.to_vec(), None);
    
    let encrypted_channel = format!("encrypted-{}", chrono::Utc::now().timestamp());
    match client.publish_encrypted(&encrypted_channel, "secure", "Secret message", &cipher_params).await {
        Ok(_) => println!("   ✅ Encrypted message published"),
        Err(e) => println!("   ❌ Failed to publish encrypted: {}", e),
    }
    
    println!("\n✨ Functionality test complete!");
}