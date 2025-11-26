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
- `APP_SERVER_HOST` - default: 0.0.0.0 (for local dev)
- `APP_SERVER_PORT` - default: 8080 (for local dev)
- `AWS_REGION` - e.g., us-east-1
- `APP_DYNAMODB_TABLE` - DynamoDB table name
- `APP_S3_BUCKET` - S3 bucket name

**CloudFront Signed URLs (Optional):**
- `APP_CLOUDFRONT_KEY_PAIR_ID` - CloudFront public key pair ID
- `APP_CLOUDFRONT_PRIVATE_KEY_SSM_PATH` - SSM parameter path for private key (default: `/from-the-hart-tech-storage/dev/cloudfront-private-key`)
- `APP_CLOUDFRONT_DOMAIN` - CloudFront distribution domain (e.g., `dev-storage.fromthehart.tech`)

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

## Architecture Patterns

### Repository Pattern with Traits

All AWS service interactions use trait-based repositories for testability and flexibility.

**Available Repository Traits:**
- `S3RepositoryTrait` - S3 object operations
- `DynamoDbRepositoryTrait` - DynamoDB CRUD operations
- `SsmRepositoryTrait` - Systems Manager Parameter Store access

**Example: Using SsmRepositoryTrait**

```rust
use crate::repository::{SsmRepositoryTrait, ssm::SsmRepository};
use crate::error::StorageError;

async fn fetch_secret<T: SsmRepositoryTrait>(
    ssm_repo: &T,
    param_path: &str,
) -> Result<String, StorageError> {
    ssm_repo.get_parameter(param_path, true).await
}

let ssm = SsmRepository::new().await;
let secret = fetch_secret(&ssm, "/my-app/secret").await?;
```

**Testing with Mocks:**

```rust
#[cfg(test)]
mod tests {
    use crate::repository::mock::MockSsmRepository;

    #[tokio::test]
    async fn test_fetch_secret_success() {
        let mock_ssm = MockSsmRepository::new()
            .with_get_parameter_response(Ok("test-value".to_string()));

        let result = fetch_secret(&mock_ssm, "/test/param").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test-value");

        let calls = mock_ssm.get_parameter_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "/test/param");
        assert!(calls[0].1);
    }

    #[tokio::test]
    async fn test_fetch_secret_failure() {
        let mock_ssm = MockSsmRepository::new()
            .with_get_parameter_response(Err(
                StorageError::Ssm {
                    context: "Not found".to_string(),
                    source: anyhow::anyhow!("Test error"),
                }
            ));

        let result = fetch_secret(&mock_ssm, "/test/param").await;
        assert!(result.is_err());
    }
}
```

**Real-World Example: CloudFront Signer Initialization**

The `CloudFrontSigner::from_ssm_config()` method demonstrates this pattern:

```rust
pub async fn from_ssm_config<T: SsmRepositoryTrait>(
    ssm_repo: &T
) -> Option<Arc<Self>> {
    let cf_config = config().cloudfront.as_ref()?;

    let private_key_pem = ssm_repo
        .get_parameter(&cf_config.private_key_ssm_path, true)
        .await
        .ok()?;

    Self::new(&private_key_pem, cf_config.key_pair_id.clone(), cf_config.domain.clone())
        .ok()
        .map(Arc::new)
}
```

This pattern allows:
- Easy mocking in unit tests
- Flexible repository implementations
- Clean separation of concerns
- Type-safe async operations

## Known Issues & Technical Debt

### Configuration Loading (src/config.rs)

**NEEDS REFACTORING** - Current implementation uses manual environment variable parsing.

**Problem:**
- Config loading manually parses each environment variable using `std::env::var()`
- Not elegant or maintainable
- Was implemented as quick fix to support single underscore env vars (e.g., `APP_CLOUDFRONT_KEY_PAIR_ID`)
- Original `config` crate implementation required double underscores (`APP_CLOUDFRONT__KEY_PAIR_ID`)

**Current Workaround:**
Manual parsing in `AppConfig::load()` directly reads env vars and constructs nested structs.

**TODO:**
Find a better solution that:
- Supports single underscore naming convention
- Uses a more elegant/maintainable approach
- Possibly uses a different config library or custom derive macro
- Maintains backward compatibility with all existing env vars

**Priority:** Medium - Works but needs improvement for long-term maintainability

### CloudFront Signed URL Generation (src/service/access.rs)

**NEEDS TESTING** - CloudFront signed URL generation currently lacks comprehensive test coverage.

**Problem:**
- `CloudFrontSigner` is a concrete struct, not abstracted behind a trait
- Cannot be easily mocked in tests
- No tests for signed URL generation logic
- HTTP handler tests skip CloudFront signing by passing `None`

**TODO:**
To improve testability:
1. Create `CloudFrontSignerTrait` trait
2. Implement trait for `CloudFrontSigner`
3. Create `MockCloudFrontSigner` for tests
4. Update `AppState` to use `Arc<dyn CloudFrontSignerTrait>`
5. Write comprehensive tests for:
   - URL signing with various expiration times
   - Policy generation
   - Query parameter formatting
   - Cookie generation
   - Edge cases and error conditions

**Priority:** Medium - Functionality works but needs test coverage for production confidence

## Related Services

- **Auth Service:** Token validation for authenticated operations
- **API Reverse Proxy:** Routes requests from Cloudflare
- **Infrastructure:** Terraform configs for S3, DynamoDB, IAM
- Dont use mod.rs for module decleration. See how its done in the rest of the project.