# From The Hart Storage Service

Rust-based microservice handling file storage, metadata extraction, and S3/DynamoDB operations.

## Overview

**Responsibilities:**
- RESTful API for file upload, download, metadata management
- Image processing and EXIF metadata extraction
- AWS S3 file storage + DynamoDB metadata indexing
- OpenAPI/Swagger documentation

**Deployment:** AWS Lambda (HTTP + SQS handlers), Cloud Run compatible

## Technology Stack

- **Language:** Rust 2021
- **HTTP:** Axum with Tower middleware
- **AWS:** aws-sdk-s3, aws-sdk-dynamodb, lambda_http, lambda_runtime
- **Image:** imagesize, kamadak-exif
- **API Docs:** Aide with OpenAPI
- **Logging:** tracing with JSON output

## Common Commands

```bash
# Build
cargo build                          # Debug build
cargo build --release                # Release build
cargo build --features http          # HTTP only
cargo build --features sqs           # SQS only

# Run
RUST_LOG=info APP_ENVIRONMENT=local cargo run

# Test
cargo test
cargo test -- --nocapture            # With output

# Format & Lint
cargo fmt
cargo clippy
```

## Docker

```bash
# Build Lambda images
docker build -f Dockerfile.lambda.http -t storage:http .
docker build -f Dockerfile.lambda.sqs -t storage:sqs .

# Run
docker run -e RUST_LOG=info -e AWS_REGION=us-east-1 -p 8080:8080 storage:http
```

## Environment Variables

**Required:**
- `APP_ENVIRONMENT` - local, development, production
- `APP_SERVER_HOST` - default: 0.0.0.0
- `APP_SERVER_PORT` - default: 8080
- `AWS_REGION` - e.g., us-east-1
- `APP_DYNAMODB_TABLE` - DynamoDB table name
- `APP_S3_BUCKET` - S3 bucket name

**Logging:**
- `RUST_LOG` - trace, debug, info, warn, error

## Feature Flags

- `http` - Axum HTTP server + OpenAPI docs
- `sqs` - Lambda SQS handlers + image processing
- Default: both enabled

## Local Development

```bash
# Install dependencies (Cargo handles automatically)
cargo build

# Copy environment template
cp .env.example .env

# Start service
RUST_LOG=debug cargo run

# Test endpoints
curl http://localhost:8080/health
```

## Testing

```bash
cargo test                           # All tests
cargo test repository::              # Specific module
RUST_LOG=debug cargo test            # With logging
```

## Related Services

- **Auth Service:** Token validation for authenticated operations
- **API Reverse Proxy:** Routes requests from Cloudflare
- **Infrastructure:** Terraform configs for S3, DynamoDB, IAM
