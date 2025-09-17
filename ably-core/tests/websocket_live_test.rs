use ably_core::transport::websocket::WebSocketTransport;
use ably_core::protocol::{ProtocolMessage, Action};

#[tokio::test]
async fn test_websocket_connection_to_ably() {
    println!("\n🔧 Testing WebSocket Connection to Ably\n");
    
    let api_key = "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA";
    
    println!("1️⃣ Creating WebSocket transport...");
    let mut transport = WebSocketTransport::with_api_key(api_key);
    
    println!("2️⃣ Connecting to Ably realtime...");
    match transport.connect().await {
        Ok(_) => println!("   ✅ Connected successfully!"),
        Err(e) => {
            println!("   ❌ Connection failed: {}", e);
            panic!("WebSocket connection should work with valid API key");
        }
    }
    
    println!("3️⃣ Waiting for CONNECTED message...");
    match transport.receive().await {
        Ok(Some(msg)) => {
            println!("   ✅ Received message with action: {:?}", msg.action);
            assert_eq!(msg.action, Action::Connected, "First message should be CONNECTED");
            
            if let Some(details) = msg.connection_details {
                println!("      Connection key: {:?}", details.connection_key);
                println!("      Max message size: {:?}", details.max_message_size);
            }
        },
        Ok(None) => panic!("Connection closed unexpectedly"),
        Err(e) => panic!("Failed to receive message: {}", e),
    }
    
    println!("4️⃣ Sending HEARTBEAT...");
    let heartbeat_msg = ProtocolMessage {
        action: Action::Heartbeat,
        ..Default::default()
    };
    
    match transport.send(heartbeat_msg).await {
        Ok(_) => println!("   ✅ HEARTBEAT sent"),
        Err(e) => println!("   ❌ Failed to send heartbeat: {}", e),
    }
    
    println!("5️⃣ Waiting for HEARTBEAT ACK...");
    match transport.receive().await {
        Ok(Some(msg)) => {
            if msg.action == Action::Heartbeat {
                println!("   ✅ Received HEARTBEAT ACK");
            } else {
                println!("   ℹ️ Received: {:?}", msg.action);
            }
        },
        Ok(None) => println!("   ⚠️ Connection closed"),
        Err(e) => println!("   ❌ Failed to receive: {}", e),
    }
    
    println!("\n✨ WebSocket connection test complete!");
}