# File Sharing Service - Implementation Guide

## Overview

This document provides implementation guidance for the DynamoDB-based file sharing service. The service implements a single-table design with S3-style prefix-based folders, prefix-level grants, and denormalized VIEW_LINK items for efficient merged folder views.

## Architecture Components

### 1. Data Models

The service uses three primary item types stored in a single DynamoDB table:

#### FileItem
Represents a file metadata record owned by a user.

```rust
pub struct FileItem {
    pub pk: String,              // USER#<OwnerID>
    pub sk: String,              // FILE#<Path>
    pub file_id: String,         // Unique identifier
    pub owner_id: String,        // Owner user ID
    pub file_name: String,       // File name only
    pub folder_prefix: String,   // S3-style folder prefix
    pub created_date: i64,       // Unix timestamp (ms)
    pub media_type: String,      // MIME type
    pub s3_key: String,          // S3 object key
    pub size: i64,               // File size in bytes
    pub content_type: String,    // Content type
}
```

#### ShareGrantItem
Represents a prefix-level sharing permission.

```rust
pub struct ShareGrantItem {
    pub pk: String,              // USER#<OwnerID>
    pub sk: String,              // GRANT#<RecipientID>#<GrantID>
    pub grant_id: String,        // Unique grant ID (UUID)
    pub owner_id: String,        // Who is sharing
    pub recipient_id: String,    // Who receives access
    pub permissions: String,     // READ or READ/WRITE
    pub prefix: String,          // Folder prefix being shared
    pub gsi1_pk: String,         // ACCESS#<RecipientID>
    pub gsi1_sk: String,         // GRANT#<OwnerID>#<Prefix>
}
```

#### ViewLinkItem
Denormalized link for efficient merged folder views.

```rust
pub struct ViewLinkItem {
    pub pk: String,              // USER#<ViewerID>
    pub sk: String,              // VIEWLINK#<OwnerID>#<FileID>
    pub file_id: String,         // File identifier
    pub owner_id: String,        // File owner
    pub grant_id: String,        // Grant ID or "OWNER"
    pub folder_name: String,     // Normalized folder name
    pub media_type: String,      // For filtering
    pub gsi2_pk: String,         // VIEWER#<ViewerID>#FOLDER#<FolderName>
    pub gsi2_sk: String,         // <MediaType>#<CreatedDate>#<FileID>
}
```

### 2. DynamoDB Table Schema

**Table Name:** `FileMetadata-dev` (environment-specific)

**Primary Keys:**
- Partition Key (PK): String - `USER#<UserID>`
- Sort Key (SK): String - `FILE#<Path>`, `GRANT#<RecipientID>#<GrantID>`, or `VIEWLINK#<OwnerID>#<FileID>`

**Global Secondary Indexes:**

1. **ShareAccessIndex (GSI1)**
   - Purpose: List all folders shared with a specific user
   - Partition Key: `GSI1-PK` = `ACCESS#<RecipientID>`
   - Sort Key: `GSI1-SK` = `GRANT#<OwnerID>#<Prefix>`

2. **MergedFolderViewIndex (GSI2)**
   - Purpose: Merged, filterable view of folder contents from multiple owners
   - Partition Key: `GSI2-PK` = `VIEWER#<ViewerID>#FOLDER#<FolderName>`
   - Sort Key: `GSI2-SK` = `<MediaType>#<CreatedDate>#<FileID>`

### 3. Infrastructure Components

The Terraform module (`file_sharing_dynamodb`) creates:

- DynamoDB table with GSIs and streams enabled
- SQS queue for async VIEW_LINK cleanup operations
- Dead letter queue for failed operations
- CloudWatch log groups for monitoring
- IAM roles and policies for Lambda functions (when enabled)

## Usage Examples

### Creating a File Record

```rust
use from_the_hart_storage::service::file_sharing::*;

let repository = FileShareRepository::new(dynamodb_client, "FileMetadata-dev".to_string());
let service = FileShareService::new(repository);

service.create_file(
    "Sheldon".to_string(),
    "media/photos/vacation.jpg".to_string(),
    "R102".to_string(),
    "vacation.jpg".to_string(),
    "media/photos/".to_string(),
    "Sheldon/media/photos/vacation.jpg".to_string(),
    "image/jpeg".to_string(),
    "image/jpeg".to_string(),
    161713,
).await?;
```

### Granting Access to a Folder

```rust
let grant_id = service.create_share(
    "Sheldon".to_string(),      // Owner
    "Justin".to_string(),       // Recipient
    "media/photos/".to_string(), // Prefix
    "READ".to_string(),         // Permissions
).await?;

println!("Created grant: {}", grant_id);
```

### Listing Shared Folders

```rust
let shared_folders = service.list_shared_folders("Justin").await?;

for folder in shared_folders {
    println!("Owner: {}, Prefix: {}, Permissions: {}",
        folder.owner_id,
        folder.prefix,
        folder.permissions
    );
}
```

### Querying Merged Folder View

