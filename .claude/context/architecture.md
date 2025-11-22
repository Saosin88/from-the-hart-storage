# Storage Service Architecture

Last updated: 2025-11-22

## Current State
- Rust service with Axum framework
- Dual Lambda handlers: HTTP (via lambda_http) and SQS (via lambda_runtime)
- Conditional compilation via Cargo features (http, sqs)
- AWS SDK integration (S3, DynamoDB)
- OpenAPI documentation via aide + schemars

## Architectural Issues

### High Priority
1. **[2025-11-22] SQS handler doesn't use AppState pattern**
   - File: `src/bin/bootstrap_sqs.rs`
   - Issue: Inconsistent with HTTP handler, manually passing Arc references
   - Impact: Maintenance burden, pattern inconsistency
   - Status: Not fixed

2. **[2025-11-22] DynamoDB batch processing bug**
   - File: `src/repository/dynamodb.rs` lines 82-101
   - Issue: Chunking at 100 items instead of 25 (DynamoDB limit)
   - Impact: Production failures, partial write inconsistency
   - Status: Not fixed

3. **[2025-11-22] Cursor deserialization security vulnerability**
   - File: `src/repository/dynamodb.rs` lines 125-150
   - Issue: Accepting arbitrary user input without HMAC/encryption
   - Impact: Information disclosure, potential DoS
   - Status: Not fixed

### Medium Priority
4. **[2025-11-22] Global OnceCell SDK clients anti-pattern**
   - Files: `src/repository/s3.rs`, `src/repository/dynamodb.rs`
   - Issue: Using static OnceCell instead of DI through AppState
   - Impact: Testing contamination, configuration lock-in
   - Status: Not fixed

5. **[2025-11-22] Domain error handling with anyhow leakage**
   - File: `src/error.rs`
   - Issue: Using anyhow::Error in domain layer loses type information
   - Impact: Cannot match specific errors without downcasting
   - Status: Not fixed

### Low Priority
6. **[2025-11-22] Value objects using Arc<str> everywhere**
   - File: `src/service/models.rs`
   - Issue: Potential over-optimization, no encapsulation (pub fields)
   - Impact: Heap fragmentation, no invariant protection
   - Status: Not fixed

## Performance Notes
- Lambda cold starts: Not yet measured
- Binary sizes: Not yet measured
- DynamoDB consumed capacity: Not yet monitored

## Testing Coverage
- Unit tests exist but coverage unknown
- Integration tests use axum-test
- Need to verify AWS SDK mocking strategy
