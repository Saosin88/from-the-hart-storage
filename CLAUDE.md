# CLAUDE.md - From The Hart Storage Service

This file provides guidance to Claude Code when working with the Storage Service in this repository.

## Service Overview

The **From The Hart Storage Service** is a Rust-based microservice that handles file storage, metadata extraction, and S3/DynamoDB operations for the From The Hart ecosystem. It supports both HTTP endpoints and AWS Lambda handlers through conditional compilation features.

### Key Responsibilities
- RESTful API for file upload, download, and metadata management
- Image processing and metadata extraction (EXIF data, dimensions)
- Integration with AWS S3 for file storage
- Integration with AWS DynamoDB for metadata indexing
- Health monitoring and graceful shutdown handling
- OpenAPI/Swagger documentation

### Deployment Targets
- **HTTP Mode**: Traditional Axum server (Cloud Run, self-hosted)
- **Lambda HTTP**: AWS Lambda with HTTP handler
- **Lambda SQS**: AWS Lambda with SQS event handler for async image processing
- **Default**: Both HTTP and SQS features enabled

## Technology Stack

### Core Technologies
- **Language**: Rust (2021 edition)
- **HTTP Framework**: Axum with Tower middleware
- **AWS Integration**: aws-sdk-s3, aws-sdk-dynamodb, lambda_http, lambda_runtime
- **Image Processing**: imagesize, kamadak-exif for metadata extraction
- **API Documentation**: Aide with OpenAPI/Swagger support
- **Logging**: tracing with structured JSON output
- **Configuration**: config crate with environment variable support
- **Error Handling**: thiserror, anyhow for rich error types

### Development Tools
- **Testing**: axum-test for integration testing
- **Build Optimization**: Release profile with LTO, size optimization, and stripping for Lambda

## Project Structure

```
from-the-hart-storage/
├── src/
│   ├── lib.rs                      # Library root, feature-gated exports
│   ├── config.rs                   # Configuration management
│   ├── logging.rs                  # Logging setup and structured output
│   ├── error.rs                    # Error types and handling
│   ├── service/
│   │   ├── health.rs               # Health check endpoint
│   │   ├── models.rs               # Data models and schemas
│   │   ├── file/
│   │   │   ├── handler.rs          # File operation handlers
│   │   │   ├── validator.rs        # File validation logic
│   │   │   └── utils.rs            # File utility functions
│   │   └── metadata/
│   │       ├── extractor.rs        # Metadata extraction logic
│   │       ├── image.rs            # Image-specific metadata
│   │       └── models.rs           # Metadata data structures
│   ├── repository/
│   │   ├── s3.rs                   # S3 operations (get, put, delete)
│   │   ├── dynamodb.rs             # DynamoDB metadata operations
│   │   ├── utils.rs                # Repository utilities
│   │   └── mock.rs                 # Mock implementation for testing
│   ├── http/
│   │   ├── handler.rs              # HTTP request handlers (feature-gated)
│   │   ├── routes.rs               # Route definitions
│   │   └── middleware.rs           # Request/response middleware
│   ├── lambda/
│   │   ├── http.rs                 # Lambda HTTP handler (feature-gated)
│   │   └── sqs.rs                  # Lambda SQS handler (feature-gated)
│   └── main.rs / bin/                # Entry points for different features
├── Cargo.toml                      # Project manifest with feature flags
├── Cargo.lock                      # Dependency lock file
├── .env                            # Local environment variables
├── Dockerfile                      # Regular container build
├── Dockerfile.lambda.http          # Lambda HTTP container build
├── Dockerfile.lambda.sqs           # Lambda SQS container build
├── README.md                       # Service documentation
└── terraform/                      # Infrastructure as Code
    └── (dev/prod configs)          # Environment-specific resources
```

## Common Commands

### Building
```bash
# Debug build
cargo build

# Release build (optimized for Lambda)
cargo build --release

# Build specific features
cargo build --features http          # HTTP only
cargo build --features sqs           # SQS only
cargo build --features http,sqs      # Both (default)

# Build for a specific target
cargo build --target x86_64-unknown-linux-gnu
```

### Running
```bash
# Local development with environment configuration
RUST_LOG=info APP_ENVIRONMENT=local APP_SERVER_HOST=127.0.0.1 APP_SERVER_PORT=8080 cargo run

# Production-like environment
RUST_LOG=info APP_ENVIRONMENT=production APP_SERVER_HOST=0.0.0.0 APP_SERVER_PORT=8080 cargo run

# With specific features
cargo run --features http

# Send shutdown signal to test graceful shutdown
kill -TERM <pid>
```

### Testing
```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run tests with logging
RUST_LOG=debug cargo test

# Run specific test module
cargo test repository::

# Watch mode (requires cargo-watch)
cargo watch -x test
```

