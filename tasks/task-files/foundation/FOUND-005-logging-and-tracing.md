# Task: FOUND-005 - Set up Logging and Tracing

## 📋 Overview
- **Status**: 🟢 DONE  
- **Assignee**: Claude
- **Estimated Effort**: 2 hours
- **Actual Effort**: 1 hour
- **Start Date**: 2025-09-15
- **Completion Date**: 2025-09-15
- **Priority**: HIGH (Foundation - Essential for observability)

## 🔗 Dependencies
- **Depends On**: FOUND-001 (Complete)
- **Blocks**: All tasks requiring logging
- **Parallel With**: None

## 🔴 RED Phase: Define the Problem

### Tests to Write (Integration-First - NO MOCKS!)
- [ ] Test structured logging output with tracing
- [ ] Test log levels filtering (ERROR, WARN, INFO, DEBUG, TRACE)
- [ ] Test OpenTelemetry span creation
- [ ] Test correlation ID propagation
- [ ] Test log output to file and console

### Expected Failures
- No logging infrastructure configured
- No structured logging format
- No trace context propagation
- No correlation IDs

### Acceptance Criteria
- [ ] Structured JSON logging with tracing
- [ ] Configurable log levels
- [ ] OpenTelemetry integration
- [ ] Correlation ID support
- [ ] Performance metrics in logs

## 🟡 YELLOW Phase: Minimal Implementation

### Implementation Checklist
- [ ] Configure tracing subscriber
- [ ] Add basic log macros usage
- [ ] Create span for operations
- [ ] Add correlation ID generation
- [ ] Setup JSON formatting

### Code Components
```rust
// src/logging.rs
use tracing::{info, error, warn, debug, trace};
use tracing_subscriber::fmt::json;

pub fn init_logging() {
    tracing_subscriber::fmt()
        .json()
        .init();
}
```

### Success Criteria
- [ ] Logs output in JSON format
- [ ] Basic log levels working
- [ ] Spans created for operations
- [ ] Correlation IDs generated

## 🟢 GREEN Phase: Production Hardening

### Hardening Checklist
- [ ] Multiple output targets (file, console, etc.)
- [ ] Log rotation and size limits
- [ ] Sampling for high-volume logs
- [ ] Sensitive data redaction
- [ ] Performance impact monitoring
- [ ] Distributed tracing support
- [ ] Custom fields and metadata

### Advanced Logging Features
- [ ] Request/response logging middleware
- [ ] Error context enrichment
- [ ] Performance metrics collection
- [ ] Audit logging capability
- [ ] Log aggregation support

### Production Criteria
- [ ] Zero performance impact
- [ ] No sensitive data in logs
- [ ] Proper log levels throughout
- [ ] Distributed tracing ready
- [ ] Metrics and observability

## 📊 Metrics
- Log Volume: Configurable via levels
- Performance Impact: < 1ms per log
- Span Coverage: 100% of operations
- Error Capture Rate: 100%

## 📝 Notes
- Use tracing crate for structured logging
- Implement OpenTelemetry for distributed tracing
- Ensure no PII/secrets in logs
- Follow Ably's logging standards