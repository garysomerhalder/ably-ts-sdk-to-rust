// 🟡 YELLOW Phase: Comprehensive REST client implementation
// Supports all major Ably REST API endpoints

use crate::auth::{AuthMode, TokenDetails, TokenRequest};
use crate::error::{AblyError, AblyResult};
use crate::http::{AblyHttpClient, HttpConfig};
use crate::protocol::messages::{Message, PresenceMessage};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, info, warn};

/// Main REST client for Ably API
pub struct RestClient {
    http_client: AblyHttpClient,
    environment: String,
}

impl RestClient {
    /// Create a new REST client with an API key
    pub fn new(api_key: impl Into<String>) -> Self {
        let config = HttpConfig::default();
        let auth = AuthMode::ApiKey(api_key.into());
        
        Self {
            http_client: AblyHttpClient::with_auth(config, auth),
            environment: "production".to_string(),
        }
    }
    
    /// Create a new REST client with a token
    pub fn with_token(token: impl Into<String>) -> Self {
        let config = HttpConfig::default();
        let auth = AuthMode::Token(token.into());
        
        Self {
            http_client: AblyHttpClient::with_auth(config, auth),
            environment: "production".to_string(),
        }
    }
    
    /// Create a builder for advanced configuration
    pub fn builder() -> RestClientBuilder {
        RestClientBuilder::default()
    }
    
    /// Get server time
    pub async fn time(&self) -> AblyResult<i64> {
        let response: Vec<i64> = self.http_client
            .get("/time")
            .send()
            .await?;
        
        response.first().copied()
            .ok_or_else(|| AblyError::unexpected("Empty time response"))
    }
    
    /// Get statistics
    pub fn stats(&self) -> StatsQuery {
        StatsQuery::new(&self.http_client)
    }
    
    /// Get a channel reference
    pub fn channel(&self, name: impl Into<String>) -> Channel {
        Channel::new(name.into(), &self.http_client)
    }
    
    /// Get channels metadata
    pub fn channels(&self) -> ChannelsQuery {
        ChannelsQuery::new(&self.http_client)
    }
    
    /// Authentication operations
    pub fn auth(&self) -> AuthOperations {
        AuthOperations::new(&self.http_client)
    }
    
    /// Push admin operations
    pub fn push(&self) -> PushAdmin {
        PushAdmin::new(&self.http_client)
    }
    
    /// Batch operations
    pub fn batch(&self) -> BatchRequest {
        BatchRequest::new(&self.http_client)
    }
}

/// Builder for REST client with advanced options
pub struct RestClientBuilder {
    api_key: Option<String>,
    token: Option<String>,
    environment: String,
    timeout: Option<Duration>,
    max_retries: u32,
    custom_headers: HashMap<String, String>,
}

impl Default for RestClientBuilder {
    fn default() -> Self {
        Self {
            api_key: None,
            token: None,
            environment: "production".to_string(),
            timeout: Some(Duration::from_secs(15)),
            max_retries: 3,
            custom_headers: HashMap::new(),
        }
    }
}

impl RestClientBuilder {
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }
    
    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }
    
    pub fn environment(mut self, env: impl Into<String>) -> Self {
        self.environment = env.into();
        self
    }
    
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
    
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }
    
    pub fn custom_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom_headers.insert(key.into(), value.into());
        self
    }
    
    pub fn build(self) -> RestClient {
        let mut config = HttpConfig::default();
        
        if let Some(timeout) = self.timeout {
            config.timeout = timeout;
        }
        
        config.max_retries = self.max_retries;
        
        // Set base URL based on environment
        config.base_url = match self.environment.as_str() {
            "sandbox" => "https://sandbox-rest.ably.io".to_string(),
            "production" | _ => "https://rest.ably.io".to_string(),
        };
        
        let auth = if let Some(key) = self.api_key {
            AuthMode::ApiKey(key)
        } else if let Some(token) = self.token {
            AuthMode::Token(token)
        } else {
            panic!("Either API key or token must be provided");
        };
        
        let mut http_client = AblyHttpClient::with_auth(config, auth);
        
        // Add custom headers
        for (key, value) in self.custom_headers {
            http_client.add_default_header(key, value);
        }
        
        RestClient {
            http_client,
            environment: self.environment,
        }
    }
}

/// Channel operations
pub struct Channel<'a> {
    name: String,
    http_client: &'a AblyHttpClient,
}

impl<'a> Channel<'a> {
    fn new(name: String, http_client: &'a AblyHttpClient) -> Self {
        Self { name, http_client }
    }
    
