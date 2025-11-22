# Storage Service Decisions

Last updated: 2025-11-22

## Decisions Made

### [2025-11-22] Adopted agent-based development workflow
- Storage-Architect, Storage-Analyst, Storage-Developer, Storage-QA
- Enables persistent role-based perspectives across sessions

### [2025-11-22] Using slash commands for operations
- `/test` - Run cargo tests
- `/build` - Build debug binary
- `/build-lambda` - Build optimized Lambda binary
- `/check` - Run cargo clippy

## Pending Decisions

### Error Handling Strategy
- Should we replace anyhow with custom error types in domain layer?
- Trade-offs: Type safety vs ergonomics

### Cursor Pagination Security
- HMAC vs encryption for cursor tokens?
- Which key rotation strategy?

### Dependency Injection Pattern
- Full AppState DI vs global OnceCell?
- Performance implications for Lambda cold starts?
