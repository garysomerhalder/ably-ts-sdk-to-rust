# Task: INFRA-001 - Implement HTTP Client with reqwest

## 📋 Overview
- **Status**: 🔴 TODO
- **Assignee**: Claude  
- **Estimated Effort**: 5 hours
- **Actual Effort**: -
- **Start Date**: TBD
- **Completion Date**: -
- **Priority**: HIGH (Infrastructure Foundation)

## 🔗 Dependencies
- **Depends On**: FOUND-003 (Testing Framework - REQUIRED)
- **Blocks**: INFRA-002 (Retry Logic), All API-dependent tasks
- **Integrates With**: Authentication system, Error handling

## 🔴 RED Phase: Define the Problem

### Tests to Write (Integration-First - Real Ably APIs Only!)
- [ ] Test basic GET request to Ably REST API `/time`
- [ ] Test authentication headers in requests
- [ ] Test request/response serialization (JSON)
- [ ] Test connection timeout handling
- [ ] Test invalid endpoint error handling
- [ ] Test rate limiting response (429) handling

### Expected Failures (Write First!)
- No HTTP client implementation exists
- No Ably endpoint configuration
- No authentication header management
- No timeout or error handling
- No request/response logging

### Acceptance Criteria
- [ ] HTTP client can make requests to real Ably REST API
- [ ] Proper authentication header injection
- [ ] JSON serialization/deserialization working
- [ ] Basic error handling for common HTTP errors
- [ ] Integration tests pass against live API

## 🟡 YELLOW Phase: Minimal Implementation

### Implementation Checklist  
- [ ] Create `AblyHttpClient` struct with reqwest
- [ ] Add basic authentication header support
- [ ] Implement GET/POST methods
- [ ] Add JSON request/response handling
- [ ] Create basic error types
- [ ] Add timeout configuration

### Code Components
```rust
// ably-core/src/http/client.rs
pub struct AblyHttpClient {
    client: reqwest::Client,
    base_url: String,
    auth_header: String,
}

impl AblyHttpClient {
    pub fn new(api_key: &str) -> Self { 
        // Basic HTTP client with auth
    }
    
    pub async fn get<T>(&self, path: &str) -> Result<T> 
    where T: DeserializeOwned 
    {
        // GET request with auth headers
    }
    
    pub async fn post<T, R>(&self, path: &str, body: T) -> Result<R>
    where T: Serialize, R: DeserializeOwned
    {
        // POST request with JSON body
    }
}

// Integration test
#[tokio::test]
async fn test_get_server_time() {
    let client = create_test_client().await;
    let response: ServerTimeResponse = client.get("/time").await.unwrap();
    assert!(response.timestamp > 0);
}
```

### Success Criteria
- [ ] Can successfully call Ably `/time` endpoint
- [ ] Authentication working with real API key
- [ ] JSON responses parsed correctly
- [ ] Basic error handling functional

## 🟢 GREEN Phase: Production Hardening

### Hardening Checklist
- [ ] Connection pooling and reuse
- [ ] Request/response logging with tracing
- [ ] Comprehensive error handling
- [ ] Timeout configuration per request type
- [ ] User-Agent header with SDK version
- [ ] Request ID generation for tracing
- [ ] HTTP/2 support optimization
- [ ] TLS configuration hardening

### Advanced Features
- [ ] Request middleware system
- [ ] Response caching (where appropriate)
- [ ] Request metrics collection
- [ ] Circuit breaker integration
- [ ] Custom header injection
- [ ] Request signing for JWT auth
- [ ] Streaming response support

### Error Handling
```rust
#[derive(thiserror::Error, Debug)]
pub enum HttpError {
    #[error("Request timeout: {0}")]
    Timeout(#[from] reqwest::Error),
    
    #[error("Authentication failed: {status}")]
    AuthenticationFailed { status: u16 },
    
    #[error("Rate limited: {retry_after:?}")]
    RateLimited { retry_after: Option<u64> },
    
    #[error("Server error: {status} - {message}")]
    ServerError { status: u16, message: String },
}
```

### Performance Targets
- Request latency: < 100ms (regional)
- Connection reuse: > 90%
- Memory usage: < 50MB for client
- TLS handshake optimization: < 50ms

### Production Criteria
- [ ] Production-ready error handling
- [ ] Comprehensive logging and observability
- [ ] Performance optimizations in place
- [ ] Security best practices implemented
- [ ] Full integration test coverage
- [ ] Documentation with examples

## 🌐 Ably API Integration

### Key Endpoints to Support
```rust
// Core REST API endpoints
/time                    // Server time
/keys/{keyId}           // API key details  
/apps/{appId}/stats     // Application statistics
/apps/{appId}/channels  // Channel enumeration
```

### Authentication Methods
- API Key (Basic auth)
- JWT Token (Bearer auth)
- Client ID injection

## 📊 Metrics
- Request Success Rate: TBD
- Average Response Time: TBD
- Connection Pool Efficiency: TBD
- Error Rate by Type: TBD

## 📝 Notes
- MUST test against real Ably REST API (no mocks!)
- Follow Ably's rate limiting guidelines
- Implement proper User-Agent identification
- Consider implementing request signing early for security