    /// Publish a single message
    pub async fn publish(&self, message: Message) -> AblyResult<()> {
        let path = format!("/channels/{}/messages", self.name);
        self.http_client
            .post(&path)
            .json(&message)
            .send::<Value>()
            .await?;
        Ok(())
    }
    
    /// Publish multiple messages
    pub async fn publish_batch(&self, messages: Vec<Message>) -> AblyResult<()> {
        let path = format!("/channels/{}/messages", self.name);
        self.http_client
            .post(&path)
            .json(&messages)
            .send::<Value>()
            .await?;
        Ok(())
    }
    
    /// Get message history
    pub fn history(&self) -> HistoryQuery<'a> {
        HistoryQuery::new(&self.name, self.http_client)
    }
    
    /// Get channel presence
    pub fn presence(&self) -> PresenceOperations<'a> {
        PresenceOperations::new(&self.name, self.http_client)
    }
    
    /// Get channel status
    pub async fn status(&self) -> AblyResult<ChannelStatus> {
        let path = format!("/channels/{}", self.name);
        self.http_client
            .get(&path)
            .send()
            .await
    }
}

/// Channel options for advanced features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelOptions {
    pub cipher: Option<CipherParams>,
    pub params: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CipherParams {
    pub algorithm: String,
    pub key: String,
}

/// Channel status information
#[derive(Debug, Deserialize)]
pub struct ChannelStatus {
    #[serde(rename = "channelId")]
    pub channel_id: String,
    pub name: Option<String>,
    pub status: Option<ChannelDetails>,
}

#[derive(Debug, Deserialize)]
pub struct ChannelDetails {
    #[serde(rename = "isActive")]
    pub is_active: bool,
    pub occupancy: Option<ChannelOccupancy>,
}

#[derive(Debug, Deserialize)]
pub struct ChannelOccupancy {
    pub metrics: ChannelMetrics,
}

#[derive(Debug, Deserialize)]
pub struct ChannelMetrics {
    pub connections: u32,
    pub publishers: u32,
    pub subscribers: u32,
    #[serde(rename = "presenceConnections")]
    pub presence_connections: u32,
    #[serde(rename = "presenceMembers")]
    pub presence_members: u32,
    #[serde(rename = "presenceSubscribers")]
    pub presence_subscribers: u32,
}

/// Query builder for history requests
pub struct HistoryQuery<'a> {
    channel: String,
    http_client: &'a AblyHttpClient,
    params: HashMap<String, String>,
}

impl<'a> HistoryQuery<'a> {
    fn new(channel: &str, http_client: &'a AblyHttpClient) -> Self {
        Self {
            channel: channel.to_string(),
            http_client,
            params: HashMap::new(),
        }
    }
    
    pub fn limit(mut self, limit: u32) -> Self {
        self.params.insert("limit".to_string(), limit.to_string());
        self
    }
    
    pub fn direction(mut self, direction: &str) -> Self {
        self.params.insert("direction".to_string(), direction.to_string());
        self
    }
    
    pub fn start(mut self, start: i64) -> Self {
        self.params.insert("start".to_string(), start.to_string());
        self
    }
    
    pub fn end(mut self, end: i64) -> Self {
        self.params.insert("end".to_string(), end.to_string());
        self
    }
    
    pub async fn execute(&self) -> AblyResult<PaginatedResult<'a, Message>> {
        let path = format!("/channels/{}/messages", self.channel);
        let response: Vec<Message> = self.http_client
            .get(&path)
            .query(&self.params)
            .send()
            .await?;
        
        Ok(PaginatedResult {
            items: response,
            http_client: self.http_client,
            next_url: None, // TODO: Parse Link header
        })
    }
}

/// Presence operations
pub struct PresenceOperations<'a> {
    channel: String,
    http_client: &'a AblyHttpClient,
}

impl<'a> PresenceOperations<'a> {
    fn new(channel: &str, http_client: &'a AblyHttpClient) -> Self {
        Self {
            channel: channel.to_string(),
            http_client,
        }
    }
    
    pub async fn get(&self) -> AblyResult<PaginatedResult<'a, PresenceMessage>> {
        let path = format!("/channels/{}/presence", self.channel);
        let response: Vec<PresenceMessage> = self.http_client
            .get(&path)
            .send()
            .await?;
        
        Ok(PaginatedResult {
            items: response,
            http_client: self.http_client,
            next_url: None,
        })
    }
    
    pub fn history(&self) -> PresenceHistoryQuery<'a> {
        PresenceHistoryQuery::new(&self.channel, self.http_client)
    }
}