### Formatting and Linting
```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Lint with Clippy
cargo clippy

# Strict lint checks
cargo clippy -- -W clippy::all

# Fix common issues automatically
cargo clippy --fix --allow-staged
```

### Docker Operations
```bash
# Build Docker image for HTTP handler
docker build -f Dockerfile.lambda.http -t from-the-hart-storage:http .

# Build Docker image for SQS handler
docker build -f Dockerfile.lambda.sqs -t from-the-hart-storage:sqs .

# Build regular image
docker build -t from-the-hart-storage:latest .

# Run container with environment
docker run -e RUST_LOG=info -e APP_ENVIRONMENT=local \
  -e AWS_REGION=us-east-1 \
  -p 8080:8080 from-the-hart-storage:latest
```

## Environment Variables

### Required Configuration
- `APP_ENVIRONMENT` - Execution environment (local, development, production)
- `APP_SERVER_HOST` - Server bind address (default: 0.0.0.0)
- `APP_SERVER_PORT` - Server port (default: 8080)
- `APP_TIMEZONE` - Timezone for timestamps (e.g., Africa/Johannesburg)

### AWS Configuration
- `AWS_REGION` - AWS region (e.g., us-east-1, af-south-1)
- `AWS_ACCESS_KEY_ID` - AWS access key (local only, use IAM roles in production)
- `AWS_SECRET_ACCESS_KEY` - AWS secret key (local only, use IAM roles in production)
- `APP_DYNAMODB_TABLE` - DynamoDB table name for metadata
- `APP_S3_BUCKET` - S3 bucket name for file storage

### Logging Configuration
- `RUST_LOG` - Log level filter (trace, debug, info, warn, error)
  - Example: `RUST_LOG=info` or `RUST_LOG=from_the_hart_storage=debug`

### Optional Configuration
- `APP_MAX_FILE_SIZE` - Maximum file size in bytes
- `APP_ALLOWED_MIME_TYPES` - Comma-separated list of allowed MIME types
- `LOG_FORMAT` - Log output format (json for structured, text for pretty)

## Local Development Setup