```rust
// Get all files in "photos/" folder from all shared sources
let (view_links, next_cursor) = service.get_merged_folder_view(
    "Justin",           // Viewer
    "photos/",          // Folder name
    None,               // No media type filter
    Some(20),           // Limit
    None,               // No cursor (first page)
).await?;

// Get only images
let (images, next_cursor) = service.get_merged_folder_view(
    "Justin",
    "photos/",
    Some("image/"),     // Filter by media type
    Some(20),
    None,
).await?;
```

### Revoking Access

```rust
service.revoke_share(
    "Sheldon",    // Owner
    "Justin",     // Recipient
    &grant_id,    // Grant ID
).await?;
```

## Key Design Decisions

### S3-Style Prefix-Based Folders
- No explicit folder items in DynamoDB
- Folders are treated as prefixes (e.g., `media/photos/`)
- Simplifies operations and eliminates folder hierarchy maintenance

### Prefix-Level Grants
- Single grant applies to all files under a prefix
- Supports nested sub-directories automatically
- No per-file permission management needed

### VIEW_LINK Denormalization
- Enables efficient merged folder views across multiple owners
- Created lazily on first folder access
- Maintained automatically via DynamoDB Streams (when implemented)
- Immediate revocation via SHARE_GRANT validation

### Permission Enforcement
- Permission checks done by verifying SHARE_GRANT existence
- Revocation is immediate, even if VIEW_LINK cleanup is async
- Eventually consistent VIEW_LINK deletion via SQS worker

## Terraform Integration

Add the module to your environment configuration:

```hcl
module "file_sharing" {
  source = "../modules/file_sharing_dynamodb"

  table_name   = "FileMetadata-dev"
  name_prefix  = "my-app-dev"
  environment  = "dev"
  lambda_role_arn = aws_iam_role.lambda_role.arn

  create_stream_processor = false  # Enable when implementing
  create_cleanup_worker   = false  # Enable when implementing

  enable_point_in_time_recovery = true
  log_retention_days            = 14

  tags = {
    Environment = "dev"
    Project     = "my-app"
  }
}
```

Add environment variables to Lambda functions:

```hcl
environment {
  variables = {
    DYNAMODB_TABLE_NAME   = module.file_sharing.table_name
    SQS_CLEANUP_QUEUE_URL = module.file_sharing.cleanup_queue_url
  }
}
```

## Future Enhancements

### DynamoDB Streams Processor
A Lambda function that automatically creates VIEW_LINKs when:
- Owner uploads a new file to a shared folder
- File metadata is updated
- Files are deleted

### SQS Cleanup Worker
A Lambda function that processes VIEW_LINK cleanup operations asynchronously:
- Deletes VIEW_LINKs when grants are revoked
- Handles batch deletions efficiently
- Retries failed operations with dead letter queue

### Additional Features
- File versioning support
- Audit trail for access events
- Advanced permission levels (COMMENT, DOWNLOAD, etc.)
- Expiring shares with TTL
- Share notifications via SNS

## Testing

Run the test suite:

```bash
cargo test --lib service::file_sharing
```

Key test areas:
- Data model serialization/deserialization
- Repository operations with DynamoDB
- Service layer business logic
- GSI query patterns
- Pagination and filtering

## Monitoring

Monitor these CloudWatch metrics:
- DynamoDB table read/write capacity
- GSI query performance
- Lambda function invocations and errors
- SQS queue depth and age
- DLQ message count

## Security Considerations

1. **IAM Policies**: Lambda functions need permissions for:
   - DynamoDB: `GetItem`, `PutItem`, `Query`, `BatchWriteItem`, `DeleteItem`
   - SQS: `ReceiveMessage`, `DeleteMessage`, `SendMessage`
   - CloudWatch: Logging permissions

2. **Encryption**:
   - Enable encryption at rest for DynamoDB table
   - Enable encryption for SQS queues
   - Use HTTPS for all API calls

3. **Access Control**:
   - Validate user identity before operations
   - Check SHARE_GRANT existence for permission enforcement
   - Log access attempts for audit

## Performance Optimization

1. **Query Patterns**: All common operations use efficient key-based queries
2. **Pagination**: Native DynamoDB pagination with `LastEvaluatedKey`
3. **Batch Operations**: Batch writes for VIEW_LINK creation (25 items per batch)
4. **GSI Design**: Optimized for merged folder views and shared folder listings
5. **Caching**: Consider caching SHARE_GRANT results for frequently accessed folders

## Troubleshooting

### Common Issues

1. **VIEW_LINKS not appearing**: Ensure lazy creation is triggered on first folder access
2. **Permission denied after grant**: Check SHARE_GRANT item exists and matches the prefix
3. **Slow merged folder queries**: Review GSI2 sort key design and pagination limits
4. **Cleanup worker falling behind**: Increase Lambda concurrency or batch size

## References

- [DynamoDB Best Practices](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/best-practices.html)
- [Single-Table Design](https://aws.amazon.com/blogs/compute/creating-a-single-table-design-with-amazon-dynamodb/)
- [DynamoDB Streams](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/Streams.html)