/// Query builder for presence history
pub struct PresenceHistoryQuery<'a> {
    channel: String,
    http_client: &'a AblyHttpClient,
    params: HashMap<String, String>,
}

impl<'a> PresenceHistoryQuery<'a> {
    fn new(channel: &str, http_client: &'a AblyHttpClient) -> Self {
        Self {
            channel: channel.to_string(),
            http_client,
            params: HashMap::new(),
        }
    }
    
    pub fn limit(mut self, limit: u32) -> Self {
        self.params.insert("limit".to_string(), limit.to_string());
        self
    }
    
    pub async fn execute(&self) -> AblyResult<PaginatedResult<'a, PresenceMessage>> {
        let path = format!("/channels/{}/presence/history", self.channel);
        let response: Vec<PresenceMessage> = self.http_client
            .get(&path)
            .query(&self.params)
            .send()
            .await?;
        
        Ok(PaginatedResult {
            items: response,
            http_client: self.http_client,
            next_url: None,
        })
    }
}

/// Stats query builder
pub struct StatsQuery<'a> {
    http_client: &'a AblyHttpClient,
    params: HashMap<String, String>,
}

impl<'a> StatsQuery<'a> {
    fn new(http_client: &'a AblyHttpClient) -> Self {
        Self {
            http_client,
            params: HashMap::new(),
        }
    }
    
    pub fn limit(mut self, limit: u32) -> Self {
        self.params.insert("limit".to_string(), limit.to_string());
        self
    }
    
    pub fn direction(mut self, direction: &str) -> Self {
        self.params.insert("direction".to_string(), direction.to_string());
        self
    }
    
    pub async fn execute(&self) -> AblyResult<PaginatedResult<'a, Stats>> {
        let response: Vec<Stats> = self.http_client
            .get("/stats")
            .query(&self.params)
            .send()
            .await?;
        
        Ok(PaginatedResult {
            items: response,
            http_client: self.http_client,
            next_url: None,
        })
    }
}

/// Statistics data
#[derive(Debug, Deserialize)]
pub struct Stats {
    pub all: StatsMessageTypes,
    pub inbound: Option<StatsMessageTraffic>,
    pub outbound: Option<StatsMessageTraffic>,
}

#[derive(Debug, Deserialize)]
pub struct StatsMessageTypes {
    pub messages: StatsMessageCount,
    pub presence: StatsMessageCount,
}

#[derive(Debug, Deserialize)]
pub struct StatsMessageCount {
    pub count: u64,
    pub data: u64,
}

#[derive(Debug, Deserialize)]
pub struct StatsMessageTraffic {
    pub realtime: StatsMessageTypes,
    pub rest: StatsMessageTypes,
}

/// Channels metadata query
pub struct ChannelsQuery<'a> {
    http_client: &'a AblyHttpClient,
    params: HashMap<String, String>,
}

impl<'a> ChannelsQuery<'a> {
    fn new(http_client: &'a AblyHttpClient) -> Self {
        Self {
            http_client,
            params: HashMap::new(),
        }
    }
    
    pub fn list(&self) -> ChannelListQuery<'a> {
        ChannelListQuery::new(self.http_client)
    }
}

pub struct ChannelListQuery<'a> {
    http_client: &'a AblyHttpClient,
    params: HashMap<String, String>,
}

impl<'a> ChannelListQuery<'a> {
    fn new(http_client: &'a AblyHttpClient) -> Self {
        Self {
            http_client,
            params: HashMap::new(),
        }
    }
    
    pub fn prefix(mut self, prefix: &str) -> Self {
        self.params.insert("prefix".to_string(), prefix.to_string());
        self
    }
    
    pub fn limit(mut self, limit: u32) -> Self {
        self.params.insert("limit".to_string(), limit.to_string());
        self
    }
    
    pub async fn execute(&self) -> AblyResult<Vec<ChannelStatus>> {
        self.http_client
            .get("/channels")
            .query(&self.params)
            .send()
            .await
    }
}

/// Authentication operations
pub struct AuthOperations<'a> {
    http_client: &'a AblyHttpClient,
}

impl<'a> AuthOperations<'a> {
    fn new(http_client: &'a AblyHttpClient) -> Self {
        Self { http_client }
    }
    
    pub fn request_token(&self) -> TokenRequestBuilder<'a> {
        TokenRequestBuilder::new(self.http_client)
    }
}

pub struct TokenRequestBuilder<'a> {
    http_client: &'a AblyHttpClient,
    params: HashMap<String, Value>,
}

