use ably_core::transport::websocket::WebSocketTransport;
use ably_core::protocol::{ProtocolMessage, Action};

#[tokio::main]
async fn main() {
    println!("🔧 Testing WebSocket Connection\n");
    
    let api_key = "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA";
    
    println!("1️⃣ Creating WebSocket transport...");
    let mut transport = WebSocketTransport::new(api_key);
    
    println!("2️⃣ Connecting to Ably realtime...");
    match transport.connect().await {
        Ok(_) => println!("   ✅ Connected successfully!"),
        Err(e) => {
            println!("   ❌ Connection failed: {}", e);
            return;
        }
    }
    
    println!("3️⃣ Waiting for CONNECTED message...");
    match transport.receive().await {
        Ok(Some(msg)) => {
            println!("   ✅ Received message: {:?}", msg.action);
            if msg.action == Action::Connected {
                println!("   ✅ Successfully connected to Ably!");
                if let Some(details) = msg.connection_details {
                    println!("      Connection ID: {:?}", details.connection_id);
                }
            }
        },
        Ok(None) => println!("   ⚠️ Connection closed"),
        Err(e) => println!("   ❌ Failed to receive: {}", e),
    }
    
    println!("4️⃣ Sending ATTACH for test channel...");
    let attach_msg = ProtocolMessage {
        action: Action::Attach,
        channel: Some("test-channel".to_string()),
        ..Default::default()
    };
    
    match transport.send(attach_msg).await {
        Ok(_) => println!("   ✅ ATTACH sent"),
        Err(e) => println!("   ❌ Failed to send: {}", e),
    }
    
    println!("5️⃣ Waiting for ATTACHED response...");
    match transport.receive().await {
        Ok(Some(msg)) => {
            if msg.action == Action::Attached {
                println!("   ✅ Channel attached successfully!");
            } else {
                println!("   ℹ️ Received: {:?}", msg.action);
            }
        },
        Ok(None) => println!("   ⚠️ Connection closed"),
        Err(e) => println!("   ❌ Failed to receive: {}", e),
    }
    
    println!("\n✨ WebSocket test complete!");
}