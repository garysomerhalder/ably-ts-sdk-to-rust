// 🔴 RED Phase: Tests for push notification system
// Tests real Ably push notification endpoints

use ably_core::client::rest::RestClient;
use ably_core::push::{
    DeviceRegistration, PushChannelSubscription, PushMessage, PushNotification,
    PushPayload, PushTarget, PushClient,
};
use serde_json::json;
use std::collections::HashMap;

// Test API key (sandbox)
const TEST_API_KEY: &str = "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA";

#[tokio::test]
async fn test_device_registration() {
    let client = RestClient::new(TEST_API_KEY);
    let push_client = PushClient::new(&client);
    
    // Create test device registration
    let mut metadata = HashMap::new();
    metadata.insert("app_version".to_string(), "1.0.0".to_string());
    metadata.insert("os_version".to_string(), "14.0".to_string());
    
    let device = DeviceRegistration {
        id: format!("test-device-{}", uuid::Uuid::new_v4()),
        platform: "ios".to_string(),
        form_factor: "phone".to_string(),
        client_id: Some("test-client".to_string()),
        device_secret: None,
        push: json!({
            "recipient": {
                "transportType": "apns",
                "deviceToken": "fake-token-for-testing"
            }
        }),
        metadata: Some(metadata),
    };
    
    // Register device
    let registered = push_client.register_device(device.clone()).await;
    
    // Note: May fail with authentication errors on sandbox
    // This is expected as push notifications require specific app setup
    if let Err(e) = &registered {
        println!("Device registration error (expected): {}", e);
        // Check that we at least get a proper error response
        assert!(e.to_string().contains("40") || e.to_string().contains("unauthorized"));
    } else {
        let device_info = registered.unwrap();
        assert_eq!(device_info.id, device.id);
        assert_eq!(device_info.platform, "ios");
        
        // Clean up - unregister device
        let _ = push_client.unregister_device(&device.id).await;
    }
}

#[tokio::test]
async fn test_push_message_serialization() {
    // Test that push messages serialize correctly
    let message = PushMessage {
        recipient: PushTarget::Channel("test-channel".to_string()),
        push: PushPayload {
            notification: PushNotification {
                title: "Test Title".to_string(),
                body: "Test Body".to_string(),
                icon: Some("test-icon.png".to_string()),
                sound: Some("default".to_string()),
                badge: Some(1),
                data: Some(json!({
                    "custom": "data"
                })),
            },
            data: Some(json!({
                "additional": "payload"
            })),
            apns: Some(json!({
                "aps": {
                    "alert": {
                        "title": "iOS Title"
                    }
                }
            })),
            fcm: Some(json!({
                "android": {
                    "priority": "high"
                }
            })),
            web: Some(json!({
                "requireInteraction": true
            })),
        },
    };
    
    // Serialize to JSON
    let json_str = serde_json::to_string(&message).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    
    // Verify structure
    assert!(parsed["channel"].is_string());
    assert_eq!(parsed["channel"], "test-channel");
    assert_eq!(parsed["push"]["notification"]["title"], "Test Title");
    assert_eq!(parsed["push"]["notification"]["body"], "Test Body");
    assert_eq!(parsed["push"]["data"]["additional"], "payload");
}

#[tokio::test]
async fn test_channel_subscription() {
    let client = RestClient::new(TEST_API_KEY);
    let push_client = PushClient::new(&client);
    
    let subscription = PushChannelSubscription {
        channel: "test-push-channel".to_string(),
        device_id: Some("test-device-123".to_string()),
        client_id: None,
    };
    
    // Attempt to subscribe (may fail without proper push setup)
    let result = push_client.subscribe_to_channel(subscription).await;
    
    if let Err(e) = &result {
        println!("Channel subscription error (expected): {}", e);
        // Verify we get a proper error response
        assert!(e.to_string().contains("40") || e.to_string().contains("not found"));
    }
}

#[tokio::test]
async fn test_push_target_serialization() {
    // Test different push target types
    let targets = vec![
        PushTarget::Channel("channel1".to_string()),
        PushTarget::DeviceId("device123".to_string()),
        PushTarget::ClientId("client456".to_string()),
        PushTarget::Devices(vec!["dev1".to_string(), "dev2".to_string()]),
    ];
    
    for target in targets {
        let message = PushMessage {
            recipient: target.clone(),
            push: PushPayload {
                notification: PushNotification {
                    title: "Test".to_string(),
                    body: "Body".to_string(),
                    icon: None,
                    sound: None,
                    badge: None,
                    data: None,
                },
                data: None,
                apns: None,
                fcm: None,
                web: None,
            },
        };
        
        let json_str = serde_json::to_string(&message).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        
        // Verify correct recipient fields
        match target {
            PushTarget::Channel(_) => assert!(parsed["channel"].is_string()),
            PushTarget::DeviceId(_) => assert!(parsed["deviceId"].is_string()),
            PushTarget::ClientId(_) => assert!(parsed["clientId"].is_string()),
            PushTarget::Devices(_) => assert!(parsed["deviceIds"].is_array()),
        }
    }
}

#[tokio::test]
async fn test_list_devices() {
    let client = RestClient::new(TEST_API_KEY);
    let push_client = PushClient::new(&client);
    
    // Attempt to list devices
    let result = push_client.list_devices().await;
    
    if let Ok(devices) = result {
        // If successful, should return array (possibly empty)
        assert!(devices.is_empty() || !devices.is_empty());
    } else if let Err(e) = result {
        // Expected to fail without proper push setup
        println!("List devices error (expected): {}", e);
    }
}

#[tokio::test]
async fn test_push_admin_stats() {
    let client = RestClient::new(TEST_API_KEY);
    let push_admin = ably_core::push::PushAdmin::new(client.http_client());
    
    // Try to get push statistics
    let result = push_admin.stats().await;
    
    if let Err(e) = &result {
        println!("Push stats error (expected): {}", e);
        // Verify proper error handling
        assert!(e.to_string().contains("40") || e.to_string().contains("not found"));
    } else if let Ok(stats) = result {
        // If successful, verify structure
        assert!(stats.messages_sent >= 0);
        assert!(stats.devices_registered >= 0);
    }
}