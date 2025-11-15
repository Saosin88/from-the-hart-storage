_This immersive artifact is rendered using Gemini AST._

# **File Sharing Service - DynamoDB Technical Specification**

## **1. Architecture Overview and Principles**

This document specifies the DynamoDB schema for a highly scalable file metadata service, utilizing a **Single-Table Design** to manage file hierarchy, ownership, and complex sharing relationships with maximum efficiency. This design is intended to power an application with functionality similar to Google Drive or Google Photos.

### **1.1. Core Architectural Principle**

All file metadata, sharing permissions, and denormalized links for different views are stored in a single DynamoDB table. The table is primarily partitioned by the user's ID (`USER#<UserID>`), which provides extreme data locality for the most common access patterns (a user interacting with their own items).

**Key Design Decisions:**

- **S3-Style Prefix-Based Folders:** There are NO explicit folder items stored in DynamoDB. Folders are treated as S3-style prefixes (e.g., `media/photos/2024/`). This eliminates the need to maintain folder hierarchy items and simplifies operations.
- **Prefix-Level Grants:** Access is granted at the prefix level, not per-file. A single `SHARE_GRANT` for `media/photos/` grants access to ALL files matching that prefix, including nested sub-directories.
- **VIEW_LINK Denormalization:** To enable efficient merged folder views (e.g., showing all "photos/" folders from multiple users in a single, sorted, pageable list), we create denormalized `VIEW_LINK` items. These are created lazily on first folder access and maintained automatically via DynamoDB Streams.
- **Immediate Revocation via SHARE_GRANT Validation:** Permission enforcement is done by checking the existence of the `SHARE_GRANT` item. This means revocation is immediate, even if `VIEW_LINK` cleanup is asynchronous.

**Operational Context:**

- **Users for Examples:** Sheldon, Leigh, and Justin.
- **Table Name:** `FileMetadata`
- **Billing Mode:** Pay-Per-Request (On-Demand), ideal for unpredictable workloads.
- **Supporting Infrastructure:** SQS queue for async cleanup, DynamoDB Streams + Lambda for automatic VIEW_LINK maintenance.

### **1.2. Schema Keys**

| Key Type               | Attribute Name | Data Type  | Purpose                                                                                   |
| :--------------------- | :------------- | :--------- | :---------------------------------------------------------------------------------------- |
| **Partition Key (PK)** | `PK`           | String (S) | **`USER#<UserID>`** (Primary data locality)                                               |
| **Sort Key (SK)**      | `SK`           | String (S) | **`FILE#<Path>`**, **`GRANT#<RecipientID>#<GrantID>`**, **`VIEWLINK#<OwnerID>#<FileID>`** |

### **1.3. Global Secondary Indexes (GSIs)**

GSIs are the key to enabling the complex, high-performance query patterns required by the application without resorting to inefficient table scans.

| Index Name                       | Partition Key                                          | Sort Key                                       | Access Pattern                                                                                                                                                                       |
| :------------------------------- | :----------------------------------------------------- | :--------------------------------------------- | :----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **GSI 1: ShareAccessIndex**      | `GSI1-PK` (`ACCESS#<RecipientID>`)                     | `GSI1-SK` (`GRANT#<OwnerID>#<Prefix>`)         | Lists all prefix-level grants **shared with a user** ("Shared With Me" view). Each item represents access to a folder prefix and all files within it.                                |
| **GSI 2: MergedFolderViewIndex** | `GSI2-PK` (`VIEWER#<RecipientID>#FOLDER#<FolderName>`) | `GSI2-SK` (`<MediaType>#<Timestamp>#<FileID>`) | Gets a merged, sorted, and filterable list of a **folder's contents** for a specific user, aggregating files from all users who shared folders with the same name (e.g., "photos/"). |

## **2. Detailed Data Modeling and Item Structures**

This section outlines the three core item types stored in the table. Each item type is designed to support specific access patterns, often in conjunction with a GSI. For a complete JSON data set showing all these items in use, see the accompanying `Complete Data Model Example.md` file.

**Important:** There are NO `FOLDER` items in this design. Folders are purely logical prefixes extracted from file paths (S3-style). This dramatically simplifies the schema and eliminates folder hierarchy maintenance.

### **2.1. Item Type: FILE (Owned File Metadata)**

This is the canonical record for a file, stored on the **Owner's** partition. It is the single source of truth for all file metadata. The file path in the sort key supports arbitrary nesting (e.g., `media/photos/2024/vacation/IMG001.jpg`).