### Prerequisites
- Rust 1.70+ (install from https://rustup.rs/)
- AWS CLI configured with credentials
- Docker (optional, for containerized development)
- DynamoDB Local (for local testing without AWS)

### Initial Setup
```bash
# Clone and navigate
cd from-the-hart-storage

# Install dependencies (Cargo handles this automatically)
# No additional npm/pip install needed

# Copy environment template
cp .env.example .env

# Update .env with local values:
# - AWS_REGION=us-east-1
# - APP_DYNAMODB_TABLE=from-the-hart-storage-local
# - APP_S3_BUCKET=your-local-bucket-name
```

### Running Locally
```bash
# Start the service
RUST_LOG=debug cargo run

# In another terminal, test health endpoint
curl http://localhost:8080/health

# Test file upload
curl -F "file=@test.txt" http://localhost:8080/api/v1/files/upload

# View OpenAPI documentation
# Navigate to http://localhost:8080/api/docs (if HTTP feature enabled)
```

### Local AWS Testing with DynamoDB
```bash
# Start DynamoDB Local (requires Docker)
docker run -p 8000:8000 amazon/dynamodb-local

# Configure AWS CLI for local DynamoDB
export AWS_ENDPOINT_URL=http://localhost:8000

# Create test table
aws dynamodb create-table \
  --table-name from-the-hart-storage-local \
  --attribute-definitions AttributeName=file_id,AttributeType=S \
  --key-schema AttributeName=file_id,KeyType=HASH \
  --billing-mode PAY_PER_REQUEST \
  --endpoint-url http://localhost:8000
```

## Testing Approach

### Unit Tests
Tests are organized by module and use mocked dependencies:

```bash
# Test storage repository
cargo test repository::

# Test metadata extraction
cargo test metadata::

# Test file validation
cargo test file::
```

### Integration Tests
Located in `tests/` directory, testing complete request/response flows:

```bash
# Run integration tests
cargo test --test '*'

# Run specific integration test
cargo test --test integration_tests upload
```

### Using axum-test
The project uses `axum-test` for HTTP integration testing:

```rust
// Example test pattern
#[tokio::test]
async fn test_file_upload() {
    let app = create_app().await;
    let response = TestClient::new(app)
        .post("/api/v1/files/upload")
        .body("test content")
        .await;

    assert_eq!(response.status(), StatusCode::OK);
}
```

### Test Coverage
```bash
# Generate coverage report (requires tarpaulin)
cargo tarpaulin --out Html --output-dir coverage
```

## Deployment Information

### Feature-Based Deployment

The service uses Cargo features for conditional compilation, enabling different deployment strategies:

**HTTP Feature** (`cargo build --features http`)
- Enables Axum HTTP server
- Enables aide/OpenAPI documentation
- Output: Traditional executable for servers

**SQS Feature** (`cargo build --features sqs`)
- Enables Lambda SQS event handlers
- Enables image processing pipeline
- Output: Lambda handler for async processing

**Default** (both features)
- Includes HTTP and SQS capabilities
- Single binary supports multiple deployment modes

### Docker Deployment Strategies

**Regular Deployment (Dockerfile)**
```dockerfile
# Multi-stage build for smaller images
# Final stage includes only runtime dependencies
# Optimized for Cloud Run, Kubernetes, etc.
```

**Lambda HTTP (Dockerfile.lambda.http)**
```dockerfile
# RIL (Runtime Interface Library) compatible
# Includes lambda_http runtime
# Optimized for Lambda HTTP handler
```

**Lambda SQS (Dockerfile.lambda.sqs)**
```dockerfile
# RIL compatible for async processing
# Includes lambda_runtime for SQS events
# Used for image processing pipeline
```

### Production Deployment Checklist
- [ ] Run `cargo clippy` for code quality checks
- [ ] Run tests with `cargo test --release`
- [ ] Build with `cargo build --release` for optimization
- [ ] Verify Docker build completes successfully
- [ ] Test graceful shutdown with SIGTERM
- [ ] Verify logging output format (structured JSON)
- [ ] Check AWS IAM roles are properly configured
- [ ] Validate DynamoDB table exists and is accessible
- [ ] Confirm S3 bucket permissions are correct

### Scaling Considerations
- **Horizontal**: Lambda scales automatically; no special configuration needed
- **Vertical**: Increase Lambda memory to improve CPU allocation
- **Caching**: Configure CloudFront caching for frequently accessed files
- **DynamoDB**: Use on-demand billing for variable workloads, or provisioned capacity for predictable patterns

## Claude Code Integration Notes

### .claude Directory
Commands and context files are located in `.claude/commands/`:
- Custom slash commands for common operations
- Integration with Claude Code agents

### Useful Patterns for Agent Work

**Analyzing Rust Code**
- Use `grep` tool to search across .rs files
- Look for `Feature cfg` attributes to understand conditional compilation
- Check error types in `src/error.rs` for custom error handling

**Understanding Dependencies**
- Check `Cargo.toml` for feature flags and versions
- Reference `Cargo.lock` for exact dependency versions
- Search for `use` statements to trace module imports

**Investigating Issues**
- Check `src/logging.rs` for logging configuration
- Look at environment variable parsing in `src/config.rs`
- Review AWS SDK call sites in `src/repository/` for API interactions

### Common Claude Code Tasks

**Modifying Feature Flags**
When adding/removing features:
1. Update `[features]` section in Cargo.toml
2. Use `#[cfg(feature = "...")]` for conditional compilation
3. Test with `cargo build --features feature-name`

**Adding New Endpoints (HTTP Feature)**
1. Create handler in `src/http/handler.rs`
2. Define route in `src/http/routes.rs`
3. Add TypeBox schema for request/response
4. Document with aide OpenAPI attributes
5. Write tests in `tests/`

**AWS Integration Changes**
1. Check existing SDK usage in `src/repository/`
2. Review error handling patterns
3. Test with local AWS services or mocks
4. Verify IAM permissions are documented

### Debugging Tips
- Enable debug logging: `RUST_LOG=from_the_hart_storage=debug`
- Use `dbg!()` macro for quick debugging (remove before commit)
- Check Docker build logs: `docker build -t test . 2>&1 | grep -i error`
- Test Lambda locally with SAM CLI or AWS Lambda emulator

## Key Files Reference

| File | Purpose |
|------|---------|
| `src/lib.rs` | Library root, re-exports for different features |
| `src/config.rs` | Configuration loading and validation |
| `src/logging.rs` | Structured logging setup |
| `src/error.rs` | Custom error types and error handling |
| `src/repository/s3.rs` | S3 operations (CRUD) |
| `src/repository/dynamodb.rs` | DynamoDB metadata operations |
| `src/service/metadata/extractor.rs` | Image metadata extraction logic |
| `src/http/routes.rs` | HTTP route definitions and handlers |
| `Cargo.toml` | Dependencies and feature flags |
| `Dockerfile.lambda.*` | Lambda-specific container builds |

## Related Services

This service integrates with:
- **Auth Service** (`from-the-hart-auth`): Token validation for authenticated operations
- **API Reverse Proxy** (`from-the-hart-tech-api-reverse-proxy-worker`): Routes requests from Cloudflare
- **Infrastructure** (`from-the-hart-infrastructure`): Terraform configs for S3, DynamoDB, IAM
