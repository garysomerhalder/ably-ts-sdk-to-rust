// 🔴 RED Phase: Push notification system tests
// Enables push notifications to mobile devices and web browsers

use ably_core::client::rest::RestClient;
use ably_core::push::{
    PushClient, PushChannel, PushDevice, PushMessage,
    DeviceRegistration, PushChannelSubscription,
    PushPayload, PushNotification, PushTarget
};
use serde_json::json;
use std::collections::HashMap;

// Note: Using placeholder API key - needs valid sandbox key
const TEST_API_KEY: &str = "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA";

#[tokio::test]
async fn test_device_registration() {
    let client = RestClient::new(TEST_API_KEY);
    let push_client = PushClient::new(&client);
    
    // Register a device
    let device = DeviceRegistration {
        id: "test-device-001".to_string(),
        platform: "android".to_string(),
        form_factor: "phone".to_string(),
        client_id: Some("test-client".to_string()),
        device_secret: None,
        push: json!({
            "recipient": {
                "transportType": "fcm",
                "registrationToken": "fake-fcm-token-12345"
            }
        }),
        metadata: None,
    };
    
    let result = push_client.register_device(device).await;
    assert!(result.is_ok());
    
    // Get device details
    let device = push_client.get_device("test-device-001").await.unwrap();
    assert_eq!(device.id, "test-device-001");
    assert_eq!(device.platform, "android");
}

#[tokio::test]
async fn test_channel_subscription() {
    let client = RestClient::new(TEST_API_KEY);
    let push_client = PushClient::new(&client);
    
    // Subscribe device to channel
    let subscription = PushChannelSubscription {
        channel: "news:sports".to_string(),
        device_id: Some("test-device-001".to_string()),
        client_id: None,
    };
    
    let result = push_client.subscribe_to_channel(subscription).await;
    assert!(result.is_ok());
    
    // List subscriptions for device
    let subscriptions = push_client
        .list_channel_subscriptions("test-device-001")
        .await
        .unwrap();
    
    assert!(subscriptions.iter().any(|s| s.channel == "news:sports"));
}

#[tokio::test]
async fn test_push_message_publish() {
    let client = RestClient::new(TEST_API_KEY);
    let push_client = PushClient::new(&client);
    
    // Create push message
    let message = PushMessage {
        recipient: PushTarget::Channel("news:sports".to_string()),
        push: PushPayload {
            notification: PushNotification {
                title: "Breaking News".to_string(),
                body: "Important sports update!".to_string(),
                icon: Some("https://example.com/icon.png".to_string()),
                sound: Some("default".to_string()),
                badge: None,
                data: None,
            },
            data: Some(json!({
                "article_id": "12345",
                "category": "sports"
            })),
            apns: None,
            fcm: None,
            web: None,
        },
    };
    
    let result = push_client.publish(message).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_push_to_client_id() {
    let client = RestClient::new(TEST_API_KEY);
    let push_client = PushClient::new(&client);
    
    // Push to specific client ID
    let message = PushMessage {
        recipient: PushTarget::ClientId("test-client".to_string()),
        push: PushPayload {
            notification: PushNotification {
                title: "Personal Message".to_string(),
                body: "You have a new message!".to_string(),
                icon: None,
                sound: Some("notification".to_string()),
                badge: Some(1),
                data: None,
            },
            data: None,
            apns: None,
            fcm: None,
            web: None,
        },
    };
    
    let result = push_client.publish(message).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_push_to_device() {
    let client = RestClient::new(TEST_API_KEY);
    let push_client = PushClient::new(&client);
    
    // Push to specific device
    let message = PushMessage {
        recipient: PushTarget::DeviceId("test-device-001".to_string()),
        push: PushPayload {
            notification: PushNotification {
                title: "Device Alert".to_string(),
                body: "This is sent to your device".to_string(),
                icon: None,
                sound: None,
                badge: None,
                data: Some(json!({
                    "action": "open_app"
                })),
            },
            data: None,
            apns: None,
            fcm: None,
            web: None,
        },
    };
    
    let result = push_client.publish(message).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_platform_specific_push() {
    let client = RestClient::new(TEST_API_KEY);
    let push_client = PushClient::new(&client);
    
    // Push with platform-specific payloads
    let message = PushMessage {
        recipient: PushTarget::Channel("updates".to_string()),
        push: PushPayload {
            notification: PushNotification {
                title: "Update Available".to_string(),
                body: "New version is ready".to_string(),
                icon: None,
                sound: None,
                badge: None,
                data: None,
            },
            data: None,
            // iOS specific
            apns: Some(json!({
                "aps": {
                    "alert": {
                        "title": "Update Available",
                        "subtitle": "Version 2.0",
                        "body": "New features await!"
                    },
                    "sound": "update.caf",
                    "category": "UPDATE_CATEGORY"
                }
            })),
            // Android specific
            fcm: Some(json!({
                "notification": {
                    "title": "Update Available",
                    "body": "New version is ready",
                    "android_channel_id": "updates",
                    "icon": "ic_update"
                },
                "android": {
                    "priority": "high",
                    "ttl": "86400s"
                }
            })),
            // Web push specific
            web: Some(json!({
                "notification": {
                    "title": "Update Available",
                    "body": "New version is ready",
                    "icon": "/icon-192.png",
                    "badge": "/badge-72.png",
                    "actions": [
                        {
                            "action": "update",
                            "title": "Update Now"
                        },
                        {
                            "action": "later",
                            "title": "Later"
                        }
                    ]
                }
            })),
        },
    };
    
    let result = push_client.publish(message).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_device_unregistration() {
    let client = RestClient::new(TEST_API_KEY);
    let push_client = PushClient::new(&client);
    
    // Unregister device
    let result = push_client.unregister_device("test-device-001").await;
    assert!(result.is_ok());
    
    // Verify device is gone
    let device = push_client.get_device("test-device-001").await;
    assert!(device.is_err());
}

#[tokio::test]
async fn test_batch_push() {
    let client = RestClient::new(TEST_API_KEY);
    let push_client = PushClient::new(&client);
    
    // Send batch push to multiple recipients
    let messages = vec![
        PushMessage {
            recipient: PushTarget::ClientId("client1".to_string()),
            push: PushPayload {
                notification: PushNotification {
                    title: "Message 1".to_string(),
                    body: "Content 1".to_string(),
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
        },
        PushMessage {
            recipient: PushTarget::ClientId("client2".to_string()),
            push: PushPayload {
                notification: PushNotification {
                    title: "Message 2".to_string(),
                    body: "Content 2".to_string(),
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
        },
    ];
    
    let result = push_client.publish_batch(messages).await;
    assert!(result.is_ok());
}