| Attribute       | Value Format          | Example (Sheldon's DSCN0010.jpg)          |
| :-------------- | :-------------------- | :---------------------------------------- |
| **PK**          | `USER#<UserID>`       | `USER#Sheldon`                            |
| **SK**          | `FILE#<FilePath>`     | `FILE#media/Project Docs/DSCN0010.jpg`    |
| `ItemType`      | `FILE`                | `FILE`                                    |
| `FileID`        | `<UUID>`              | `R102`                                    |
| `OwnerID`       | `<UserID>`            | `Sheldon`                                 |
| `FileName`      | `<string>`            | `DSCN0010.jpg`                            |
| `FolderPrefix`  | `<string>`            | `media/Project Docs/`                     |
| `CreatedDate`   | `<Timestamp>`         | **`1224685719000`**                       |
| `MediaType`     | `<MIME Type>`         | `image/jpeg`                              |
| `S3Key`         | `<UserID>/<FilePath>` | `Sheldon/media/Project Docs/DSCN0010.jpg` |
| `Size`          | `<number>`            | `161713`                                  |
| `MediaMetadata` | (JSON Object)         | `{ "type": "image", "width": 640, ... }`  |

**Note on FolderPrefix:** Extracted from the file path for efficient querying. For `media/Project Docs/photo.jpg`, the prefix is `media/Project Docs/`. For root-level files like `photo.jpg`, the prefix is empty string `""`.

**Note on Sub-directories:** Clients browsing a specific folder (e.g., `media/`) should query with `begins_with(SK, "FILE#media/")` and then filter client-side to show only direct children (files without additional `/` separators in the remaining path).

### **2.2. Item Type: SHARE_GRANT (Prefix-Level Access Permission)**

Tracks permissions granted by an owner to a recipient for a specific **folder prefix**. A single grant provides access to ALL files matching that prefix, including nested sub-directories. Stored on the **Owner's** partition. This item is projected onto GSI 1 to power the "Shared With Me" view.

| Attribute     | Value Format                    | Example (Sheldon → Justin)          |
| :------------ | :------------------------------ | :---------------------------------- |
| **PK**        | `USER#<OwnerID>`                | `USER#Sheldon`                      |
| **SK**        | `GRANT#<RecipientID>#<GrantID>` | `GRANT#Justin#G-a1b2c3d4`           |
| `ItemType`    | `SHARE_GRANT`                   | `SHARE_GRANT`                       |
| `GrantID`     | `<UUID>`                        | `G-a1b2c3d4`                        |
| `OwnerID`     | `<UserID>`                      | `Sheldon`                           |
| `RecipientID` | `<UserID>`                      | `Justin`                            |
| `Permissions` | `READ`, `READ/WRITE`            | `READ`                              |
| `Prefix`      | `<FolderPath>`                  | `media/Project Docs/`               |
| `CreatedDate` | `<Timestamp>`                   | `1234567890000`                     |
| **`GSI1-PK`** | `ACCESS#<RecipientID>`          | `ACCESS#Justin`                     |
| **`GSI1-SK`** | `GRANT#<OwnerID>#<Prefix>`      | `GRANT#Sheldon#media/Project Docs/` |

**Key Design Notes:**

- **GrantID:** A unique identifier (UUID) for this specific grant. Used for: 1) Preventing duplicate grants via SK uniqueness, 2) Referencing the grant from VIEW_LINK items, 3) Tracking grants in audit logs.
- **Prefix-Level Access:** Granting access to `media/photos/` automatically includes access to `media/photos/2024/`, `media/photos/2024/vacation/`, etc. No per-file grants needed.
- **Atomic Revocation:** Deleting this single item immediately revokes access (enforced by API permission checks). VIEW_LINK cleanup happens asynchronously.

### **2.3. Item Type: VIEW_LINK (Denormalized File Pointer for Merged Views)**

A denormalized pointer created for every file a user can view within a specific folder context. It is the key to enabling merged folder views (e.g., combining "photos/" from multiple users) and efficient filtering by media type. This item is projected onto GSI 2.

**Creation Strategy:** VIEW_LINKs are created **lazily on first folder access** (not immediately when a grant is created). When a user first opens a shared folder, a background job creates all VIEW_LINKs for that folder. Subsequently, VIEW_LINKs are maintained automatically via DynamoDB Streams (new files trigger VIEW_LINK creation, deleted files trigger VIEW_LINK deletion).

