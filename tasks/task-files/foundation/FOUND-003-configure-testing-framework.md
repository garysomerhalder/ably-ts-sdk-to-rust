# Task: FOUND-003 - Configure Testing Framework

## 📋 Overview
- **Status**: 🔴 TODO  
- **Assignee**: Claude
- **Estimated Effort**: 4 hours
- **Actual Effort**: -
- **Start Date**: TBD
- **Completion Date**: -
- **Priority**: HIGH (Foundation - Critical for Integration-First)

## 🔗 Dependencies
- **Depends On**: FOUND-001 (Complete)
- **Blocks**: INFRA-001, all testing-dependent tasks
- **Parallel With**: FOUND-002 (CI/CD setup)

## 🔴 RED Phase: Define the Problem

### Tests to Write (Integration-First - NO MOCKS!)
- [ ] Test connection to real Ably sandbox environment
- [ ] Test API key authentication against live Ably
- [ ] Test basic HTTP request/response cycle
- [ ] Test credential loading from secure storage
- [ ] Test cleanup of test data after runs

### Expected Failures
- No test framework configuration
- No Ably credentials setup
- No test environment isolation
- No cleanup procedures for test data

### Acceptance Criteria
- [ ] Integration test framework configured (tokio-test)
- [ ] Real Ably sandbox credentials securely managed
- [ ] Test helpers for common Ably operations  
- [ ] Automatic test data cleanup
- [ ] Clear separation of unit vs integration tests

## 🟡 YELLOW Phase: Minimal Implementation

### Implementation Checklist
- [ ] Add tokio-test and criterion to dev-dependencies
- [ ] Create `/reference/ably-credentials.env.example`
- [ ] Set up test credential loading
- [ ] Create basic test helper functions
- [ ] Add simple integration test against Ably REST API

### Code Components
```rust
// tests/common/mod.rs
pub fn load_test_credentials() -> AblyCredentials {
    // Load from environment or reference file
}

pub async fn create_test_client() -> Result<AblyClient> {
    // Create client with test credentials
}

// tests/integration/basic_connection.rs
#[tokio::test]
async fn test_basic_api_connection() {
    let client = create_test_client().await.unwrap();
    let response = client.get_server_time().await;
    assert!(response.is_ok());
}
```

### Success Criteria
- [ ] Can run tests against real Ably sandbox
- [ ] Test credentials securely managed
- [ ] Basic integration test passes
- [ ] Clear test vs production separation

## 🟢 GREEN Phase: Production Hardening

### Hardening Checklist
- [ ] Comprehensive test helper library
- [ ] Test data cleanup automation
- [ ] Parallel test execution safety
- [ ] Rate limiting for API calls
- [ ] Test environment health checks
- [ ] Error handling and retry logic
- [ ] Test metrics and reporting

### Advanced Testing Features
- [ ] Property-based testing with proptest
- [ ] Performance benchmarking with criterion
- [ ] Test fixtures and factories
- [ ] Mock-free testing patterns
- [ ] Integration test orchestration

### Security & Reliability
- [ ] Credential rotation support
- [ ] Test isolation (no shared state)
- [ ] Cleanup verification
- [ ] Resource leak detection
- [ ] Environment validation

### Production Criteria
- [ ] Complete test suite infrastructure
- [ ] Zero reliance on mocks or fakes
- [ ] Automated cleanup and validation
- [ ] Comprehensive error handling
- [ ] Performance and reliability metrics

## 🗂️ Reference Files Needed

### `/reference/ably-credentials.env.example`
```bash
# Copy to ably-credentials.env and fill with real values
ABLY_API_KEY_SANDBOX=your_sandbox_key_here
ABLY_APP_ID_SANDBOX=your_app_id_here
ABLY_ENVIRONMENT=sandbox
```

### Security Notes
- Real credentials go in `/reference/ably-credentials.env` (gitignored)
- Never commit actual credentials to repository
- Use dedicated test app in Ably dashboard
- Implement credential rotation procedures

## 📊 Metrics
- Test Execution Speed: TBD
- API Call Success Rate: TBD  
- Cleanup Success Rate: TBD
- Integration Coverage: TBD

## 📝 Notes
- CRITICAL: Absolutely no mocks or fakes (Integration-First requirement)
- Must test against real Ably services only
- Implement proper cleanup to avoid test data pollution
- Consider rate limiting and API quotas