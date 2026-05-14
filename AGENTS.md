# From The Hart Storage

> Rust microservice for file storage, metadata extraction, and S3/DynamoDB operations. WIP. See [master AGENTS.md](../AGENTS.md) for principles.

## Responsibilities

- HTTP API: file/folder listing, CloudFront signed URL generation
- SQS Worker: S3 upload event → EXIF/image metadata extraction → DynamoDB indexing
- OpenAPI/Swagger documentation (Aide)

## Tech Stack

- **Language:** Rust 2021 edition
- **HTTP:** Axum + Tower (`TraceLayer`)
- **AWS SDK:** `aws-sdk-s3`, `aws-sdk-dynamodb`, `aws-sdk-ssm`
- **Lambda:** `lambda_http` (HTTP), `lambda_runtime` + `aws_lambda_events` (SQS)
- **Image:** `imagesize`, `kamadak-exif` (SQS feature)
- **API Docs:** Aide with OpenAPI + Swagger UI
- **Logging:** `tracing` + `tracing-subscriber` (JSON, file+line, thread IDs)
- **Error:** `thiserror` (domain errors), `anyhow` (internal)
- **Config:** `config` crate + `dotenvy`

## Common Commands

```bash
cargo build                          # Debug
cargo build --release                # Release (opt-level=z, lto, strip, panic=abort)
cargo build --features http          # HTTP only
cargo build --features sqs           # SQS only

RUST_LOG=info APP_ENVIRONMENT=local cargo run   # Local

cargo test                           # All tests
cargo fmt                            # Format
cargo clippy                         # Lint
```

## Docker

Three Dockerfiles for three deployment targets:

| Dockerfile | Target | Binary |
|------------|--------|--------|
| `Dockerfile` | Local/ECS/Cloud Run | `from-the-hart-storage` (main.rs) |
| `Dockerfile.lambda.http` | Lambda Function URL | `bootstrap_http` |
| `Dockerfile.lambda.sqs` | Lambda SQS trigger | `bootstrap_sqs` |

## Feature Flags

- `http` — Axum HTTP server + OpenAPI docs + CloudFront signer
- `sqs` — Lambda SQS handler + S3 repo + metadata service
- Default: both enabled
- Lambda builds: `--no-default-features --features http` or `--features sqs`

## Architecture Patterns

### Repository Pattern with Traits

All AWS service interactions use trait-based repositories for testability:

- `DynamoDbRepositoryTrait` — DynamoDB CRUD
- `S3RepositoryTrait` — S3 object operations
- `SsmRepositoryTrait` — Parameter Store access
- Each has real impl + mock impl (`mock.rs`)

### AppState

```rust
pub struct AppState {
    pub s3_repository: Option<Arc<dyn S3RepositoryTrait>>,       // #[cfg(feature = "sqs")]
    pub dynamo_db_repository: Arc<dyn DynamoDbRepositoryTrait>,
    pub metadata_service: Option<Arc<dyn MetadataServiceTrait>>,  // #[cfg(feature = "sqs")]
    pub cloudfront_signer: Option<Arc<CloudFrontSigner>>,         // #[cfg(feature = "http")]
}
```

Feature-gated fields allow single `AppState` used by both HTTP and SQS binaries.

### Module Convention

**Do NOT use `mod.rs`.** Use 2018 edition style:

```
src/
├── lib.rs               # pub mod handler; pub mod repository; ...
├── handler.rs            # pub mod http; pub mod sqs;
├── handler/
│   ├── http.rs           # pub mod storage;
│   ├── http/
│   │   └── storage.rs    # pub mod access; pub mod folder; ...
│   │   └── storage/
│   │       ├── access.rs
│   │       ├── folder.rs
│   │       └── list.rs
│   └── sqs.rs
│       └── sqs/
│           └── worker.rs
├── repository.rs         # pub mod dynamodb; pub mod s3; pub trait DynamoDbRepositoryTrait; ...
├── repository/
│   ├── mock.rs
│   ├── dynamodb.rs
│   ├── s3.rs
│   └── ssm.rs
└── service.rs            # pub mod access; pub mod file; pub mod models; ...
```

Pattern: `foo.rs` re-exports submodules from `foo/` directory. No `mod.rs` files.

## Key Files

```
src/
├── main.rs               # Local Docker entry (Tokio + TcpListener)
├── lib.rs                # Module declarations
├── config.rs             # Env var loading via OnceLock singleton
├── state.rs              # AppState with feature-gated fields
├── error.rs              # thiserror StorageError enum
├── logging.rs            # tracing-subscriber init (JSON, panic hook)
├── bin/
│   ├── bootstrap_http.rs # Lambda HTTP entry
│   └── bootstrap_sqs.rs  # Lambda SQS entry
├── handler/
│   ├── http/routes.rs    # Axum router + Aide OpenAPI docs
│   └── sqs/worker.rs     # SQS event handler
├── repository/           # Trait definitions + impls + mocks
├── service/              # Business logic
└── utils/                # gps, jwt, string, time helpers
```
