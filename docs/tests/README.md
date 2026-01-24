# Testing Documentation

This directory contains testing-related documentation including test plans, test reports, and testing strategies.

## 📋 Test Documentation

### Integration Test Plans

#### AgentFS Integration Tests
- **[2025-01-18-agentfs-docker-integration-test-plan.md](./2025-01-18-agentfs-docker-integration-test-plan.md)** - Docker integration test plan for AgentFS
- **[2025-01-18-agentfs-docker-integration-test-results.md](./2025-01-18-agentfs-docker-integration-test-results.md)** - Docker integration test results
- **[2026-01-19-agentfs-container-integration-test-design.md](./2026-01-19-agentfs-container-integration-test-design.md)** - Container integration test design
- **[2026-01-19-agentfs-container-integration-test-results.md](./2026-01-19-agentfs-container-integration-test-results.md)** - Container integration test results
- **[2025-01-18-agentfs-container-session-test-report.md](./2025-01-18-agentfs-container-session-test-report.md)** - Container session test report

### Capture Testing
- **[opencode-capture-test-report.md](./opencode-capture-test-report.md)** - OpenCode message capture testing report

## 🧪 Testing Strategy

### Test Types

VibeRepo follows a comprehensive testing approach:

1. **Unit Tests** - Located in `backend/src/` with `#[cfg(test)]` modules
2. **Integration Tests** - Located in `backend/tests/` directory
3. **Property Tests** - Using `proptest` crate for property-based testing

### Test Coverage (v0.3.0)

- **Total tests**: 589+
- **Passing**: 100%
- **Unit tests**: 375+
- **Integration tests**: 214+

**Test Categories:**
- Task management: 50+ tests
- Execution engine: 10+ tests
- Failure analysis: 4 tests
- Scheduler: 7 tests
- Concurrency control: 6 tests
- WebSocket logs: 4 tests

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output visible
cargo test -- --nocapture

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test '*'
```

## 📝 Test Documentation Guidelines

### Creating Test Plans

When creating a new test plan:

1. Use date prefix: `YYYY-MM-DD-feature-test-plan.md`
2. Include:
   - Test objectives
   - Test scope and coverage
   - Test environment setup
   - Test cases with expected results
   - Success criteria
3. Update this README with a link to the new plan

### Writing Test Reports

Test reports should include:

1. Date prefix: `YYYY-MM-DD-feature-test-report.md` or `YYYY-MM-DD-feature-test-results.md`
2. Content:
   - Test execution summary
   - Test results (pass/fail counts)
   - Issues discovered
   - Performance metrics (if applicable)
   - Recommendations

### Test Naming Conventions

- **Test Plans**: `YYYY-MM-DD-feature-test-plan.md`
- **Test Results**: `YYYY-MM-DD-feature-test-results.md`
- **Test Reports**: `YYYY-MM-DD-feature-test-report.md`
- **Test Design**: `YYYY-MM-DD-feature-test-design.md`

## 🔗 Related Documentation

- **[AGENTS.md](../../AGENTS.md)** - Testing philosophy and TDD workflow
- **[Plans](../plans/)** - Implementation plans that reference these tests
- **[Research](../research/)** - Research findings that inform testing strategies

## 🎯 Testing Best Practices

### Test-Driven Development (TDD)

VibeRepo follows strict TDD:

1. **Red**: Write a failing test first
2. **Green**: Write minimal code to make the test pass
3. **Refactor**: Refactor code while keeping tests passing

### Test Structure

```rust
#[tokio::test]
async fn test_feature_name() {
    // Arrange: Set up test data and environment
    let test_data = create_test_data();
    
    // Act: Execute the functionality being tested
    let result = function_under_test(test_data).await;
    
    // Assert: Verify the results
    assert_eq!(result, expected_value);
}
```

### Test Documentation

Every test should have:
- Clear, descriptive name
- Doc comment explaining what is being tested
- Reference to requirements (if applicable)

Example:
```rust
/// Test GET /health returns 200 when healthy
/// Requirements: 7.1, 7.2
#[tokio::test]
async fn test_health_endpoint_returns_200_when_healthy() {
    // Test implementation
}
```

## 📊 Test Metrics

### Current Coverage

- **API Endpoints**: 100% covered
- **Service Layer**: 95%+ covered
- **Database Operations**: 100% covered
- **Error Handling**: 100% covered

### Performance Benchmarks

Key performance metrics tracked:
- API response times
- Database query performance
- Container startup times
- Task execution duration

## 🔄 Continuous Testing

### Automated Testing

Tests run automatically:
- On every commit (pre-commit hook)
- On pull requests (CI/CD pipeline)
- Before releases

### Test Maintenance

- Review and update tests when features change
- Add tests for bug fixes
- Remove obsolete tests
- Keep test documentation up-to-date

---

**Last Updated:** 2026-01-21  
**Test Coverage:** 327 tests (100% passing)
