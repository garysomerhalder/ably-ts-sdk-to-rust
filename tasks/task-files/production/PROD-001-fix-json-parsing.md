# PROD-001: Fix JSON Parsing for Stats/History Endpoints

## 🎯 Objective
Fix critical JSON deserialization failures for stats, history, and channel list endpoints that prevent REST client from working with production Ably API responses.

## 📋 Task Details

**Priority:** 🔴 CRITICAL (Production Blocker)
**Effort:** 3-4 days
**Assignee:** Senior Rust Engineer
**Dependencies:** None

## 🔍 Problem Analysis

The current `Stats` struct in `protocol/messages.rs` doesn't match the actual Ably API response format, causing deserialization failures. Similar issues exist with history pagination and channel metadata responses.

### Current Failing Code
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub interval_id: Option<String>,
    pub unit: Option<String>,
    pub interval_granularity: Option<String>,
    // Missing nested structures causing failures
}
```

### Actual API Response Structure
```json
{
  "intervalId": "2024-01-17:10:21",
  "unit": "minute",
  "all": {
    "messages": { "count": 100, "data": 5000 },
    "presence": { "count": 50, "data": 2500 }
  },
  "inbound": { /* nested */ },
  "outbound": { /* nested */ }
}
```

## ✅ Acceptance Criteria

1. [ ] Stats endpoint returns valid data without parse errors
2. [ ] History pagination correctly parses Link headers
3. [ ] Channel list endpoint properly deserializes metadata
4. [ ] All existing integration tests pass
5. [ ] New tests added for complex JSON structures

## 🛠️ Implementation Plan

### Phase 1: Stats Structure Fix (Day 1)
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub interval_id: Option<String>,
    pub unit: Option<String>,
    pub interval_granularity: Option<String>,
    pub interval_time: Option<i64>,

    #[serde(default)]
    pub all: Option<MessageTypes>,
    #[serde(default)]
    pub inbound: Option<MessageTraffic>,
    #[serde(default)]
    pub outbound: Option<MessageTraffic>,
    #[serde(default)]
    pub persisted: Option<MessageTypes>,
    #[serde(default)]
    pub connections: Option<ResourceCount>,
    #[serde(default)]
    pub channels: Option<ResourceCount>,
    #[serde(default)]
    pub api_requests: Option<RequestCount>,
    #[serde(default)]
    pub token_requests: Option<RequestCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessageTypes {
    #[serde(default)]
    pub all: Option<MessageCount>,
    #[serde(default)]
    pub messages: Option<MessageCount>,
    #[serde(default)]
    pub presence: Option<MessageCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessageCount {
    pub count: Option<i64>,
    pub data: Option<i64>,
}
```

### Phase 2: History Pagination Fix (Day 2)
```rust
// In client/rest.rs line 379
fn parse_link_header(headers: &HeaderMap) -> Option<PaginationLinks> {
    headers.get("Link")?
        .to_str().ok()?
        .split(',')
        .filter_map(|link| {
            let parts: Vec<&str> = link.split(';').collect();
            if parts.len() != 2 { return None; }

            let url = parts[0].trim().trim_start_matches('<').trim_end_matches('>');
            let rel = parts[1].trim()
                .strip_prefix("rel=\"")?
                .strip_suffix("\"")?;

            Some((rel.to_string(), url.to_string()))
        })
        .collect()
}
```

### Phase 3: Channel Metadata Fix (Day 3)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelDetails {
    pub channel_id: String,
    pub name: String,
    pub status: Option<ChannelStatus>,
    pub occupancy: Option<ChannelOccupancy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelOccupancy {
    pub metrics: Option<ChannelMetrics>,
    pub presence_members: Option<i32>,
    pub presence_connections: Option<i32>,
    pub publishers: Option<i32>,
    pub subscribers: Option<i32>,
}
```

### Phase 4: Integration Testing (Day 4)
```rust
#[tokio::test]
async fn test_actual_api_stats_parsing() {
    let client = RestClient::new(ABLY_API_KEY);
    let stats = client.stats()
        .unit("minute")
        .limit(10)
        .execute()
        .await
        .expect("Stats should parse correctly");

    assert!(stats.items.len() > 0);
    if let Some(item) = stats.items.first() {
        assert!(item.interval_id.is_some());
        // Verify nested structures parse correctly
        if let Some(all) = &item.all {
            println!("Messages: {:?}", all.messages);
        }
    }
}
```

## 🚦 Traffic-Light Development

### 🔴 RED Phase
- Write failing tests that demonstrate parsing failures
- Capture actual API responses for test data
- Document all failing deserialization points

### 🟡 YELLOW Phase
- Update structs to match actual API responses
- Make all fields Option<T> with serde(default)
- Implement Link header parsing

### 🟢 GREEN Phase
- Add comprehensive error handling
- Implement retry on parse failures
- Add debug logging for malformed responses
- Performance optimization for large responses

## 🧪 Testing Strategy

1. **Unit Tests**: Test each struct against known JSON samples
2. **Integration Tests**: Test against real Ably API
3. **Edge Cases**: Handle missing fields, null values, empty responses
4. **Performance**: Test with large datasets (1000+ stats items)

## 📚 References

- [Ably REST API Docs](https://ably.com/docs/rest-api)
- [Serde JSON Documentation](https://serde.rs/json.html)
- Current failing tests: `test_stats_endpoint`, `test_actual_api_history_format`

## ⚠️ Risks & Mitigations

- **Risk**: API response format changes
- **Mitigation**: Make all fields optional with defaults

- **Risk**: Performance impact with deeply nested structures
- **Mitigation**: Use Box<T> for large nested structures

## 📊 Success Metrics

- ✅ All REST API endpoints return valid data
- ✅ Zero JSON parsing errors in production logs
- ✅ Integration tests pass consistently
- ✅ Performance: < 10ms parsing for typical responses