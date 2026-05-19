# From The Hart Storage

Rust microservice for file storage, metadata extraction, and S3/DynamoDB operations.

## Architecture

- **HTTP API** (Axum) — file metadata, folder listing, CloudFront signed URL generation
- **SQS Worker** (Lambda) — S3 upload events → EXIF/image metadata extraction → DynamoDB indexing
- **Two deployment targets** — Lambda (serverless) and Docker (local/ECS)
- **OpenAPI/Swagger** docs auto-generated from schemas via [Aide](https://github.com/tamasfe/aide)

## Tech Stack

- **Language:** Rust 2024 edition (pinned via `rust-toolchain.toml` to `1.95.0`)
- **HTTP:** Axum + Tower (`TraceLayer`)
- **AWS SDK:** `aws-sdk-s3`, `aws-sdk-dynamodb`, `aws-sdk-ssm`
- **Lambda:** `lambda_http` (HTTP), `lambda_runtime` + `aws_lambda_events` (SQS)
- **Image:** `imagesize`, `kamadak-exif` (SQS feature)
- **API Docs:** Aide with OpenAPI + Swagger UI
- **Logging:** `tracing` + `tracing-subscriber` (JSON, file+line, thread IDs, `tracing-error`)
- **Error:** `thiserror` (domain errors), `anyhow` (internal)

## Quick Start

```bash
# Install Rust (pinned version)
rustup update 1.95.0

# Local dev
cp .env.example .env  # set APP_ENVIRONMENT=local
RUST_LOG=info APP_ENVIRONMENT=local cargo run

# Tests + lint
cargo test --all-targets
cargo clippy --all-targets --all-features
cargo fmt --check
```

## Common Commands

```bash
cargo build                          # Debug
cargo build --release                # Production (opt-level=z, lto=fat, strip, panic=abort)
cargo build --profile release-dev --features http  # Fast CI build for Lambda
cargo build --features http          # HTTP only
cargo build --features sqs           # SQS only
cargo test --all-targets             # All tests (unit + doc)
cargo clippy --all-targets --all-features  # Lint
cargo fmt --check                    # Format check (CI)
```

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/storage/health` | Health check (`{ status, uptime, timestamp }`) |
| `GET` | `/storage/{user_id}/{path}` | File metadata (no trailing slash) or folder listing (trailing slash) |
| `POST` | `/storage/folders` | Create a new folder |
| `POST` | `/storage/access` | Get signed CloudFront URL for secure file access |
| `GET` | `/storage/documentation/swagger` | Swagger UI |
| `GET` | `/storage/documentation/openapi.json` | OpenAPI 3.0 spec |

## Build Profiles

| Profile | Use | LTO | Codegen Units | Build Time |
|---------|-----|-----|---------------|------------|
| `debug` | Local dev | off | default | Fast |
| `release` | Production Docker | fat, 1 unit | 1 | ~20 min |
| `release-dev` | Lambda CI builds | thin, 8 units | 8 | ~8 min |

`release-dev` inherits from `release` with `lto = "thin"` and `codegen-units = 8` for fast CI. Lambda Dockerfiles use this profile.

## Docker

| Dockerfile | Target | Binary | Build Profile |
|------------|--------|--------|---------------|
| `Dockerfile` | Local/ECS/Cloud Run | `from-the-hart-storage` (main.rs) | `release` |
| `Dockerfile.lambda.http` | Lambda Function URL | `bootstrap_http` | `release-dev` |
| `Dockerfile.lambda.sqs` | Lambda SQS trigger | `bootstrap_sqs` | `release-dev` |

```bash
docker build -t storage .                      # Local/ECS
docker build -f Dockerfile.lambda.http -t storage-http .   # Lambda HTTP
docker build -f Dockerfile.lambda.sqs -t storage-sqs .     # Lambda SQS
```

## Feature Flags

- `http` — Axum HTTP server + OpenAPI docs + CloudFront signer
- `sqs` — Lambda SQS handler + S3 repo + metadata service
- Default: both enabled
- Lambda builds: `--no-default-features --features http` or `--features sqs`

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `APP_ENVIRONMENT` | Yes | `local`, `dev`, or `production` |
| `APP_SERVER_HOST` | No | Bind host (default: `127.0.0.1`) |
| `APP_SERVER_PORT` | No | Bind port (default: `8080`) |
| `APP_CLOUDFRONT_DOMAIN` | No | CloudFront domain for signed URLs |
| `APP_CLOUDFRONT_KEY_PAIR_ID` | No | CloudFront key pair ID |
| `APP_TIMEZONE` | No | IANA timezone (default: UTC) |
| `AWS_*` | — | Standard AWS SDK env vars |
| `RUST_LOG` | No | Tracing filter (default: `info`) |

## Development

See [AGENTS.md](./AGENTS.md) for code conventions, boundaries, and patterns used in this project. See [master AGENTS.md](../AGENTS.md) for architecture principles and universal standards.