impl<'a> TokenRequestBuilder<'a> {
    fn new(http_client: &'a AblyHttpClient) -> Self {
        Self {
            http_client,
            params: HashMap::new(),
        }
    }
    
    pub fn capability(mut self, channel: &str, operations: &[&str]) -> Self {
        let capability = json!({
            channel: operations
        });
        self.params.insert("capability".to_string(), capability);
        self
    }
    
    pub fn ttl(mut self, seconds: u64) -> Self {
        self.params.insert("ttl".to_string(), json!(seconds * 1000));
        self
    }
    
    pub async fn execute(&self) -> AblyResult<TokenDetails> {
        self.http_client
            .post("/keys/token")
            .json(&self.params)
            .send()
            .await
    }
}

/// Push admin operations
pub struct PushAdmin<'a> {
    http_client: &'a AblyHttpClient,
}

impl<'a> PushAdmin<'a> {
    fn new(http_client: &'a AblyHttpClient) -> Self {
        Self { http_client }
    }
    
    pub fn publish(&self) -> PushPublishBuilder<'a> {
        PushPublishBuilder::new(self.http_client)
    }
}

pub struct PushPublishBuilder<'a> {
    http_client: &'a AblyHttpClient,
    payload: HashMap<String, Value>,
}

impl<'a> PushPublishBuilder<'a> {
    fn new(http_client: &'a AblyHttpClient) -> Self {
        Self {
            http_client,
            payload: HashMap::new(),
        }
    }
    
    pub fn recipient(mut self, recipient: Value) -> Self {
        self.payload.insert("recipient".to_string(), recipient);
        self
    }
    
    pub fn notification(mut self, notification: Value) -> Self {
        self.payload.insert("push".to_string(), json!({
            "notification": notification
        }));
        self
    }
    
    pub async fn execute(&self) -> AblyResult<()> {
        self.http_client
            .post("/push/publish")
            .json(&self.payload)
            .send::<Value>()
            .await?;
        Ok(())
    }
}

/// Batch request operations
pub struct BatchRequest<'a> {
    http_client: &'a AblyHttpClient,
    requests: Vec<BatchRequestSpec>,
}

impl<'a> BatchRequest<'a> {
    fn new(http_client: &'a AblyHttpClient) -> Self {
        Self {
            http_client,
            requests: Vec::new(),
        }
    }
    
    pub fn add_request(
        mut self,
        id: &str,
        action: &str,
        method: &str,
        headers: Option<HashMap<String, String>>,
        params: Option<Value>,
    ) -> Self {
        self.requests.push(BatchRequestSpec {
            id: id.to_string(),
            action: action.to_string(),
            method: method.to_string(),
            headers,
            params,
        });
        self
    }
    
    pub async fn execute(&self) -> AblyResult<Vec<BatchResponse>> {
        let payload = json!({
            "requests": self.requests
        });
        
        self.http_client
            .post("/batch")
            .json(&payload)
            .send()
            .await
    }
}

#[derive(Debug, Serialize)]
struct BatchRequestSpec {
    id: String,
    action: String,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct BatchResponse {
    pub id: String,
    pub success: bool,
    pub status_code: u16,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<Value>,
}

/// Paginated result with navigation
pub struct PaginatedResult<'a, T> {
    pub items: Vec<T>,
    http_client: &'a AblyHttpClient,
    next_url: Option<String>,
}

impl<'a, T: for<'de> Deserialize<'de>> PaginatedResult<'a, T> {
    pub fn next(&self) -> Option<PaginationQuery<'a, T>> {
        self.next_url.as_ref().map(|url| {
            PaginationQuery::new(url.clone(), self.http_client)
        })
    }
    
    pub fn has_next(&self) -> bool {
        self.next_url.is_some()
    }
}

pub struct PaginationQuery<'a, T> {
    url: String,
    http_client: &'a AblyHttpClient,
    _phantom: std::marker::PhantomData<T>,
}

impl<'a, T: for<'de> Deserialize<'de>> PaginationQuery<'a, T> {
    fn new(url: String, http_client: &'a AblyHttpClient) -> Self {
        Self {
            url,
            http_client,
            _phantom: std::marker::PhantomData,
        }
    }
    
    pub async fn execute(&self) -> AblyResult<PaginatedResult<'a, T>> {
        let response: Vec<T> = self.http_client
            .get(&self.url)
            .send()
            .await?;
        
        Ok(PaginatedResult {
            items: response,
            http_client: self.http_client,
            next_url: None, // TODO: Parse Link header
        })
    }
}

// Re-export commonly used types
pub use crate::protocol::messages::Message as MessageBuilder;