| Attribute     | Value Format                               | Example (Justin viewing Sheldon's R102) |
| :------------ | :----------------------------------------- | :-------------------------------------- |
| **PK**        | `USER#<RecipientID>`                       | `USER#Justin`                           |
| **SK**        | `VIEWLINK#<OwnerID>#<FileID>`              | `VIEWLINK#Sheldon#R102`                 |
| `ItemType`    | `VIEW_LINK`                                | `VIEW_LINK`                             |
| `FileID`      | `<UUID>`                                   | `R102`                                  |
| `OwnerID`     | `<UserID>`                                 | `Sheldon`                               |
| `GrantID`     | `<UUID>`                                   | `G-a1b2c3d4`                            |
| `CreatedDate` | `<Timestamp>`                              | `1224685719000`                         |
| `FolderName`  | `<Path>`                                   | `Project Docs/`                         |
| `MediaType`   | `<MIME Type>`                              | `image/jpeg`                            |
| **`GSI2-PK`** | `VIEWER#<RecipientID>#FOLDER#<FolderName>` | `VIEWER#Justin#FOLDER#Project Docs/`    |
| **`GSI2-SK`** | `<MediaType>#<Timestamp>#<FileID>`         | `image/jpeg#1224685719000#R102`         |

**Key Design Notes:**

- **GrantID Reference:** Links back to the SHARE_GRANT that authorized this view. Used for quick invalidation checks and audit trails.
- **Lazy Creation:** On first folder access, user may see "Loading shared content..." for 1-2 seconds while VIEW_LINKs are created in the background (batches of 25). Subsequent accesses are instant.
- **Automatic Maintenance:** DynamoDB Streams trigger Lambda functions to:
  - Create VIEW_LINKs when owner uploads a new file
  - Delete VIEW_LINKs when owner deletes a file
  - Update VIEW_LINKs when file metadata changes (e.g., MediaType correction)
- **GSI2-SK Design:** The sort key `<MediaType>#<Timestamp>#<FileID>` enables:
  - Filtering by media type (e.g., `begins_with(SK, "image/")` for all images)
  - Sorting by creation date (newest first with `ScanIndexForward: false`)
  - Unique file identification (FileID prevents collisions)

## **3. Access Patterns and Query Details (Use Cases)**

This section provides the query strategy for each use case. The developer's goal is to write application code that maps a user action to one of these efficient DynamoDB queries.

### **Use Case 1: Sheldon views the contents of his own folder (media/Project Docs/)**

- **Goal:** List all files directly inside a specific prefix that Sheldon owns.
- **Strategy:** Query the Base Table on the owner's partition using a prefix match on the sort key. This is a very fast and common operation.
- **Query Details:**

  ```rust
  // Query for files under "media/Project Docs/" owned by Sheldon
  let query_input = QueryInput {
      table_name: "FileMetadata".to_string(),
      key_condition_expression: Some("PK = :pk AND begins_with(SK, :sk_prefix)".to_string()),
      expression_attribute_values: Some(hashmap! {
          ":pk".to_string() => AttributeValue::S("USER#Sheldon".to_string()),
          ":sk_prefix".to_string() => AttributeValue::S("FILE#media/Project Docs/".to_string()),
      }),
      ..Default::default()
  };
  // Result: All files matching FILE#media/Project Docs/*
  //   - FILE#media/Project Docs/DSCN0010.jpg ✓
  //   - FILE#media/Project Docs/2024/vacation.jpg ✓ (nested)

  // Client-side filtering for direct children only (no nested paths):
  // For each returned item, extract the path after "media/Project Docs/"
  // and check if it contains additional "/" separators.
  // Example: "DSCN0010.jpg" → show (direct child)
  //          "2024/vacation.jpg" → hide (nested, requires clicking "2024/" first)
  ```

**Note on Sub-directories:** The prefix match returns all files recursively. Clients should filter results to show only direct children by checking if the remaining path contains `/` separators. This matches S3-style folder browsing behavior.

### **Use Case 2: Justin views his "Shared With Me" list**

- **Goal:** List all folder prefixes that have been explicitly shared with Justin, along with who shared them and what permissions he has.
- **Strategy:** Query GSI 1 (ShareAccessIndex). This index is specifically designed to collate all of a user's incoming prefix-level grants.
- **Query Details:**

  ```rust
  // Query for all grants shared with Justin
  let query_input = QueryInput {
      table_name: "FileMetadata".to_string(),
      index_name: Some("ShareAccessIndex".to_string()),
      key_condition_expression: Some("GSI1-PK = :pk".to_string()),
      expression_attribute_values: Some(hashmap! {
          ":pk".to_string() => AttributeValue::S("ACCESS#Justin".to_string()),
      }),
      ..Default::default()
  };
  // Result: SHARE_GRANT items showing:
  //   - Sheldon shared "media/Project Docs/" (READ)
  //   - Leigh shared "media/Team Data/" (READ/WRITE)
  ```

**Result Shape:** Each returned item is a SHARE_GRANT with:

- `OwnerID`: Who shared the folder
- `Prefix`: The folder path shared (e.g., `media/Project Docs/`)
- `Permissions`: Access level (READ or READ/WRITE)
- `GrantID`: Unique grant identifier
- `CreatedDate`: When the share was created

**UI Display:** This powers the "Shared With Me" sidebar/view, showing folder names grouped by owner.

### **Use Case 3: Sheldon revokes Justin's access to a folder prefix**

- **Goal:** Immediately remove Justin's access to all files under `media/Project Docs/`, ensuring he can no longer view or access any content.
- **Strategy:**
  1. Delete the SHARE_GRANT item atomically (immediate permission revocation)
  2. Queue async cleanup job to remove VIEW_LINK items (eventual consistency)
- **Implementation Note:** Permission enforcement is done by checking SHARE_GRANT existence, so revocation is immediate even if VIEW_LINK cleanup is in progress.

**Step 1: Atomic Grant Deletion**

```rust
// Delete the SHARE_GRANT - this immediately revokes access
let transact_input = TransactWriteItemsInput {
    transact_items: vec![
        TransactWriteItem {
            delete: Some(Delete {
                table_name: "FileMetadata".to_string(),
                key: hashmap! {
                    "PK".to_string() => AttributeValue::S("USER#Sheldon".to_string()),
                    "SK".to_string() => AttributeValue::S("GRANT#Justin#G-a1b2c3d4".to_string()),
                },
                ..Default::default()
            }),
            ..Default::default()
        }
    ],
    ..Default::default()
};
// Result: Grant deleted atomically. Justin's access is immediately revoked.
```

**Step 2: Queue VIEW_LINK Cleanup**

```rust
// Send message to SQS for async VIEW_LINK deletion
let message = CleanupMessage {
    action: "DELETE_VIEW_LINKS".to_string(),
    grant_id: "G-a1b2c3d4".to_string(),
    owner_id: "Sheldon".to_string(),
    recipient_id: "Justin".to_string(),
    prefix: "media/Project Docs/".to_string(),
};

sqs_client.send_message()
    .queue_url(&config.cleanup_queue_url)
    .message_body(serde_json::to_string(&message)?)
    .send()
    .await?;
// Worker will process this message and delete VIEW_LINKs in batches of 25
```

**Step 3: Worker Processes Cleanup (Async)**

```rust
// Worker Lambda function triggered by SQS
async fn process_cleanup(message: CleanupMessage) -> Result<()> {
    // 1. Query owner's files for the prefix
    let files = query_files_by_prefix(&message.owner_id, &message.prefix).await?;

    // 2. Delete VIEW_LINKs in batches of 25 (DynamoDB BatchWriteItem limit)
    for chunk in files.chunks(25) {
        let delete_requests: Vec<_> = chunk.iter()
            .map(|file| {
                hashmap! {
                    "PK".to_string() => AttributeValue::S(format!("USER#{}", message.recipient_id)),
                    "SK".to_string() => AttributeValue::S(format!("VIEWLINK#{}#{}", message.owner_id, file.file_id)),
                }
            })
            .collect();

        batch_delete_items(delete_requests).await?;
    }

    Ok(())
}
```

**Permission Enforcement:**

```rust
// API validation before returning any file data
async fn validate_access(user_id: &str, file: &FileMetadata) -> Result<bool> {
    // If user is the owner, access granted
    if file.owner_id == user_id {
        return Ok(true);
    }

    // Check if a SHARE_GRANT exists for this prefix
    let grant_exists = check_grant_exists(
        &file.owner_id,
        user_id,
        &file.folder_prefix
    ).await?;

    // Access granted only if grant exists (immediate revocation)
    Ok(grant_exists)
}
```

**Key Benefits:**

- ✅ Revocation is **immediate** (enforced by grant check, not VIEW_LINK presence)
- ✅ Single atomic operation (no transaction size limits)
- ✅ VIEW_LINK cleanup happens in background (user never waits)
- ✅ Idempotent cleanup (safe to retry on failure)

### **Use Case 4: Justin views the merged contents of folders named "Project Docs/"**

- **Goal:** List all files from any folder with the path ending in "Project Docs/" that Justin has access to (e.g., from Sheldon's `media/Project Docs/`, Leigh's `work/Project Docs/`, etc.), merged into a single view, filtered by media type, and sorted by creation date (newest first).
- **Strategy:** Query GSI 2 (MergedFolderViewIndex). This is the key to the seamless shared folder experience and enables native DynamoDB cursor-based pagination.
- **Explanation:** The `GSI2-PK` groups all files visible to Justin within any folder named "Project Docs/" into a single item collection. The `GSI2-SK` enables efficient sorting by date and filtering by media type.

**Example 1: Query all files (no media type filter)**

```rust
// Query for ALL files in "Project Docs/" visible to Justin
let query_input = QueryInput {
    table_name: "FileMetadata".to_string(),
    index_name: Some("MergedFolderViewIndex".to_string()),
    key_condition_expression: Some("GSI2-PK = :pk".to_string()),
    expression_attribute_values: Some(hashmap! {
        ":pk".to_string() => AttributeValue::S("VIEWER#Justin#FOLDER#Project Docs/".to_string()),
    }),
    scan_index_forward: Some(false), // Newest first (sort by timestamp descending)
    limit: Some(50), // Page size
    exclusive_start_key: None, // For first page; use returned LastEvaluatedKey for subsequent pages
    ..Default::default()
};
// Result: Merged, sorted list of VIEW_LINK items from all owners
```

**Example 2: Query only images**

```rust
// Query for IMAGES ONLY in "Project Docs/"
let query_input = QueryInput {
    table_name: "FileMetadata".to_string(),
    index_name: Some("MergedFolderViewIndex".to_string()),
    key_condition_expression: Some("GSI2-PK = :pk AND begins_with(GSI2-SK, :sk_prefix)".to_string()),
    expression_attribute_values: Some(hashmap! {
        ":pk".to_string() => AttributeValue::S("VIEWER#Justin#FOLDER#Project Docs/".to_string()),
        ":sk_prefix".to_string() => AttributeValue::S("image/".to_string()),
    }),
    scan_index_forward: Some(false),
    limit: Some(50),
    ..Default::default()
};
// Result: Only files with MediaType starting with "image/" (image/jpeg, image/png, etc.)
```

**Example 3: Query only videos**

```rust
// Query for VIDEOS ONLY in "Project Docs/"
let query_input = QueryInput {
    key_condition_expression: Some("GSI2-PK = :pk AND begins_with(GSI2-SK, :sk_prefix)".to_string()),
    expression_attribute_values: Some(hashmap! {
        ":pk".to_string() => AttributeValue::S("VIEWER#Justin#FOLDER#Project Docs/".to_string()),
        ":sk_prefix".to_string() => AttributeValue::S("video/".to_string()),
    }),
    // ... rest same as above
};
// Result: Only files with MediaType starting with "video/" (video/mp4, video/quicktime, etc.)
```

**Pagination:**

```rust
// Page 1: Initial query (no cursor)
let page1_result = client.query().send().await?;
let files_page1 = page1_result.items;
let next_cursor = page1_result.last_evaluated_key;

// Page 2: Use cursor from page 1
let query_input = QueryInput {
    // ... same key conditions as above
    exclusive_start_key: next_cursor, // Native DynamoDB cursor
    ..Default::default()
};
let page2_result = client.query().send().await?;
// Pagination works perfectly because GSI2 maintains global sort order
```

**Key Benefits:**

- ✅ **Single query** returns merged results from multiple owners (Sheldon, Leigh, etc.)
- ✅ **Native DynamoDB pagination** with LastEvaluatedKey (no complex cursor logic needed)
- ✅ **Efficient filtering** by media type via sort key prefix match
- ✅ **Correct global sort order** by creation date across all contributors
- ✅ **Scales to 20+ contributors** without query complexity or latency issues

### **Use Case 5: Sheldon grants Justin access to a folder prefix**

- **Goal:** Share all files under `media/Project Docs/` with Justin, giving him READ access.
- **Strategy:** Create a single SHARE_GRANT item. VIEW_LINKs will be created lazily on Justin's first access.

```rust
use uuid::Uuid;

// Generate unique grant ID
let grant_id = format!("G-{}", Uuid::new_v4().to_string());

// Create SHARE_GRANT
let put_input = PutItemInput {
    table_name: "FileMetadata".to_string(),
    item: hashmap! {
        "PK".to_string() => AttributeValue::S("USER#Sheldon".to_string()),
        "SK".to_string() => AttributeValue::S(format!("GRANT#Justin#{}", grant_id)),
        "ItemType".to_string() => AttributeValue::S("SHARE_GRANT".to_string()),
        "GrantID".to_string() => AttributeValue::S(grant_id.clone()),
        "OwnerID".to_string() => AttributeValue::S("Sheldon".to_string()),
        "RecipientID".to_string() => AttributeValue::S("Justin".to_string()),
        "Permissions".to_string() => AttributeValue::S("READ".to_string()),
        "Prefix".to_string() => AttributeValue::S("media/Project Docs/".to_string()),
        "CreatedDate".to_string() => AttributeValue::N(chrono::Utc::now().timestamp_millis().to_string()),
        "GSI1-PK".to_string() => AttributeValue::S("ACCESS#Justin".to_string()),
        "GSI1-SK".to_string() => AttributeValue::S("GRANT#Sheldon#media/Project Docs/".to_string()),
    },
    condition_expression: Some("attribute_not_exists(PK)".to_string()), // Prevent duplicate grants
    ..Default::default()
};

client.put_item(put_input).await?;
// Result: Single write operation. Justin now has access to all files under the prefix.
// VIEW_LINKs will be created on first folder access.
```

**Note:** This single operation grants access to potentially thousands of files. No per-file writes required.

### **Use Case 6: Sheldon uploads a new file to a shared folder**

- **Goal:** When Sheldon uploads `media/Project Docs/NewPhoto.jpg`, automatically create VIEW_LINKs for all users who have access to that prefix.
- **Strategy:** DynamoDB Streams + Lambda trigger automatically detects the new FILE item and creates VIEW_LINKs.

**Step 1: Sheldon uploads file (creates FILE item)**

```rust
// Application creates FILE item on S3 upload
let put_input = PutItemInput {
    table_name: "FileMetadata".to_string(),
    item: hashmap! {
        "PK".to_string() => AttributeValue::S("USER#Sheldon".to_string()),
        "SK".to_string() => AttributeValue::S("FILE#media/Project Docs/NewPhoto.jpg".to_string()),
        "ItemType".to_string() => AttributeValue::S("FILE".to_string()),
        "FileID".to_string() => AttributeValue::S("R999".to_string()),
        "OwnerID".to_string() => AttributeValue::S("Sheldon".to_string()),
        "FileName".to_string() => AttributeValue::S("NewPhoto.jpg".to_string()),
        "FolderPrefix".to_string() => AttributeValue::S("media/Project Docs/".to_string()),
        "MediaType".to_string() => AttributeValue::S("image/jpeg".to_string()),
        "CreatedDate".to_string() => AttributeValue::N("1234567890000".to_string()),
        "S3Key".to_string() => AttributeValue::S("Sheldon/media/Project Docs/NewPhoto.jpg".to_string()),
        // ... other metadata
    },
    ..Default::default()
};
```

**Step 2: DynamoDB Stream triggers Lambda**

```rust
// Lambda function receives stream event
async fn handle_stream_event(event: DynamoDbEvent) -> Result<()> {
    for record in event.records {
        if record.event_name == "INSERT" && record.dynamodb.new_image.item_type == "FILE" {
            let file = parse_file_from_stream_record(&record)?;

            // Find all grants for this prefix
            let grants = find_grants_for_prefix(&file.owner_id, &file.folder_prefix).await?;

            // Create VIEW_LINK for each recipient (+ owner)
            let mut view_links = vec![];
            for grant in grants {
                view_links.push(create_view_link_item(
                    &grant.recipient_id,
                    &file,
                    &grant.grant_id,
                    &grant.prefix
                ));
            }

            // Also create VIEW_LINK for owner
            view_links.push(create_view_link_item(
                &file.owner_id,
                &file,
                "OWNER",
                &file.folder_prefix
            ));

            // Batch write VIEW_LINKs (25 at a time)
            batch_write_items(view_links).await?;
        }
    }
    Ok(())
}
```

**Key Benefits:**

- ✅ Automatic VIEW_LINK maintenance (no manual sync needed)
- ✅ New files immediately visible to recipients
- ✅ Handles file deletions and updates automatically via streams

## **4. Implementation Guide**

This section provides detailed implementation guidance for the core infrastructure components needed to support this schema.

### **4.1. DynamoDB Streams Configuration**

Enable DynamoDB Streams on the FileMetadata table to automatically trigger Lambda functions for VIEW_LINK maintenance.

```rust
// Terraform configuration (see DynamoDB Terraform Configuration.md for complete setup)
resource "aws_dynamodb_table" "file_metadata" {
  // ... table configuration

  stream_enabled   = true
  stream_view_type = "NEW_AND_OLD_IMAGES" // Capture both old and new item states
}
```

**Stream Processing Lambda:**

```rust
use aws_lambda_events::event::dynamodb::{Event as DynamoDbEvent, EventRecord};
use aws_sdk_dynamodb::Client as DynamoDbClient;

#[tokio::main]
async fn main() -> Result<(), Error> {
    lambda_runtime::run(handler(func)).await?;
    Ok(())
}

async fn func(event: DynamoDbEvent, _ctx: Context) -> Result<(), Error> {
    let config = aws_config::load_from_env().await;
    let client = DynamoDbClient::new(&config);

    for record in event.records {
        match record.event_name.as_str() {
            "INSERT" => handle_insert(&client, &record).await?,
            "MODIFY" => handle_modify(&client, &record).await?,
            "REMOVE" => handle_remove(&client, &record).await?,
            _ => {}
        }
    }

    Ok(())
}

async fn handle_insert(client: &DynamoDbClient, record: &EventRecord) -> Result<()> {
    let new_image = record.dynamodb.new_image.as_ref().unwrap();
    let item_type = new_image.get("ItemType")?.as_s()?;

    match item_type.as_str() {
        "FILE" => {
            // New file uploaded - create VIEW_LINKs for all recipients
            let file = parse_file(new_image)?;
            let grants = find_grants_for_prefix(client, &file.owner_id, &file.folder_prefix).await?;

            // Create VIEW_LINK for owner + all recipients
            let mut recipients = vec![file.owner_id.clone()];
            recipients.extend(grants.iter().map(|g| g.recipient_id.clone()));

            create_view_links_batch(client, &file, &recipients, &grants).await?;
        },
        "SHARE_GRANT" => {
            // New grant created - VIEW_LINKs will be created lazily on first access
            // No action needed here
        },
        _ => {}
    }

    Ok(())
}

async fn handle_remove(client: &DynamoDbClient, record: &EventRecord) -> Result<()> {
    let old_image = record.dynamodb.old_image.as_ref().unwrap();
    let item_type = old_image.get("ItemType")?.as_s()?;

    match item_type.as_str() {
        "FILE" => {
            // File deleted - remove all VIEW_LINKs
            let file = parse_file(old_image)?;
            let grants = find_grants_for_prefix(client, &file.owner_id, &file.folder_prefix).await?;

            let mut recipients = vec![file.owner_id.clone()];
            recipients.extend(grants.iter().map(|g| g.recipient_id.clone()));

            delete_view_links_batch(client, &file.file_id, &file.owner_id, &recipients).await?;
        },
        _ => {}
    }

    Ok(())
}
```

### **4.2. SQS Cleanup Queue Configuration**

Create an SQS queue for asynchronous VIEW_LINK cleanup when grants are revoked.

```rust
// Terraform configuration
resource "aws_sqs_queue" "view_link_cleanup" {
  name                       = "view-link-cleanup-queue"
  visibility_timeout_seconds = 300  // 5 minutes for Lambda processing
  message_retention_seconds  = 1209600  // 14 days
  receive_wait_time_seconds  = 20  // Long polling

  redrive_policy = jsonencode({
    deadLetterTargetArn = aws_sqs_queue.view_link_cleanup_dlq.arn
    maxReceiveCount     = 3
  })
}

resource "aws_sqs_queue" "view_link_cleanup_dlq" {
  name = "view-link-cleanup-dlq"
}
```

**Cleanup Worker Lambda:**

```rust
use aws_lambda_events::event::sqs::{SqsEvent, SqsMessage};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct CleanupMessage {
    action: String,
    grant_id: String,
    owner_id: String,
    recipient_id: String,
    prefix: String,
}

async fn func(event: SqsEvent, _ctx: Context) -> Result<(), Error> {
    let config = aws_config::load_from_env().await;
    let client = DynamoDbClient::new(&config);

    for record in event.records {
        let message: CleanupMessage = serde_json::from_str(&record.body)?;

        match message.action.as_str() {
            "DELETE_VIEW_LINKS" => {
                delete_view_links_for_grant(&client, &message).await?;
            },
            _ => {
                eprintln!("Unknown action: {}", message.action);
            }
        }
    }

    Ok(())
}

async fn delete_view_links_for_grant(
    client: &DynamoDbClient,
    message: &CleanupMessage
) -> Result<()> {
    // 1. Query owner's files for the prefix
    let files = query_files_by_prefix(
        client,
        &message.owner_id,
        &message.prefix
    ).await?;

    // 2. Delete VIEW_LINKs in batches of 25
    for chunk in files.chunks(25) {
        let delete_requests: Vec<_> = chunk.iter()
            .map(|file| {
                WriteRequest::builder()
                    .delete_request(
                        DeleteRequest::builder()
                            .key("PK", AttributeValue::S(format!("USER#{}", message.recipient_id)))
                            .key("SK", AttributeValue::S(format!("VIEWLINK#{}#{}", message.owner_id, file.file_id)))
                            .build()
                    )
                    .build()
            })
            .collect();

        client.batch_write_item()
            .request_items("FileMetadata", delete_requests)
            .send()
            .await?;

        // Small delay to avoid throttling
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Ok(())
}
```

### **4.3. Lazy VIEW_LINK Creation**

When a user first accesses a shared folder, create VIEW_LINKs in the background.

```rust
async fn lazy_create_view_links(
    client: &DynamoDbClient,
    recipient_id: &str,
    grant: &ShareGrant
) -> Result<()> {
    // Check if VIEW_LINKs already exist for this grant
    let check_result = client.query()
        .table_name("FileMetadata")
        .key_condition_expression("PK = :pk AND begins_with(SK, :sk)")
        .expression_attribute_values(":pk", AttributeValue::S(format!("USER#{}", recipient_id)))
        .expression_attribute_values(":sk", AttributeValue::S(format!("VIEWLINK#{}#", grant.owner_id)))
        .limit(1)
        .send()
        .await?;

    if check_result.items.is_some() && !check_result.items.unwrap().is_empty() {
        // VIEW_LINKs already exist
        return Ok(());
    }

    // Query all files under the granted prefix
    let files = query_files_by_prefix(
        client,
        &grant.owner_id,
        &grant.prefix
    ).await?;

    // Create VIEW_LINKs in batches of 25
    for chunk in files.chunks(25) {
        let put_requests: Vec<_> = chunk.iter()
            .map(|file| create_view_link_put_request(recipient_id, file, &grant.grant_id, &grant.prefix))
            .collect();

        client.batch_write_item()
            .request_items("FileMetadata", put_requests)
            .send()
            .await?;
    }

    Ok(())
}

// Call this when user first opens a shared folder
async fn get_shared_folder_contents(
    client: &DynamoDbClient,
    recipient_id: &str,
    folder_name: &str
) -> Result<Vec<FileMetadata>> {
    // 1. Get grant for this folder
    let grants = get_grants_for_folder(client, recipient_id, folder_name).await?;

    // 2. Ensure VIEW_LINKs exist (lazy creation)
    for grant in &grants {
        tokio::spawn({
            let client = client.clone();
            let recipient = recipient_id.to_string();
            let grant = grant.clone();
            async move {
                lazy_create_view_links(&client, &recipient, &grant).await
            }
        });
    }

    // 3. Query GSI2 for merged folder view
    let result = client.query()
        .table_name("FileMetadata")
        .index_name("MergedFolderViewIndex")
        .key_condition_expression("GSI2-PK = :pk")
        .expression_attribute_values(":pk", AttributeValue::S(format!("VIEWER#{}#FOLDER#{}", recipient_id, folder_name)))
        .scan_index_forward(false)
        .limit(50)
        .send()
        .await?;

    // Parse and return files
    Ok(parse_files(result.items))
}
```

### **4.4. Permission Validation**

Always validate permissions against SHARE_GRANT existence, not VIEW_LINK presence.

```rust
async fn validate_file_access(
    client: &DynamoDbClient,
    user_id: &str,
    file: &FileMetadata
) -> Result<bool> {
    // Owner always has access
    if file.owner_id == user_id {
        return Ok(true);
    }

    // Check if a grant exists for this prefix
    let result = client.query()
        .table_name("FileMetadata")
        .index_name("ShareAccessIndex")
        .key_condition_expression("GSI1-PK = :pk AND begins_with(GSI1-SK, :sk)")
        .expression_attribute_values(":pk", AttributeValue::S(format!("ACCESS#{}", user_id)))
        .expression_attribute_values(":sk", AttributeValue::S(format!("GRANT#{}#{}", file.owner_id, file.folder_prefix)))
        .limit(1)
        .send()
        .await?;

    // Access granted if any matching grant exists
    Ok(result.items.is_some() && !result.items.unwrap().is_empty())
}
```

### **4.5. Batch Write Helper Functions**

Utility functions for efficient batch operations.

```rust
async fn batch_write_items(
    client: &DynamoDbClient,
    items: Vec<HashMap<String, AttributeValue>>
) -> Result<()> {
    const BATCH_SIZE: usize = 25; // DynamoDB limit

    for chunk in items.chunks(BATCH_SIZE) {
        let put_requests: Vec<_> = chunk.iter()
            .map(|item| {
                WriteRequest::builder()
                    .put_request(PutRequest::builder().set_item(Some(item.clone())).build())
                    .build()
            })
            .collect();

        let mut request = client.batch_write_item()
            .request_items("FileMetadata", put_requests);

        // Retry with exponential backoff for unprocessed items
        loop {
            let response = request.send().await?;

            if response.unprocessed_items.is_none() || response.unprocessed_items.unwrap().is_empty() {
                break;
            }

            // Retry unprocessed items after delay
            tokio::time::sleep(Duration::from_millis(100)).await;
            request = client.batch_write_item()
                .set_request_items(response.unprocessed_items);
        }
    }

    Ok(())
}
```

### **4.6. Error Handling and Idempotency**

**Idempotent Grant Creation:**

```rust
// Use condition expression to prevent duplicate grants
let put_input = PutItemInput {
    // ... item attributes
    condition_expression: Some("attribute_not_exists(PK)".to_string()),
    ..Default::default()
};

match client.put_item(put_input).await {
    Ok(_) => Ok(()),
    Err(e) if e.is_conditional_check_failed() => {
        // Grant already exists - idempotent success
        Ok(())
    },
    Err(e) => Err(e.into()),
}
```

**Idempotent Cleanup:**

```rust
// VIEW_LINK deletion is naturally idempotent
// Deleting a non-existent item succeeds in DynamoDB
async fn delete_view_link(client: &DynamoDbClient, pk: &str, sk: &str) -> Result<()> {
    client.delete_item()
        .table_name("FileMetadata")
        .key("PK", AttributeValue::S(pk.to_string()))
        .key("SK", AttributeValue::S(sk.to_string()))
        .send()
        .await?;

    // Success whether item existed or not
    Ok(())
}
```

## **5. Performance Characteristics and Cost Analysis**

### **5.1. Read Performance**

| Operation                 | Strategy         | Latency | RCU Cost (per operation)       |
| ------------------------- | ---------------- | ------- | ------------------------------ |
| **View own folder**       | Base table query | 5-10ms  | 0.5-1 RCU (50KB typical)       |
| **"Shared With Me" list** | GSI1 query       | 5-10ms  | 0.5 RCU (typically <10 grants) |
| **Merged folder view**    | GSI2 query       | 10-20ms | 1-2 RCU (50 items × 2KB each)  |
| **Permission check**      | GSI1 query       | 5-10ms  | 0.5 RCU (1 item)               |

**Scaling Characteristics:**

- ✅ Merged folder view latency is **constant** regardless of contributor count (1 query for 1 owner or 20 owners)
- ✅ Pagination is native DynamoDB cursor (no server-side merge complexity)
- ✅ Filtering by media type is efficient (uses sort key prefix match)

### **5.2. Write Performance**

| Operation        | Strategy           | Latency | WCU Cost | Notes                                 |
| ---------------- | ------------------ | ------- | -------- | ------------------------------------- |
| **Upload file**  | Put FILE item      | 5-10ms  | 1 WCU    | + async VIEW_LINK creation via Stream |
| **Create grant** | Put SHARE_GRANT    | 5-10ms  | 1 WCU    | Single write, VIEW_LINKs lazy         |
| **Revoke grant** | Delete SHARE_GRANT | 5-10ms  | 1 WCU    | + async VIEW_LINK cleanup via SQS     |
| **Delete file**  | Delete FILE item   | 5-10ms  | 1 WCU    | + async VIEW_LINK cleanup via Stream  |

**Write Amplification:**

- **With VIEW_LINKs (this design):** 1 grant write + N VIEW_LINK writes (async, lazy)
- **Without VIEW_LINKs:** 1 grant write, but complex merge/pagination logic required

### **5.3. Cost Analysis**

**Scenario: Photo sharing service with 10,000 active users**

**Monthly usage assumptions:**

- 1,000 photo uploads per day (10% of users)
- 50 folder views per user per day (500,000 views/day)
- 100 new shares created per day
- 20 revocations per day

**DynamoDB costs (Pay-Per-Request):**

| Item                            | Daily Operations                   | WCU/RCU Cost | Monthly Cost     |
| ------------------------------- | ---------------------------------- | ------------ | ---------------- |
| **Read Operations**             |                                    |              |                  |
| Folder views (GSI2)             | 500,000 queries × 2 RCU            | 1M RCU       | $0.25            |
| Permission checks               | 500,000 checks × 0.5 RCU           | 500K RCU     | $0.13            |
| "Shared With Me"                | 100,000 views × 0.5 RCU            | 50K RCU      | $0.01            |
| **Write Operations**            |                                    |              |                  |
| File uploads                    | 1,000 files × 1 WCU                | 30K WCU      | $0.04            |
| VIEW_LINK creation              | 1,000 files × 5 recipients × 1 WCU | 150K WCU     | $0.19            |
| Grant creation                  | 100 grants × 1 WCU                 | 3K WCU       | $0.004           |
| Revocations                     | 20 × 1 WCU                         | 600 WCU      | $0.001           |
| **Storage**                     |                                    |              |                  |
| 10M files (500 bytes/file)      |                                    | 5 GB         | $1.25            |
| 50M VIEW_LINKs (200 bytes/link) |                                    | 10 GB        | $2.50            |
| **Total DynamoDB**              |                                    |              | **~$4.35/month** |

**Supporting infrastructure:**

- Lambda executions (Stream + SQS): ~$2/month
- SQS messages: ~$0.10/month
- **Total: ~$6.50/month** for 10,000 active users

**Cost per user: $0.00065/month**

### **5.4. Comparison: VIEW_LINK vs No VIEW_LINK Design**

| Aspect                     | With VIEW_LINKs (This Design) | Without VIEW_LINKs              |
| -------------------------- | ----------------------------- | ------------------------------- |
| **Storage Cost**           | Higher (denormalized data)    | Lower (grants only)             |
| **Write Cost**             | Higher (create VIEW_LINKs)    | Lower (grants only)             |
| **Read Cost**              | Lower (single queries)        | Higher (parallel queries)       |
| **Read Latency**           | Excellent (10-20ms)           | Good (50-100ms with merge)      |
| **Pagination**             | Native DynamoDB cursor        | Complex server-side merge       |
| **Code Complexity**        | Moderate (async cleanup)      | High (merge + cursor logic)     |
| **Scalability**            | Excellent (20+ contributors)  | Limited (5-10 contributors max) |
| **Revocation**             | Immediate (grant check)       | Immediate (grant check)         |
| **Total Cost (10K users)** | ~$6.50/month                  | ~$8/month (higher read cost)    |

**Conclusion:** VIEW_LINK design is **more cost-effective** for read-heavy workloads (typical for photo/file sharing) and provides superior UX with native pagination and instant merged views.

### **5.5. Scaling Considerations**

**Hot Partition Prevention:**

- GSI2 partition keys include recipient ID (e.g., `VIEWER#Justin#FOLDER#photos/`)
- Each user's view is on a separate partition
- No hot partitions even with 1M+ users

**Large Folder Handling:**

- Folders with 10,000+ files work efficiently with lazy VIEW_LINK creation
- Stream-based processing handles bulk uploads (batch creation in background)
- Pagination ensures UI remains responsive

**Contributor Limits:**

- Design supports 20+ users sharing same folder name to one recipient
- GSI2 query performance remains constant regardless of contributor count
- No complex merge logic or parallel query limits

## **6. Supporting Artifacts**

- **Full Data Model:** See `Complete Data Model Example.md` for the complete dataset with examples of all item types.
- **Infrastructure:** See `DynamoDB Terraform Configuration.md` for the complete Terraform configuration including table, GSIs, streams, and supporting infrastructure.
- **Schema Summary:** Single-table design with S3-style prefix-based folders, prefix-level grants, and lazy VIEW_LINK denormalization for optimal read performance.
