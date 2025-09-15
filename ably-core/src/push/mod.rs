// 🟡 YELLOW Phase: Push notification system implementation
// Enables push notifications to mobile devices and web browsers

use crate::client::rest::RestClient;
use crate::error::{AblyError, AblyResult};
use crate::http::AblyHttpClient;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Push client for managing push notifications
pub struct PushClient<'a> {
    http_client: &'a AblyHttpClient,
}

impl<'a> PushClient<'a> {
    /// Create a new push client
    pub fn new(rest_client: &'a RestClient) -> Self {
        Self {
            http_client: rest_client.http_client(),
        }
    }
    
    /// Register a device for push notifications
    pub async fn register_device(&self, device: DeviceRegistration) -> AblyResult<PushDevice> {
        let response = self.http_client
            .put(&format!("/push/deviceRegistrations/{}", device.id))
            .json(&device)
            .send()
            .await?;
        response.json().await
    }
    
    /// Get device details
    pub async fn get_device(&self, device_id: &str) -> AblyResult<PushDevice> {
        let response = self.http_client
            .get(&format!("/push/deviceRegistrations/{}", device_id))
            .send()
            .await?;
        response.json().await
    }
    
    /// Unregister a device
    pub async fn unregister_device(&self, device_id: &str) -> AblyResult<()> {
        self.http_client
            .delete(&format!("/push/deviceRegistrations/{}", device_id))
            .send()
            .await?;
        Ok(())
    }
    
    /// List all registered devices
    pub async fn list_devices(&self) -> AblyResult<Vec<PushDevice>> {
        let response = self.http_client
            .get("/push/deviceRegistrations")
            .send()
            .await?;
        response.json().await
    }
    
    /// Subscribe to a channel
    pub async fn subscribe_to_channel(&self, subscription: PushChannelSubscription) -> AblyResult<()> {
        self.http_client
            .post("/push/channelSubscriptions")
            .json(&subscription)
            .send()
            .await?;
        Ok(())
    }
    
    /// Unsubscribe from a channel
    pub async fn unsubscribe_from_channel(
        &self, 
        channel: &str, 
        device_id: Option<&str>,
        client_id: Option<&str>
    ) -> AblyResult<()> {
        let mut params = HashMap::new();
        params.insert("channel".to_string(), channel.to_string());
        
        if let Some(device) = device_id {
            params.insert("deviceId".to_string(), device.to_string());
        }
        if let Some(client) = client_id {
            params.insert("clientId".to_string(), client.to_string());
        }
        
        self.http_client
            .delete("/push/channelSubscriptions")
            .query(&params)
            .send()
            .await?;
        Ok(())
    }
    
    /// List channel subscriptions for a device
    pub async fn list_channel_subscriptions(&self, device_id: &str) -> AblyResult<Vec<PushChannel>> {
        let mut params = HashMap::new();
        params.insert("deviceId".to_string(), device_id.to_string());
        
        let response = self.http_client
            .get("/push/channelSubscriptions")
            .query(&params)
            .send()
            .await?;
        response.json().await
    }
    
    /// Publish a push notification
    pub async fn publish(&self, message: PushMessage) -> AblyResult<()> {
        self.http_client
            .post("/push/publish")
            .json(&message)
            .send()
            .await?;
        Ok(())
    }
    
    /// Publish batch push notifications
    pub async fn publish_batch(&self, messages: Vec<PushMessage>) -> AblyResult<()> {
        let batch_request = json!({
            "requests": messages.into_iter().map(|msg| {
                json!({
                    "push": msg.push,
                    "recipient": msg.recipient
                })
            }).collect::<Vec<_>>()
        });
        
        self.http_client
            .post("/push/publish")
            .json(&batch_request)
            .send()
            .await?;
        Ok(())
    }
}

/// Device registration for push notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceRegistration {
    pub id: String,
    pub platform: String,
    pub form_factor: String,
    pub client_id: Option<String>,
    pub device_secret: Option<String>,
    pub push: Value,
    pub metadata: Option<HashMap<String, String>>,
}

/// Registered push device
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushDevice {
    pub id: String,
    pub platform: String,
    pub form_factor: String,
    pub client_id: Option<String>,
    pub device_secret: Option<String>,
    pub push: Value,
    pub metadata: Option<HashMap<String, String>>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
}

/// Push channel subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushChannelSubscription {
    pub channel: String,
    pub device_id: Option<String>,
    pub client_id: Option<String>,
}

/// Push channel information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushChannel {
    pub channel: String,
    pub subscriptions: Option<i32>,
}

/// Target for push notification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PushTarget {
    Channel(String),
    DeviceId(String),
    ClientId(String),
    Devices(Vec<String>),
}

impl PushTarget {
    fn to_recipient(&self) -> Value {
        match self {
            PushTarget::Channel(channel) => json!({
                "channel": channel
            }),
            PushTarget::DeviceId(device_id) => json!({
                "deviceId": device_id  
            }),
            PushTarget::ClientId(client_id) => json!({
                "clientId": client_id
            }),
            PushTarget::Devices(devices) => json!({
                "deviceIds": devices
            }),
        }
    }
}

/// Push message to send
#[derive(Debug, Clone)]
pub struct PushMessage {
    pub recipient: PushTarget,
    pub push: PushPayload,
}

impl Serialize for PushMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(2))?;
        
        // Add recipient fields
        let recipient = self.recipient.to_recipient();
        if let Some(obj) = recipient.as_object() {
            for (k, v) in obj {
                map.serialize_entry(k, v)?;
            }
        }
        
        // Add push payload
        map.serialize_entry("push", &self.push)?;
        map.end()
    }
}

/// Push notification payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushPayload {
    pub notification: PushNotification,
    pub data: Option<Value>,
    pub apns: Option<Value>,
    pub fcm: Option<Value>,
    pub web: Option<Value>,
}

/// Push notification content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushNotification {
    pub title: String,
    pub body: String,
    pub icon: Option<String>,
    pub sound: Option<String>,
    pub badge: Option<i32>,
    pub data: Option<Value>,
}

/// Push admin interface for advanced operations
pub struct PushAdmin<'a> {
    http_client: &'a AblyHttpClient,
}

impl<'a> PushAdmin<'a> {
    /// Create new push admin interface
    pub fn new(http_client: &'a AblyHttpClient) -> Self {
        Self { http_client }
    }
    
    /// Get push statistics
    pub async fn stats(&self) -> AblyResult<PushStats> {
        let response = self.http_client
            .get("/push/stats")
            .send()
            .await?;
        response.json().await
    }
    
    /// Test push notification
    pub async fn test(&self, recipient: PushTarget) -> AblyResult<()> {
        let test_message = PushMessage {
            recipient,
            push: PushPayload {
                notification: PushNotification {
                    title: "Test Notification".to_string(),
                    body: "This is a test push notification".to_string(),
                    icon: None,
                    sound: Some("default".to_string()),
                    badge: None,
                    data: None,
                },
                data: Some(json!({
                    "test": true,
                    "timestamp": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                })),
                apns: None,
                fcm: None,
                web: None,
            },
        };
        
        self.http_client
            .post("/push/test")
            .json(&test_message)
            .send()
            .await?;
        Ok(())
    }
}

/// Push statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushStats {
    pub messages_sent: i64,
    pub messages_failed: i64,
    pub devices_registered: i64,
    pub channels_active: i64,
}

// Helper to import json! macro
use serde_json::json;