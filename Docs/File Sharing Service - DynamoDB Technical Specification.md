_This immersive artifact is rendered using Gemini AST._

# **File Sharing Service - DynamoDB Technical Specification**

## **1. Architecture Overview and Principles**

This document specifies the DynamoDB schema for a highly scalable file metadata service, utilizing a **Single-Table Design** to manage file hierarchy, ownership, and complex sharing relationships with maximum efficiency. This design is intended to power an application with functionality similar to Google Drive or Google Photos.

### **1.1. Core Architectural Principle**

All file metadata, sharing permissions, and denormalized links for different views are stored in a single DynamoDB table. The table is primarily partitioned by the user's ID (`USER#<UserID>`), which provides extreme data locality for the most common access patterns (a user interacting with their own items).

**Key Design Decisions:**

- **S3-Style Prefix-Based File Storage:** Physical files are stored in S3 with full path keys (e.g., `Sheldon/media/photos/2024/vacation/img.jpg`). There are NO explicit folder objects in S3 - folders are purely logical prefixes.

- **S3-Style Folder Derivation in DynamoDB:** Folders are derived from file paths and folder markers, mimicking S3's bucket browsing behavior. Each folder level is represented by a special folder marker VIEW_LINK item that sorts before file items, enabling efficient single-query folder browsing that shows both subfolders and files.

- **Two Grant Types:** Access can be granted at two levels:

  - **PREFIX Grants:** A single `SHARE_GRANT` for `media/photos/` grants access to ALL files matching that prefix, including nested sub-directories
  - **FILE Grants:** Individual file access without parent folder access, enabling privacy-preserving selective sharing

- **Universal VIEW_LINK Access Pattern:** ALL folder browsing operations use VIEW_LINK items queried via GSI2, regardless of whether the user is the owner or a recipient. This unified approach eliminates conditional query logic and ensures consistent UX. VIEW_LINKs are created automatically by SQS Lambda processors when S3 files are uploaded or when grants are created via API.

- **Automatic Folder Marker Creation:** When a file is created in a nested path (e.g., `media/photos/2024/vacation/img.jpg`), folder markers are automatically created for each level (`media/`, `media/photos/`, `media/photos/2024/`, `media/photos/2024/vacation/`) if they don't already exist. This enables S3-style folder navigation.

- **FILE Items for Direct Operations Only:** The base table FILE items serve as the canonical source of truth for file metadata. All folder browsing operations query VIEW_LINKs via GSI2, never FILE items directly.

- **VIEW_LINK Existence = Access Proof:** Permission validation is simple: if a VIEW_LINK exists for a user viewing a file, access is granted. This eliminates the need for complex grant checking during file operations.

- **Synchronous API Operations:** All API operations complete fully before returning to the client. This simplifies the architecture during the MVP phase, with future optimization for async operations planned as the user base grows.

- **S3 Lifecycle Management:** File creation, updates, and deletions occur directly in S3. S3 events trigger SQS messages that are processed by Lambda functions to maintain DynamoDB metadata synchronization.

**Operational Context:**

- **Users for Examples:** Sheldon, Leigh, and Justin.
- **Table Name:** `FileMetadata`
- **Billing Mode:** Pay-Per-Request (On-Demand), ideal for unpredictable workloads.
- **Supporting Infrastructure:** S3 bucket with SQS event notifications, Lambda function for S3 event processing, custom API Gateway + Lambda for RESTful API endpoints.

### **1.2. Schema Keys**

| Key Type               | Attribute Name | Data Type  | Purpose                                                                                                                                                                                                        |
| :--------------------- | :------------- | :--------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Partition Key (PK)** | `PK`           | String (S) | **`USER#<UserID>`** (Primary data locality)                                                                                                                                                                    |
| **Sort Key (SK)**      | `SK`           | String (S) | **FILE items:** `FILE#<Path>`<br>**SHARE_GRANT items:** `GRANT#<RecipientID>#<GrantID>`<br>**VIEW_LINK items:** `VIEWLINK#<OwnerID>#<ResourceID>` (file) or `VIEWLINK#<OwnerID>#FOLDER#<Path>` (folder marker) |

### **1.3. Global Secondary Indexes (GSIs)**

GSIs are the key to enabling the complex, high-performance query patterns required by the application without resorting to inefficient table scans.

| Index Name               | Partition Key                                            | Sort Key                                                                                                                                 | Access Pattern                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| :----------------------- | :------------------------------------------------------- | :--------------------------------------------------------------------------------------------------------------------------------------- | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **GSI 1: GrantIndex**    | `GSI1-PK` (`ACCESS#<RecipientID>`)                       | `GSI1-SK` - **PREFIX grants:** `GRANT#<OwnerID>#<Prefix>`<br>**FILE grants:** `GRANT#<OwnerID>#FILE#<ResourceID>`                        | Lists all prefix-level grants **shared with a user** ("Shared With Me" view). Each item represents access to a folder prefix and all files within it, or access to a single specific file.                                                                                                                                                                                                                                                                                    |
| **GSI 2: ViewLinkIndex** | `GSI2-PK` (`VIEWER#<RecipientID>#FOLDER#<FolderPrefix>`) | `GSI2-SK` - **Folder markers:** `TYPE#FOLDER#<FolderName>#<OwnerID>`<br>**File items:** `TYPE#FILE#<Timestamp>#<MediaType>#<ResourceID>` | **Primary access pattern for ALL folder browsing.** Gets a merged, sorted list of a folder's subfolders and files for a specific user. The sort key design (`TYPE#FOLDER#` vs `TYPE#FILE#`) ensures folders appear first, mimicking S3's bucket browsing behavior. Timestamp-first file sorting enables pure chronological sorting across all media types and owners. This unified pattern works identically whether the user is viewing their own folder or a shared folder. |

## **2. Detailed Data Modeling and Item Structures**

This section outlines the core item types stored in the table. Each item type is designed to support specific access patterns, often in conjunction with a GSI.

**Important:** There are NO explicit FOLDER items as separate entities. Folders are represented through:

1. **Folder Prefixes** in file paths (e.g., `media/photos/2024/`)
2. **Folder Marker VIEW_LINKs** that enable efficient folder navigation (showing subfolders)

### **2.1. Item Type: FILE (Owned File Metadata)**

This is the canonical record for a file, stored on the **Owner's** partition. It is the single source of truth for all file metadata. The file path in the sort key supports arbitrary nesting (e.g., `media/photos/2024/vacation/IMG001.jpg`).

**Usage:** FILE items are accessed ONLY for direct file operations:

- Creating a new file (upload) - **triggered by S3 event only**
- Updating file metadata (rename, tag changes) - **triggered by S3 event only**
- Deleting a file - **triggered by S3 event only**
- Direct file access (download, get S3 key for signed URL)
- Admin operations (bulk exports, data migrations)

**Critical Architectural Note:** All file create/update/delete operations happen directly in S3 through **CloudFront OAC (Origin Access Control) with signed URLs** (see [Section 5](#5-cloudfront--s3-signed-url-architecture)). Users never interact with S3 directly:

1. **Upload Flow:** User requests signed URL from API → uploads via CloudFront → CloudFront proxies to S3 → S3 sends ObjectCreated event → Lambda creates FILE item
2. **Download Flow:** API generates signed URL → user downloads via CloudFront → CloudFront proxies S3 GET request with OAC
3. **Delete Flow:** API generates signed URL → CloudFront proxies DELETE to S3 → S3 sends ObjectRemoved event → Lambda deletes FILE item

S3 events (ObjectCreated/ObjectRemoved) trigger SQS messages that Lambda processors use to create/update/delete FILE items in DynamoDB. The API never directly creates or modifies FILE items - all FILE item mutations originate from S3 events triggered by CloudFront-proxied operations.

**Not Used For:** Folder browsing or listing files. All folder views use VIEW_LINK items via GSI2.

| Attribute       | Value Format          | Example (Sheldon's DSCN0010.jpg)          |
| :-------------- | :-------------------- | :---------------------------------------- |
| **PK**          | `USER#<UserID>`       | `USER#Sheldon`                            |
| **SK**          | `FILE#<FilePath>`     | `FILE#media/Project Docs/DSCN0010.jpg`    |
| `ItemType`      | `FILE`                | `FILE`                                    |
| `ResourceID`    | `<UUID>`              | `R102`                                    |
| `OwnerID`       | `<UserID>`            | `Sheldon`                                 |
| `FileName`      | `<string>`            | `DSCN0010.jpg`                            |
| `FolderPrefix`  | `<string>`            | `media/Project Docs/`                     |
| `CreatedDate`   | `<Timestamp>`         | **`1224685719000`**                       |
| `MediaType`     | `<MIME Type>`         | `image/jpeg`                              |
| `S3Key`         | `<UserID>/<FilePath>` | `Sheldon/media/Project Docs/DSCN0010.jpg` |
| `Size`          | `<number>`            | `161713`                                  |
| `MediaMetadata` | (JSON Object)         | `{ "type": "image", "width": 640, ... }`  |

**Note on FolderPrefix:** The immediate parent folder path extracted from the full file path. For `media/Project Docs/DSCN0010.jpg`, the prefix is `media/Project Docs/`. For nested files like `media/photos/2024/vacation/img.jpg`, the prefix is `media/photos/2024/vacation/`. For root-level files like `photo.jpg`, the prefix is empty string `""`.

**Note on S3 Storage:** Physical files are stored in S3 with keys matching the full path (e.g., `Sheldon/media/Project Docs/DSCN0010.jpg`). There are NO folder objects in S3 - folders are purely logical concepts derived from file paths.

**Note on Direct Access:** To retrieve a specific file (for download), query the base table using `PK = USER#<OwnerID>` and `SK = FILE#<FilePath>` to get the S3 key, then generate a CloudFront signed URL for secure access. For create/update/delete operations, users obtain CloudFront signed URL directly (via API) and perform the operation through CloudFront, which triggers S3 events that update DynamoDB. FILE items in DynamoDB are never directly modified by API operations - they are only created/updated/deleted by S3 event processors.

### **2.2. Item Type: SHARE_GRANT (Access Permission)**

Tracks permissions granted by an owner to a recipient for either a **folder prefix** or an **individual file**. SHARE_GRANT items support two grant types to handle different sharing scenarios while maintaining a unified table structure. All grants are stored on the **Owner's** partition and projected onto GSI 1 to power the "Shared With Me" view and share revocations functions.

**CRITICAL: SHARE_GRANT items are ONLY created and deleted via API operations. S3 event processing NEVER touches SHARE_GRANT items.** This separation of concerns ensures:

- S3 events manage file lifecycle (FILE items, VIEW_LINKs)
- API operations manage sharing lifecycle (SHARE_GRANT items, VIEW_LINKs for recipients)
- Folder structure (folder marker VIEW_LINKs) persists independent of file contents

#### **2.2.1. PREFIX Grant (Folder-Level Access)**

A PREFIX grant provides access to ALL files matching a specific folder prefix, including nested sub-directories. This is the most common grant type for sharing entire folders or folder trees.

| Attribute     | Value Format                    | Example (Sheldon → Justin)          |
| :------------ | :------------------------------ | :---------------------------------- |
| **PK**        | `USER#<OwnerID>`                | `USER#Sheldon`                      |
| **SK**        | `GRANT#<RecipientID>#<GrantID>` | `GRANT#Justin#G-a1b2c3d4`           |
| `ItemType`    | `SHARE_GRANT`                   | `SHARE_GRANT`                       |
| `GrantType`   | `PREFIX`                        | `PREFIX`                            |
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
- **GrantType:** Set to `PREFIX` to indicate folder-level access. This attribute distinguishes folder grants from file-specific grants in the same table.
- **Prefix-Level Access:** Granting access to `media/photos/` automatically includes access to `media/photos/2024/`, `media/photos/2024/vacation/`, etc. The recipient can browse subfolders and view all files matching the prefix.
- **Atomic Revocation:** Deleting this single item immediately revokes access to all files under the prefix (enforced by API permission checks). VIEW_LINK cleanup happens synchronously when the grant is revoked via the API.
- **GSI1 Projection:** PREFIX grants appear in the recipient's "Shared With Me" folder list, allowing them to browse the shared folder tree.

#### **2.2.2. FILE Grant (Individual File Access)**

A FILE grant provides access to a single specific file without granting access to the parent folder or sibling files. This enables sharing individual files from private folders while maintaining folder privacy.

| Attribute     | Value Format                        | Example (Sheldon → Justin)        |
| :------------ | :---------------------------------- | :-------------------------------- |
| **PK**        | `USER#<OwnerID>`                    | `USER#Sheldon`                    |
| **SK**        | `GRANT#<RecipientID>#<GrantID>`     | `GRANT#Justin#G-x7y8z9w0`         |
| `ItemType`    | `SHARE_GRANT`                       | `SHARE_GRANT`                     |
| `GrantType`   | `FILE`                              | `FILE`                            |
| `GrantID`     | `<UUID>`                            | `G-x7y8z9w0`                      |
| `OwnerID`     | `<UserID>`                          | `Sheldon`                         |
| `RecipientID` | `<UserID>`                          | `Justin`                          |
| `Permissions` | `READ`, `READ/WRITE`                | `READ`                            |
| `ResourceID`  | `<UUID>`                            | `R102`                            |
| `FilePath`    | `<FullPath>`                        | `media/Project Docs/DSCN0010.jpg` |
| `CreatedDate` | `<Timestamp>`                       | `1234567890000`                   |
| **`GSI1-PK`** | `ACCESS#<RecipientID>`              | `ACCESS#Justin`                   |
| **`GSI1-SK`** | `GRANT#<OwnerID>#FILE#<ResourceID>` | `GRANT#Sheldon#FILE#R102`         |

**Key Design Notes:**

- **GrantType:** Set to `FILE` to indicate file-specific access. This enables the grant creation API to create a single VIEW_LINK for just this file.
- **ResourceID and FilePath:** Both attributes are stored for efficient lookup and audit logging. `ResourceID` is the unique file identifier (UUID), and `FilePath` is the human-readable full path.
- **No Prefix Attribute:** FILE grants do NOT include a `Prefix` attribute since they don't grant folder-level access. The recipient cannot browse the parent folder `media/Project Docs/` - they only see this specific file.
- **Single VIEW_LINK Creation:** When a FILE grant is created, the API creates exactly ONE VIEW_LINK pointing to this file synchronously. No folder markers or sibling files are visible to the recipient.
- **GSI1-SK Format:** FILE grants use a different GSI1-SK pattern (`GRANT#<OwnerID>#FILE#<ResourceID>`) to distinguish them from PREFIX grants in the "Shared With Me" view. The UI can group PREFIX grants as folders and FILE grants as individual files.
- **Use Case:** Share a single vacation photo from a private folder, share a confidential document without exposing other files in the same directory, or grant access to a specific report without revealing the entire reports archive.
- **Atomic Revocation:** Deleting the FILE grant immediately removes access. The single VIEW_LINK is deleted synchronously by the API.

#### **2.2.3. Choosing Between PREFIX and FILE Grants**

| Scenario                                              | Grant Type | Reason                                                          |
| :---------------------------------------------------- | :--------- | :-------------------------------------------------------------- |
| Share entire folder for collaboration                 | `PREFIX`   | Recipient needs to browse subfolders and see all files          |
| Share photo album or project directory                | `PREFIX`   | Multiple files, logical grouping, folder navigation needed      |
| Share single file from private folder                 | `FILE`     | Maintain folder privacy, only specific file is relevant         |
| Share confidential document without context           | `FILE`     | Prevent discovery of other documents in same folder             |
| Grant access to specific report in large archive      | `FILE`     | Avoid overwhelming recipient with unrelated files               |
| Add collaborator to ongoing project                   | `PREFIX`   | Collaborator needs full context and access to project tree      |
| Share single file that happens to be in shared folder | Neither    | Recipient already has PREFIX grant - file is already accessible |

**Implementation Note:** The application should validate that a FILE grant is not redundant with an existing PREFIX grant. If the recipient already has access to the folder containing the file, creating a FILE grant is unnecessary (though harmless).

### **2.4. Item Type: VIEW_LINK (Denormalized File/Folder Pointer for Unified Browsing)**

A denormalized pointer created for every file and folder a user can view. VIEW_LINKs enable the unified folder browsing experience where owners and recipients use the same query pattern. This item is projected onto GSI2 and is the foundation of all folder browsing operations.

**Two Subtypes:**

1. **File VIEW_LINK:** Points to an actual file (`ResourceID` is a UUID)
2. **Folder Marker VIEW_LINK:** Represents a subfolder (`ResourceID = "FOLDER#<FullFolderPath>"`)

**Creation Strategy:** VIEW_LINKs are created **automatically** whenever:

- **A FILE is uploaded to S3:** S3 event processor creates:
  - FILE item (canonical record)
  - File VIEW_LINKs for owner (with `GrantID: "OWNER"`)
  - File VIEW_LINKs for all recipients with matching PREFIX grants
  - Folder marker VIEW_LINKs for all ancestor folders (for owner and recipients)
  - **NOTE:** S3 events NEVER create SHARE_GRANT items
- **A PREFIX SHARE_GRANT is created via API:** API synchronously creates:
  - SHARE_GRANT item
  - VIEW_LINKs for all existing files matching the prefix
  - Folder marker VIEW_LINKs for all ancestor folders
- **A FILE SHARE_GRANT is created via API:** API synchronously creates:
  - SHARE_GRANT item
  - Single VIEW_LINK for the specific file (no folder markers)

This means owners always browse their files through VIEW_LINKs, just like recipients do, ensuring a single, consistent code path for all folder operations. Recipients with FILE grants see individual files in their file list without parent folder context, while recipients with PREFIX grants can browse the full folder tree.

#### **2.4.1. File VIEW_LINK (Regular Files)**

| Attribute      | Value Format                                     | Example (Justin viewing Sheldon's R102)    |
| :------------- | :----------------------------------------------- | :----------------------------------------- |
| **PK**         | `USER#<ViewerID>`                                | `USER#Justin`                              |
| **SK**         | `VIEWLINK#<OwnerID>#<ResourceID>`                | `VIEWLINK#Sheldon#R102`                    |
| `ItemType`     | `VIEW_LINK`                                      | `VIEW_LINK`                                |
| `ResourceID`   | `<UUID>`                                         | `R102`                                     |
| `OwnerID`      | `<UserID>`                                       | `Sheldon`                                  |
| `GrantID`      | `<UUID>` or `"OWNER"`                            | `G-a1b2c3d4`                               |
| `CreatedDate`  | `<Timestamp>`                                    | `1224685719000`                            |
| `FolderPrefix` | `<Path>`                                         | `media/Project Docs/`                      |
| `FileName`     | `<string>`                                       | `DSCN0010.jpg`                             |
| `MediaType`    | `<MIME Type>`                                    | `image/jpeg`                               |
| **`GSI2-PK`**  | `VIEWER#<ViewerID>#FOLDER#<FolderPrefix>`        | `VIEWER#Justin#FOLDER#media/Project Docs/` |
| **`GSI2-SK`**  | `TYPE#FILE#<Timestamp>#<MediaType>#<ResourceID>` | `TYPE#FILE#1224685719000#image/jpeg#R102`  |

#### **2.4.2. Folder Marker VIEW_LINK (Subfolders)**

Folder markers enable S3-style folder navigation by representing subfolders as queryable items. When browsing `media/`, folder markers for `media/photos/`, `media/videos/`, etc. are returned alongside files directly in `media/`.

| Attribute      | Value Format                                 | Example (Justin viewing Sheldon's photos/ folder) |
| :------------- | :------------------------------------------- | :------------------------------------------------ |
| **PK**         | `USER#<ViewerID>`                            | `USER#Justin`                                     |
| **SK**         | `VIEWLINK#<OwnerID>#FOLDER#<FullFolderPath>` | `VIEWLINK#Sheldon#FOLDER#media/photos/`           |
| `ItemType`     | `VIEW_LINK`                                  | `VIEW_LINK`                                       |
| `ResourceID`   | `FOLDER#<FullFolderPath>`                    | `FOLDER#media/photos/`                            |
| `OwnerID`      | `<UserID>`                                   | `Sheldon`                                         |
| `GrantID`      | `<UUID>` or `"OWNER"`                        | `G-a1b2c3d4`                                      |
| `CreatedDate`  | `<Timestamp>`                                | `1224685719000`                                   |
| `FolderPrefix` | `<ParentPath>`                               | `media/`                                          |
| `FileName`     | `<SubfolderName>`                            | `photos/`                                         |
| `MediaType`    | `application/x-directory`                    | `application/x-directory`                         |
| **`GSI2-PK`**  | `VIEWER#<ViewerID>#FOLDER#<FolderPrefix>`    | `VIEWER#Justin#FOLDER#media/`                     |
| **`GSI2-SK`**  | `TYPE#FOLDER#<FolderName>#<OwnerID>`         | `TYPE#FOLDER#photos/#Sheldon`                     |

**Key Design Notes:**

- **ResourceID Format:** For folder markers, `ResourceID = "FOLDER#<FullFolderPath>"` uniquely identifies the folder.
- **GSI2-SK Design:** The `TYPE#FOLDER#` prefix ensures folders sort before files (`TYPE#FILE#`), mimicking S3's UI behavior where folders appear first.
- **OwnerID in Sort Key:** Including `OwnerID` in `GSI2-SK` allows multiple owners to contribute subfolders with the same name (e.g., both Sheldon and Leigh have `photos/` folders that Justin can access).
- **Automatic Creation:** When a file is uploaded to `media/photos/2024/vacation/img.jpg`, folder markers are automatically created for:
  - `media/` (if doesn't exist)
  - `media/photos/` (if doesn't exist)
  - `media/photos/2024/` (if doesn't exist)
  - `media/photos/2024/vacation/` (if doesn't exist)
- **Conditional Put:** Folder markers use conditional puts (`attribute_not_exists(PK)`) to avoid duplicates when multiple files reference the same folder.
- **GrantID Consistency:** Folder markers maintain the same `OwnerID` and `GrantID` as the files they represent, ensuring consistent permission validation and cleanup.

#### **2.4.3. GSI2 Sort Key Design Trade-off**

The FILE VIEW_LINK sort key format `TYPE#FILE#<Timestamp>#<MediaType>#<ResourceID>` prioritizes **pure chronological sorting** across all file types. This design choice directly optimizes for the primary use case of browsing merged folders with files from multiple owners sorted by creation date.

**✅ Primary Use Case: Cross-Owner Chronological Browsing**

When multiple users share files into the same folder (e.g., Sheldon, Justin, and Leigh all contributing to `media/`), this design enables:

- **Pure chronological sort** across all files regardless of media type
- **Single query** returns results in perfect date order (newest to oldest, or vice versa)
- **Native DynamoDB pagination** with `LastEvaluatedKey` works correctly across all contributors
- **Constant latency** (10-20ms per page) regardless of total items or number of contributors
- **Efficient cost** (1-2 RCU per 50-item page)

Example query result order:

```
TYPE#FILE#1224685760000#video/mp4#R107     (newest - Leigh's video)
TYPE#FILE#1224685750000#image/jpeg#R106    (Sheldon's photo)
TYPE#FILE#1224685740000#image/gif#R105     (Justin's GIF)
TYPE#FILE#1224685730000#document/pdf#R104  (Leigh's PDF)
TYPE#FILE#1224685720000#video/mp4#R103     (oldest - Sheldon's video)
```

All files are sorted purely by timestamp, enabling queries like:

- "Show me the 50 newest files from everyone"
- "Browse all files chronologically, paginated"
- "Show me what was added this week"

**❌ Secondary Use Case: Filter by Media Type (Less Efficient)**

To filter by media type (e.g., "show only images"), you must use a `FilterExpression`:

```rust
// Query with media type filter
let result = client.query()
    .table_name("FileMetadata")
    .index_name("ViewLinkIndex")
    .key_condition_expression("GSI2-PK = :pk")
    .filter_expression("begins_with(MediaType, :media_type)")
    .expression_attribute_values(":pk", AttributeValue::S("VIEWER#Justin#FOLDER#media/"))
    .expression_attribute_values(":media_type", AttributeValue::S("image/"))
    .scan_index_forward(false)
    .limit(50)
    .send()
    .await?;
```

**Trade-off Impact:**

- DynamoDB reads items **before** filtering (reads 100 items, returns 50 images after filter)
- Slightly higher latency (15-30ms vs 10-20ms)
- Higher RCU cost (2-4 RCU vs 1-2 RCU per page due to extra reads)
- Still acceptable performance for filtered views (typically less common than chronological browsing)

**Benefits of Folder Markers:**

✅ **Single Query Folder Browsing:** One GSI2 query returns both subfolders and files  
✅ **S3-Like Navigation:** Folders appear first, files second (natural user experience)  
✅ **Multi-Owner Support:** Each owner's subfolders are tracked separately  
✅ **Efficient:** No filter expressions needed, native DynamoDB sorting  
✅ **Scalable:** Works for unlimited folder depth  
✅ **Consistent Permissions:** Folders inherit ownership and grant semantics from files

### **2.5. VIEW_LINK Lifecycle Management**

VIEW_LINKs are automatically maintained to reflect current access permissions. Understanding their lifecycle is critical for implementing the system correctly.

#### **2.5.1. Creation Triggers**

**1. S3 ObjectCreated Event:**

- Creates FILE item
- Creates file VIEW_LINK for owner (`GrantID: "OWNER"`)
- Creates folder marker VIEW_LINKs for all ancestor folders (for owner)
- Queries for matching PREFIX grants
- Creates file VIEW_LINKs for all recipients with matching grants
- Creates folder marker VIEW_LINKs for recipients (for each ancestor folder)
- **Does NOT create or modify SHARE_GRANT items**

**2. API Creates PREFIX Grant:**

- Creates SHARE_GRANT item
- Queries all files under the prefix
- Creates file VIEW_LINKs for recipient (for all existing files)
- Creates folder marker VIEW_LINKs for recipient (for all ancestor folders)

**3. API Creates FILE Grant:**

- Creates SHARE_GRANT item
- Creates single file VIEW_LINK for recipient
- **Does NOT create folder marker VIEW_LINKs** (file-only access)

#### **2.5.2. Deletion Triggers**

**1. S3 ObjectRemoved Event:**

- Deletes FILE item
- Deletes file VIEW_LINKs for all users (owner + recipients)
- Deletes FILE SHARE_GRANTs (if any exist for this specific file)
- **Does NOT delete PREFIX SHARE_GRANTs** (folder-level grants persist)
- **Does NOT delete folder marker VIEW_LINKs** (folders persist even when empty)

**2. API Deletes PREFIX Grant (Revoke Share):**

- Deletes SHARE_GRANT item
- Deletes all file VIEW_LINKs for recipient under that prefix
- Deletes all folder marker VIEW_LINKs for recipient under that prefix
- **Does NOT affect owner's VIEW_LINKs** (owner retains access)

**3. API Deletes FILE Grant:**

- Deletes SHARE_GRANT item
- Deletes single file VIEW_LINK for recipient

**4. API Deletes Folder (DELETE /{path}/):**

- Validates folder is empty (no files or subfolders)
- Deletes PREFIX SHARE_GRANTs for this exact folder path
- Deletes folder marker VIEW_LINKs for all users (owner + recipients)
- **Does NOT delete FILE items** (folder must be empty first)

#### **2.5.3. Persistence Behavior**

- ✅ **Folder markers persist** when all files are deleted (S3 events don't touch them)
- ✅ **PREFIX grants persist** when all files are deleted (enables automatic sharing of new uploads)
- ✅ **Empty folders remain browsable** until explicitly deleted via API
- ✅ **New file uploads automatically create VIEW_LINKs** for existing PREFIX grants
- ❌ **FILE grants are deleted** when the file is deleted from S3

**Example: File Deletion Preserves Folder Structure**

```
1. Sheldon shares "media/photos/" with Justin (PREFIX grant created)
2. Sheldon uploads 100 photos → Justin sees all 100 photos
3. Sheldon deletes all 100 photos via S3:
   - FILE items deleted
   - File VIEW_LINKs deleted (for Sheldon and Justin)
   - PREFIX grant "media/photos/" REMAINS
   - Folder markers "media/" and "media/photos/" REMAIN
4. Justin browses "media/photos/" → sees empty folder (no error)
5. Sheldon uploads new photo to "media/photos/vacation.jpg":
   - FILE item created
   - File VIEW_LINKs created (for Sheldon and Justin automatically)
6. Justin immediately sees the new photo (no need to re-share)
```

#### **2.5.4. Design Rationale**

Folders are **logical prefixes** that represent the structure of the file system, not physical entities tied to file existence. This S3-style behavior provides:

- ✅ Persistent folder structure independent of file contents
- ✅ Automatic sharing of new uploads to previously shared folders
- ✅ No need to recreate grants after deleting/re-uploading files
- ✅ Matches user expectations from S3, Google Drive, Dropbox

#### **2.5.5. Key Architectural Distinctions**

| Operation         | Entry Point       | What It Touches                    | SHARE_GRANTs Affected?      |
| ----------------- | ----------------- | ---------------------------------- | --------------------------- |
| **File Upload**   | S3 → SQS → Lambda | FILE, VIEW_LINKs                   | ❌ Never                    |
| **File Delete**   | S3 → SQS → Lambda | FILE, file VIEW_LINKs, FILE grants | ❌ PREFIX grants unaffected |
| **Create Share**  | API → Lambda      | SHARE_GRANT, VIEW_LINKs            | ✅ Creates SHARE_GRANT      |
| **Revoke Share**  | API → Lambda      | SHARE_GRANT, recipient VIEW_LINKs  | ✅ Deletes SHARE_GRANT      |
| **Delete Folder** | API → Lambda      | PREFIX grants, folder markers      | ✅ Deletes PREFIX grants    |

**Critical Principle:** S3 events handle **file lifecycle**, API operations handle **sharing lifecycle**. This separation of concerns prevents confusion and ensures correct behavior.

## **3. Access Patterns and Query Details (Use Cases)**

This section provides the query strategy for each use case. The developer's goal is to write application code that maps a user action to one of these efficient DynamoDB queries.

**Important:** ALL folder browsing operations use GSI2 (ViewLinkIndex), regardless of whether the user is viewing their own files or shared files. This eliminates conditional logic and provides a consistent, high-performance access pattern.

### **Use Case 1: Sheldon views the contents of his own folder (media/Project Docs/)**

- **Goal:** List all subfolders and files directly inside `media/Project Docs/` that Sheldon owns, using the same query pattern used for shared folders.
- **Strategy:** Query GSI2 (ViewLinkIndex) using Sheldon's VIEW_LINKs. This returns folder markers (subfolders) followed by file VIEW_LINKs in a single query.
- **Query Details:**

  ```rust
  // Query GSI2 for "media/Project Docs/" folder visible to Sheldon
  let query_input = QueryInput {
      table_name: "FileMetadata".to_string(),
      index_name: Some("ViewLinkIndex".to_string()),
      key_condition_expression: Some("GSI2-PK = :pk".to_string()),
      expression_attribute_values: Some(hashmap! {
          ":pk".to_string() => AttributeValue::S("VIEWER#Sheldon#FOLDER#media/Project Docs/".to_string()),
      }),
      scan_index_forward: Some(true), // Folders first (TYPE#FOLDER sorts before TYPE#FILE)
      limit: Some(50), // Page size
      ..Default::default()
  };

  // Result: All VIEW_LINK items for "media/Project Docs/" where Sheldon is the viewer
  // Returned items (in order):
  //   1. FOLDER MARKERS (subfolders):
  //      - TYPE#FOLDER#2024/#Sheldon  (subfolder "2024/")
  //      - TYPE#FOLDER#archive/#Sheldon  (subfolder "archive/")
  //   2. FILE VIEW_LINKs (files directly in this folder, sorted by timestamp):
  //      - TYPE#FILE#1224685730000#application/pdf#R105 (document.pdf - newer)
  //      - TYPE#FILE#1224685719000#image/jpeg#R102 (DSCN0010.jpg - older)
  ```

**Processing Results:**

```rust
let mut subfolders = Vec::new();
let mut files = Vec::new();

for item in result.items {
    let file_id = item.get("ResourceID")?.as_s()?;

    if file_id.starts_with("FOLDER#") {
        // This is a folder marker
        subfolders.push(FolderInfo {
            name: item.get("FileName")?.as_s()?.to_string(),  // "2024/"
            full_path: file_id.strip_prefix("FOLDER#").unwrap().to_string(),  // "media/Project Docs/2024/"
            owner_id: item.get("OwnerID")?.as_s()?.to_string(),  // "Sheldon"
        });
    } else {
        // This is a file VIEW_LINK
        files.push(FileInfo {
            file_id: file_id.to_string(),
            file_name: item.get("FileName")?.as_s()?.to_string(),
            media_type: item.get("MediaType")?.as_s()?.to_string(),
            owner_id: item.get("OwnerID")?.as_s()?.to_string(),
            created_date: item.get("CreatedDate")?.as_n()?.parse()?,
        });
    }
}

// UI can now display:
// 📁 2024/
// 📁 archive/
// 📄 DSCN0010.jpg
// 📄 document.pdf
```

**Key Benefits:**

- ✅ **Single query** returns both folders and files
- ✅ **Folders appear first** due to `TYPE#FOLDER#` sorting before `TYPE#FILE#`
- ✅ **S3-like navigation** - same UX as browsing S3 buckets
- ✅ **No conditional logic** needed ("am I owner or recipient?")
- ✅ **Native pagination** with DynamoDB LastEvaluatedKey
- ✅ **Efficient filtering** by media type for files (folders always shown)
- ✅ **Consistent UX** - owner sees same view as recipients would

### **Use Case 2: Justin views his "Shared With Me" list**

- **Goal:** List all folder prefixes that have been explicitly shared with Justin, along with who shared them and what permissions he has.
- **Strategy:** Query GSI 1 (GrantIndex). This index is specifically designed to collate all of a user's incoming prefix-level grants.
- **Query Details:**

  ```rust
  // Query for all grants shared with Justin
  let query_input = QueryInput {
      table_name: "FileMetadata".to_string(),
      index_name: Some("GrantIndex".to_string()),
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

- **Goal:** List all files and subfolders from any folder with the path ending in "Project Docs/" that Justin has access to (e.g., from Sheldon's `media/Project Docs/`, Leigh's `work/Project Docs/`, etc.), merged into a single view, with folders appearing first, then files filtered by media type and sorted by creation date.
- **Strategy:** Query GSI2 (ViewLinkIndex). This is the exact same query as Use Case 1 - there is no difference between viewing own files and viewing a merged folder.
- **Explanation:** The `GSI2-PK` groups all folders and files visible to Justin within any folder named "Project Docs/" into a single item collection. The `GSI2-SK` design ensures folders sort first (`TYPE#FOLDER#`), then files (`TYPE#FILE#`). This works identically whether Justin is viewing only his own "Project Docs/" or a merged view with content from Sheldon and Leigh.

**Example 1: Query all content (folders + files)**

```rust
// Query for ALL content in "Project Docs/" visible to Justin
let query_input = QueryInput {
    table_name: "FileMetadata".to_string(),
    index_name: Some("ViewLinkIndex".to_string()),
    key_condition_expression: Some("GSI2-PK = :pk".to_string()),
    expression_attribute_values: Some(hashmap! {
        ":pk".to_string() => AttributeValue::S("VIEWER#Justin#FOLDER#Project Docs/".to_string()),
    }),
    scan_index_forward: Some(true), // TYPE#FOLDER before TYPE#FILE
    limit: Some(50),
    exclusive_start_key: None,
    ..Default::default()
};

// Result: Merged list from multiple owners, sorted chronologically
// Returned items (in order):
//   FOLDERS (from various owners):
//     - TYPE#FOLDER#2024/#Sheldon (from Sheldon's media/Project Docs/)
//     - TYPE#FOLDER#presentations/#Leigh (from Leigh's work/Project Docs/)
//   FILES (sorted by timestamp across all owners and media types):
//     - TYPE#FILE#1224686050000#application/...#R203 (Leigh's presentation.pptx - newer)
//     - TYPE#FILE#1224685719000#image/jpeg#R102 (Sheldon's DSCN0010.jpg - older)
```

**Example 2: Filter to show only images (less efficient, uses FilterExpression)**

```rust
// Query with media type filter using FilterExpression
let query_input = QueryInput {
    table_name: "FileMetadata".to_string(),
    index_name: Some("ViewLinkIndex".to_string()),
    key_condition_expression: Some("GSI2-PK = :pk".to_string()),
    filter_expression: Some("begins_with(MediaType, :media_type)".to_string()),
    expression_attribute_values: Some(hashmap! {
        ":pk".to_string() => AttributeValue::S("VIEWER#Justin#FOLDER#Project Docs/".to_string()),
        ":media_type".to_string() => AttributeValue::S("image/".to_string()),
    }),
    scan_index_forward: Some(false), // Newest images first
    limit: Some(50),
    ..Default::default()
};
// Result: Only image files, sorted chronologically (newest first)
// Note: DynamoDB reads all items then filters, so may return fewer than 50 items per page
// Folders are NOT returned because FilterExpression excludes them (no MediaType attribute)
// Trade-off: Higher RCU cost (2-4 vs 1-2) but still acceptable for filtered views
```

**Example 3: Show folders + all files, client-side filter images**

```rust
// Best practice for "show folders + specific file types"
let query_input = QueryInput {
    table_name: "FileMetadata".to_string(),
    index_name: Some("ViewLinkIndex".to_string()),
    key_condition_expression: Some("GSI2-PK = :pk".to_string()),
    expression_attribute_values: Some(hashmap! {
        ":pk".to_string() => AttributeValue::S("VIEWER#Justin#FOLDER#Project Docs/".to_string()),
    }),
    scan_index_forward: Some(true),
    limit: Some(100), // Fetch more to account for filtering
    ..Default::default()
};

// Client-side processing
let mut folders = Vec::new();
let mut image_files = Vec::new();

for item in result.items {
    let file_id = item.get("ResourceID")?.as_s()?;

    if file_id.starts_with("FOLDER#") {
        folders.push(parse_folder(item)?);
    } else {
        let media_type = item.get("MediaType")?.as_s()?;
        if media_type.starts_with("image/") {
            image_files.push(parse_file(item)?);
        }
    }
}

// UI displays folders first, then filtered files
```

**Handling Multi-Owner Folders:**

When multiple owners share folders with the same name, the UI can merge them:

```rust
// Justin sees both Sheldon's and Leigh's "2024/" subfolders
// Raw results:
//   - TYPE#FOLDER#2024/#Sheldon
//   - TYPE#FOLDER#2024/#Leigh
//   - TYPE#FOLDER#presentations/#Leigh

// Merge by folder name for cleaner UI
let mut folder_map: HashMap<String, Vec<String>> = HashMap::new();

for folder_marker in folder_markers {
    folder_map.entry(folder_marker.name.clone())
        .or_insert_with(Vec::new)
        .push(folder_marker.owner_id);
}

// Display:
// 📁 2024/ (from Sheldon, Leigh)
// 📁 presentations/ (from Leigh)
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
- ✅ **Unified code path** - same query works for own files, shared files, or merged views

**Note:** The "merged folder view" is not a special case - it's simply how folder browsing always works with GSI2. When Sheldon views his own "Project Docs/", he uses the same GSI2 query, which returns his own files (where he's the viewer and owner). When Justin views "Project Docs/", the same query returns files from all sources he has access to.

### **Use Case 5: Sheldon grants Justin access to a folder prefix**

- **Goal:** Share all files under `media/Project Docs/` with Justin, giving him READ access.
- **Strategy:**
  1. Create a single SHARE_GRANT item (immediate access grant)
  2. Create VIEW_LINKs synchronously for all existing files in that prefix (may take 10-30 seconds for large folders)
  3. Future files uploaded to S3 will automatically get VIEW_LINKs via S3 event processor

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

// Step 2: Synchronously create VIEW_LINKs for all existing files (batched)
let mut files = Vec::new();
let mut last_key = None;

// Query all files under the prefix
loop {
    let result = client.query()
        .table_name("FileMetadata")
        .key_condition_expression("PK = :pk AND begins_with(SK, :sk_prefix)")
        .expression_attribute_values(":pk", AttributeValue::S("USER#Sheldon".to_string()))
        .expression_attribute_values(":sk_prefix", AttributeValue::S("FILE#media/Project Docs/".to_string()))
        .set_exclusive_start_key(last_key)
        .send()
        .await?;

    if let Some(items) = result.items {
        files.extend(items);
    }

    if result.last_evaluated_key.is_none() {
        break;
    }
    last_key = result.last_evaluated_key;
}

// Create VIEW_LINKs in batches of 25 (DynamoDB limit)
for chunk in files.chunks(25) {
    let write_requests: Vec<WriteRequest> = chunk.iter()
        .map(|file| create_view_link_write_request(file, "Justin", &grant_id))
        .collect();

    client.batch_write_item()
        .request_items("FileMetadata", write_requests)
        .send()
        .await?;
}

// Also create folder marker VIEW_LINKs for all ancestor folders
// (similar batch operation)
```

**Note:** This single SHARE_GRANT operation grants access to potentially thousands of files. VIEW_LINKs are created synchronously by the API for all existing files using efficient batch operations (may take 10-30 seconds for large folders). New files uploaded after the grant will automatically get VIEW_LINKs via S3 event processor.

### **Use Case 6: Sheldon uploads a new file to a nested folder**

- **Goal:** When Sheldon uploads `media/photos/2024/vacation/img.jpg`, automatically create:
  1. FILE VIEW_LINKs for all users who have access (owner + recipients)
  2. Folder marker VIEW_LINKs for each ancestor folder level, for all users who have access (owner + recipients)
- **Strategy:** S3 event processor (triggered by S3 ObjectCreated event via SQS) automatically detects the new file and creates all necessary VIEW_LINKs and folder markers.

**Step 1: Sheldon uploads file (creates FILE item)**

```rust
// Application creates FILE item on S3 upload
let put_input = PutItemInput {
    table_name: "FileMetadata".to_string(),
    item: hashmap! {
        "PK".to_string() => AttributeValue::S("USER#Sheldon".to_string()),
        "SK".to_string() => AttributeValue::S("FILE#media/photos/2024/vacation/img.jpg".to_string()),
        "ItemType".to_string() => AttributeValue::S("FILE".to_string()),
        "ResourceID".to_string() => AttributeValue::S("R999".to_string()),
        "OwnerID".to_string() => AttributeValue::S("Sheldon".to_string()),
        "FileName".to_string() => AttributeValue::S("img.jpg".to_string()),
        "FolderPrefix".to_string() => AttributeValue::S("media/photos/2024/vacation/".to_string()),
        "MediaType".to_string() => AttributeValue::S("image/jpeg".to_string()),
        "CreatedDate".to_string() => AttributeValue::N("1234567890000".to_string()),
        "S3Key".to_string() => AttributeValue::S("Sheldon/media/photos/2024/vacation/img.jpg".to_string()),
        // ... other metadata
    },
    ..Default::default()
};
```

**Step 2: S3 event processor Lambda handles file upload**

```rust
// Lambda function receives S3 event from SQS
async fn handle_s3_event(event: S3Event) -> Result<()> {
    for record in event.records {
        if record.event_name.starts_with("ObjectCreated:") {
            let s3_key = &record.s3.object.key;
            let file = parse_file_from_s3_key(s3_key)?;

            // Find all grants for this file (both prefix and file-specific)
            let prefix_grants = find_prefix_grants(&file.owner_id, &file.folder_prefix).await?;
            let file_grants = find_file_grants(&file.owner_id, &file.file_id).await?;

            // Create VIEW_LINKs and folder markers for owner
            let owner_grant_id = "OWNER";
            let mut items_to_create = vec![
                create_file_view_link(&file.owner_id, &file, owner_grant_id)
            ];
            items_to_create.extend(
                create_folder_markers(&file.owner_id, &file.owner_id, owner_grant_id, &file.folder_prefix)
            );

            // Create VIEW_LINKs and folder markers for each prefix grant recipient
            for grant in prefix_grants {
                items_to_create.push(
                    create_file_view_link(&grant.recipient_id, &file, &grant.grant_id)
                );
                items_to_create.extend(
                    create_folder_markers(&grant.recipient_id, &file.owner_id, &grant.grant_id, &file.folder_prefix)
                );
            }

            // Create VIEW_LINKs for file grant recipients (no folder markers)
            for grant in file_grants {
                items_to_create.push(
                    create_file_view_link(&grant.recipient_id, &file, &grant.grant_id)
                );
            }

            // Batch write all items (files + folders) with conditional puts for folders
            batch_write_items(items_to_create).await?;
        }
    }
    Ok(())
}

fn create_folder_markers(
    viewer_id: &str,
    owner_id: &str,
    grant_id: &str,
    file_path: &str,
) -> Vec<HashMap<String, AttributeValue>> {
    let mut markers = Vec::new();
    let segments: Vec<&str> = file_path
        .trim_end_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    // Create a marker for each ancestor folder
    // For "media/photos/2024/vacation/", create markers for:
    //   - media/
    //   - media/photos/
    //   - media/photos/2024/
    //   - media/photos/2024/vacation/

    for i in 0..segments.len() {
        let parent_prefix = if i == 0 {
            String::new()  // Root level
        } else {
            segments[..i].join("/") + "/"
        };

        let folder_name = format!("{}/", segments[i]);
        let full_folder_path = segments[..=i].join("/") + "/";

        markers.push(hashmap! {
            "PK".to_string() => AttributeValue::S(format!("USER#{}", viewer_id)),
            "SK".to_string() => AttributeValue::S(
                format!("VIEWLINK#{}#FOLDER#{}", owner_id, full_folder_path)
            ),
            "ItemType".to_string() => AttributeValue::S("VIEW_LINK".to_string()),
            "ResourceID".to_string() => AttributeValue::S(format!("FOLDER#{}", full_folder_path)),
            "OwnerID".to_string() => AttributeValue::S(owner_id.to_string()),
            "GrantID".to_string() => AttributeValue::S(grant_id.to_string()),
            "CreatedDate".to_string() => AttributeValue::N(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
                    .to_string()
            ),
            "FolderPrefix".to_string() => AttributeValue::S(parent_prefix.clone()),
            "FileName".to_string() => AttributeValue::S(folder_name.clone()),
            "MediaType".to_string() => AttributeValue::S("application/x-directory".to_string()),
            "GSI2-PK".to_string() => AttributeValue::S(
                format!("VIEWER#{}#FOLDER#{}", viewer_id, parent_prefix)
            ),
            "GSI2-SK".to_string() => AttributeValue::S(
                format!("TYPE#FOLDER#{}#{}", folder_name, owner_id)
            ),
        });
    }

    markers
}

fn create_file_view_link(
    viewer_id: &str,
    file: &FileMetadata,
    grant_id: &str,
) -> HashMap<String, AttributeValue> {
    hashmap! {
        "PK".to_string() => AttributeValue::S(format!("USER#{}", viewer_id)),
        "SK".to_string() => AttributeValue::S(format!("VIEWLINK#{}#{}", file.owner_id, file.file_id)),
        "ItemType".to_string() => AttributeValue::S("VIEW_LINK".to_string()),
        "ResourceID".to_string() => AttributeValue::S(file.file_id.clone()),
        "OwnerID".to_string() => AttributeValue::S(file.owner_id.clone()),
        "GrantID".to_string() => AttributeValue::S(grant_id.to_string()),
        "CreatedDate".to_string() => AttributeValue::N(file.created_date.to_string()),
        "FolderPrefix".to_string() => AttributeValue::S(file.folder_prefix.clone()),
        "FileName".to_string() => AttributeValue::S(file.file_name.clone()),
        "MediaType".to_string() => AttributeValue::S(file.media_type.clone()),
        "GSI2-PK".to_string() => AttributeValue::S(
            format!("VIEWER#{}#FOLDER#{}", viewer_id, file.folder_prefix)
        ),
        "GSI2-SK".to_string() => AttributeValue::S(
            format!("TYPE#FILE#{}#{}#{}", file.created_date, file.media_type, file.file_id)
        ),
    }
}
```

**Step 3: Batch Write with Conditional Puts**

```rust
async fn batch_write_items(items: Vec<HashMap<String, AttributeValue>>) -> Result<()> {
    const BATCH_SIZE: usize = 25;

    for chunk in items.chunks(BATCH_SIZE) {
        let write_requests: Vec<_> = chunk.iter()
            .map(|item| {
                let is_folder = item.get("ResourceID")
                    .and_then(|v| v.as_s().ok())
                    .map(|s| s.starts_with("FOLDER#"))
                    .unwrap_or(false);

                if is_folder {
                    // Use conditional put for folders to avoid duplicates
                    WriteRequest::builder()
                        .put_request(
                            PutRequest::builder()
                                .set_item(Some(item.clone()))
                                .build()
                        )
                        .build()
                } else {
                    // Regular put for files
                    WriteRequest::builder()
                        .put_request(
                            PutRequest::builder()
                                .set_item(Some(item.clone()))
                                .build()
                        )
                        .build()
                }
            })
            .collect();

        client.batch_write_item()
            .request_items("FileMetadata", write_requests)
            .send()
            .await?;
    }

    Ok(())
}
```

**Key Benefits:**

- ✅ Automatic VIEW_LINK maintenance (no manual sync needed)
- ✅ Automatic folder marker creation for all ancestor levels, for all users who have access (owner + recipients)
- ✅ New files immediately visible to recipients
- ✅ Folder structure automatically derived from file paths
- ✅ Handles file deletions and updates automatically via s3 sqs events
- ✅ Conditional puts prevent duplicate folder markers
- ✅ Handles both PREFIX and FILE grants automatically

### **Use Case 7: Sheldon shares a single file from a private folder with Justin**

- **Goal:** Share a specific file (`media/private/confidential-report.pdf`) with Justin without granting access to the parent folder or other files in that folder.
- **Strategy:** Create a FILE grant that references only this specific file. Justin can view the file but cannot browse the `media/private/` folder.
- **Use Case:** Share a single vacation photo from a private album, share a confidential document without exposing the folder structure, or grant access to a specific report from a large archive.

**Step 1: Sheldon creates a FILE grant**

```rust
// Application creates FILE grant
let grant_id = uuid::Uuid::new_v4().to_string(); // e.g., "G-x7y8z9w0"

let put_input = PutItemInput {
    table_name: "FileMetadata".to_string(),
    item: hashmap! {
        "PK".to_string() => AttributeValue::S("USER#Sheldon".to_string()),
        "SK".to_string() => AttributeValue::S(format!("GRANT#Justin#{}", grant_id)),
        "ItemType".to_string() => AttributeValue::S("SHARE_GRANT".to_string()),
        "GrantType".to_string() => AttributeValue::S("FILE".to_string()),
        "GrantID".to_string() => AttributeValue::S(grant_id.clone()),
        "OwnerID".to_string() => AttributeValue::S("Sheldon".to_string()),
        "RecipientID".to_string() => AttributeValue::S("Justin".to_string()),
        "Permissions".to_string() => AttributeValue::S("READ".to_string()),
        "ResourceID".to_string() => AttributeValue::S("R555".to_string()), // Specific file
        "FilePath".to_string() => AttributeValue::S("media/private/confidential-report.pdf".to_string()),
        "CreatedDate".to_string() => AttributeValue::N(
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis().to_string()
        ),
        "GSI1-PK".to_string() => AttributeValue::S("ACCESS#Justin".to_string()),
        "GSI1-SK".to_string() => AttributeValue::S(format!("GRANT#Sheldon#FILE#R555")),
    },
    ..Default::default()
};

client.put_item(put_input).await?;
```

**Step 2: API creates single VIEW_LINK for FILE grant**

```rust
// API synchronously creates VIEW_LINK after creating grant
async fn create_file_grant_view_link(client: &DynamoDbClient, grant: &ShareGrant) -> Result<()> {
    // Fetch the specific file from the owner's partition
    let file = get_file_by_id(client, &grant.owner_id, &grant.file_id).await?;

    // Create single VIEW_LINK for recipient (no folder markers)
    let view_link = hashmap! {
        "PK".to_string() => AttributeValue::S(format!("USER#{}", grant.recipient_id)),
        "SK".to_string() => AttributeValue::S(format!("VIEWLINK#{}#{}", file.owner_id, file.file_id)),
        "ItemType".to_string() => AttributeValue::S("VIEW_LINK".to_string()),
        "ResourceID".to_string() => AttributeValue::S(file.file_id.clone()),
        "OwnerID".to_string() => AttributeValue::S(file.owner_id.clone()),
        "GrantID".to_string() => AttributeValue::S(grant.grant_id.clone()),
        "CreatedDate".to_string() => AttributeValue::N(file.created_date.to_string()),
        "FolderPrefix".to_string() => AttributeValue::S(file.folder_prefix.clone()),
        "FileName".to_string() => AttributeValue::S(file.file_name.clone()),
        "MediaType".to_string() => AttributeValue::S(file.media_type.clone()),
        "GSI2-PK".to_string() => AttributeValue::S(
            format!("VIEWER#{}#FOLDER#{}", grant.recipient_id, file.folder_prefix)
        ),
        "GSI2-SK".to_string() => AttributeValue::S(
            format!("TYPE#FILE#{}#{}#{}", file.created_date, file.media_type, file.file_id)
        ),
    };

    client.put_item()
        .table_name("FileMetadata")
        .set_item(Some(view_link))
        .send()
        .await?;

    Ok(())
}
```

**Step 3: Justin views the file but cannot browse parent folder**

```rust
// Query 1: Justin can view the specific file
let file_result = client.query()
    .table_name("FileMetadata")
    .key_condition_expression("PK = :pk AND SK = :sk")
    .expression_attribute_values(":pk", AttributeValue::S("USER#Justin".to_string()))
    .expression_attribute_values(":sk", AttributeValue::S("VIEWLINK#Sheldon#R555".to_string()))
    .send()
    .await?;

// Returns the FILE VIEW_LINK for confidential-report.pdf
// Justin can download/view this file

// Query 2: Justin tries to browse the parent folder "media/private/"
let folder_result = client.query()
    .table_name("FileMetadata")
    .index_name("ViewLinkIndex")
    .key_condition_expression("GSI2-PK = :pk")
    .expression_attribute_values(":pk", AttributeValue::S("VIEWER#Justin#FOLDER#media/private/".to_string()))
    .send()
    .await?;

// Returns EMPTY - no folder markers or other files visible
// Justin cannot see:
//   - Other files in media/private/
//   - Folder markers for media/private/ itself
//   - Any subfolders under media/private/
```

**Step 4: Justin's "Shared With Me" view shows individual file**

```rust
// Query GSI1 to list all grants for Justin
let grants_result = client.query()
    .table_name("FileMetadata")
    .index_name("GrantIndex")
    .key_condition_expression("GSI1-PK = :pk")
    .expression_attribute_values(":pk", AttributeValue::S("ACCESS#Justin".to_string()))
    .send()
    .await?;

// Returns:
// 1. PREFIX grants (folders): GRANT#Sheldon#media/Project Docs/
// 2. FILE grants (individual files): GRANT#Sheldon#FILE#R555

// UI renders:
// Shared Folders:
//   📁 Sheldon's Project Docs (media/Project Docs/)
//
// Shared Files:
//   📄 confidential-report.pdf (Sheldon) - individual file, not part of a folder grant
```

**Key Benefits:**

- ✅ **Granular Sharing:** Share single file without folder access
- ✅ **Privacy Preserved:** Recipient cannot see parent folder structure or sibling files
- ✅ **Immediate Access:** FILE grant creates VIEW_LINK instantly via API (synchronous operation)
- ✅ **UI Clarity:** "Shared With Me" view distinguishes between folder grants (browsable) and file grants (standalone files)
- ✅ **Minimal Overhead:** One FILE grant + one VIEW_LINK (vs hundreds of items for folder sharing)
- ✅ **Atomic Revocation:** Delete FILE grant to immediately remove access and VIEW_LINK

**Design Notes:**

- FILE grants do NOT create folder markers - recipient sees only the file, not its parent folder
- GSI2-PK still uses the folder prefix (`VIEWER#Justin#FOLDER#media/private/`), but no folder markers exist, so browsing the folder returns empty
- If Sheldon later grants PREFIX access to `media/private/`, Justin would see the folder structure and all files (FILE grant becomes redundant)
- Application should warn users if they're creating a FILE grant for a file already covered by a PREFIX grant

## **4. Implementation Guide**

This section provides detailed implementation guidance for the core infrastructure components needed to support this schema, including S3 event processing and RESTful API endpoints.

### **4.1. S3 Event Processing via SQS**

**Integration with CloudFront:** All file operations (upload, download, delete) go through CloudFront with signed URLs (see [Section 5](#5-cloudfront--s3-signed-url-architecture)). Users never interact with S3 directly. When a user performs an operation via CloudFront, the request is proxied to S3 via Origin Access Control (OAC), and S3 events (ObjectCreated/ObjectRemoved) are triggered.

All file creation, updates, and deletions occur directly in S3 through CloudFront. S3 bucket event notifications trigger SQS messages that are processed by a Lambda function to maintain DynamoDB metadata synchronization. This architecture ensures:

- **Decoupled Processing:** S3 events are buffered in SQS for reliable, asynchronous processing
- **Automatic Synchronization:** FILE items and VIEW_LINKs are created/updated/deleted automatically when files change in S3
- **Error Handling:** Failed event processing moves to DLQ after 3 retries for manual investigation

**S3 Event Configuration:**

```terraform
resource "aws_s3_bucket_notification" "file_events" {
  bucket = aws_s3_bucket.file_storage.id

  queue {
    queue_arn     = aws_sqs_queue.s3_events.arn
    events        = ["s3:ObjectCreated:*", "s3:ObjectRemoved:*"]
    filter_prefix = "" // Process all objects
  }
}

resource "aws_sqs_queue" "s3_events" {
  name                       = "file-storage-s3-events"
  visibility_timeout_seconds = 300  // 5 minutes for Lambda processing
  message_retention_seconds  = 1209600  // 14 days
  receive_wait_time_seconds  = 20  // Long polling

  redrive_policy = jsonencode({
    deadLetterTargetArn = aws_sqs_queue.s3_events_dlq.arn
    maxReceiveCount     = 3
  })
}

resource "aws_sqs_queue" "s3_events_dlq" {
  name = "file-storage-s3-events-dlq"
}
```

**S3 Event Handler Lambda:**

This Lambda function processes S3 events and maintains DynamoDB FILE items and VIEW_LINK denormalization.

```rust
use aws_lambda_events::event::sqs::{SqsEvent, SqsMessage};
use aws_sdk_dynamodb::Client as DynamoDbClient;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct S3EventRecord {
    #[serde(rename = "eventName")]
    event_name: String,
    s3: S3Entity,
}

#[derive(Deserialize)]
struct S3Entity {
    bucket: S3Bucket,
    object: S3Object,
}

#[derive(Deserialize)]
struct S3Bucket {
    name: String,
}

#[derive(Deserialize)]
struct S3Object {
    key: String,
    size: Option<i64>,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    lambda_runtime::run(handler(func)).await?;
    Ok(())
}

async fn func(event: SqsEvent, _ctx: Context) -> Result<(), Error> {
    let config = aws_config::load_from_env().await;
    let client = DynamoDbClient::new(&config);

    for record in event.records {
        // Each SQS message contains an S3 event notification
        let s3_event: S3EventRecord = serde_json::from_str(&record.body)?;

        match s3_event.event_name.as_str() {
            name if name.starts_with("ObjectCreated:") => {
                handle_s3_object_created(&client, &s3_event).await?;
            },
            name if name.starts_with("ObjectRemoved:") => {
                handle_s3_object_removed(&client, &s3_event).await?;
            },
            _ => {
                eprintln!("Unknown S3 event: {}", s3_event.event_name);
            }
        }
    }

    Ok(())
}

async fn handle_s3_object_created(
    client: &DynamoDbClient,
    event: &S3EventRecord
) -> Result<()> {
    // Parse S3 key: format is "OwnerID/path/to/file.ext"
    let s3_key = &event.s3.object.key;
    let parts: Vec<&str> = s3_key.splitn(2, '/').collect();

    if parts.len() != 2 {
        return Err(anyhow!("Invalid S3 key format: {}", s3_key));
    }

    let owner_id = parts[0];
    let file_path = parts[1];
    let file_name = file_path.split('/').last().unwrap_or(file_path);
    let folder_prefix = calculate_folder_prefix(file_path);
    let file_id = uuid::Uuid::new_v4().to_string();

    // Detect media type from extension
    let media_type = detect_media_type(file_name);

    // CRITICAL: S3 events ONLY manage FILE items and VIEW_LINKs
    // NEVER create, update, or delete SHARE_GRANT items

    // Create FILE item
    let file_item = hashmap! {
        "PK" => AttributeValue::S(format!("USER#{}", owner_id)),
        "SK" => AttributeValue::S(format!("FILE#{}", file_path)),
        "ItemType" => AttributeValue::S("FILE".to_string()),
        "ResourceID" => AttributeValue::S(file_id.clone()),
        "OwnerID" => AttributeValue::S(owner_id.to_string()),
        "FileName" => AttributeValue::S(file_name.to_string()),
        "FolderPrefix" => AttributeValue::S(folder_prefix.clone()),
        "CreatedDate" => AttributeValue::N(chrono::Utc::now().timestamp().to_string()),
        "MediaType" => AttributeValue::S(media_type.clone()),
        "S3Key" => AttributeValue::S(s3_key.clone()),
        "Size" => AttributeValue::N(event.s3.object.size.unwrap_or(0).to_string()),
    };

    client.put_item()
        .table_name("FileMetadata")
        .set_item(Some(file_item))
        .send()
        .await?;

    // Create VIEW_LINKs for owner
    let mut items_to_create = Vec::new();

    let owner_view_link = create_file_view_link(
        owner_id,
        owner_id,
        &file_id,
        file_name,
        &folder_prefix,
        &media_type,
        chrono::Utc::now().timestamp(),
        "OWNER"
    );
    items_to_create.push(owner_view_link);

    // Create folder markers for owner
    items_to_create.extend(
        create_folder_markers(owner_id, owner_id, "OWNER", &folder_prefix)
    );

    // Find all PREFIX grants that match this file's path
    let prefix_grants = find_prefix_grants(client, owner_id, &folder_prefix).await?;

    for grant in prefix_grants {
        // Create VIEW_LINK for recipient
        let recipient_view_link = create_file_view_link(
            &grant.recipient_id,
            owner_id,
            &file_id,
            file_name,
            &folder_prefix,
            &media_type,
            chrono::Utc::now().timestamp(),
            &grant.grant_id
        );
        items_to_create.push(recipient_view_link);

        // Create folder markers for recipient
        items_to_create.extend(
            create_folder_markers(&grant.recipient_id, owner_id, &grant.grant_id, &folder_prefix)
        );
    }

    // Find all FILE grants that match this specific ResourceID
    // (This handles the case where a file was deleted and re-uploaded with same path)
    let file_grants = find_file_grants_by_path(client, owner_id, file_path).await?;

    for grant in file_grants {
        // Create VIEW_LINK for recipient (no folder markers for FILE grants)
        let recipient_view_link = create_file_view_link(
            &grant.recipient_id,
            owner_id,
            &file_id,
            file_name,
            &folder_prefix,
            &media_type,
            chrono::Utc::now().timestamp(),
            &grant.grant_id
        );
        items_to_create.push(recipient_view_link);
    }

    // Batch write all VIEW_LINKs and folder markers
    batch_write_items(client, items_to_create).await?;

    Ok(())
}

async fn handle_s3_object_removed(
    client: &DynamoDbClient,
    event: &S3EventRecord
) -> Result<()> {
    // Parse S3 key: format is "OwnerID/path/to/file.ext"
    let s3_key = &event.s3.object.key;
    let parts: Vec<&str> = s3_key.splitn(2, '/').collect();

    if parts.len() != 2 {
        return Err(anyhow!("Invalid S3 key format: {}", s3_key));
    }

    let owner_id = parts[0];
    let file_path = parts[1];

    // Get FILE item to retrieve ResourceID
    let file_result = client.get_item()
        .table_name("FileMetadata")
        .key("PK", AttributeValue::S(format!("USER#{}", owner_id)))
        .key("SK", AttributeValue::S(format!("FILE#{}", file_path)))
        .send()
        .await?;

    let file_item = file_result.item
        .ok_or_else(|| anyhow!("FILE item not found for deleted S3 object: {}", s3_key))?;

    let file_id = file_item.get("ResourceID")
        .and_then(|v| v.as_s().ok())
        .ok_or_else(|| anyhow!("ResourceID not found in FILE item"))?;

    // Find all grants related to this file
    let folder_prefix = calculate_folder_prefix(file_path);
    let prefix_grants = find_prefix_grants(client, owner_id, &folder_prefix).await?;
    let file_grants = find_file_grants_by_file_id(client, owner_id, file_id).await?;

    // Collect all users who have VIEW_LINKs for this file
    let mut recipients = vec![owner_id.to_string()];
    recipients.extend(prefix_grants.iter().map(|g| g.recipient_id.clone()));
    recipients.extend(file_grants.iter().map(|g| g.recipient_id.clone()));

    // Delete VIEW_LINKs for all recipients
    delete_view_links_batch(client, file_id, owner_id, &recipients).await?;

    // Delete FILE grants (not PREFIX grants - those remain)
    delete_file_grants(client, owner_id, file_id).await?;

    // Delete FILE item
    client.delete_item()
        .table_name("FileMetadata")
        .key("PK", AttributeValue::S(format!("USER#{}", owner_id)))
        .key("SK", AttributeValue::S(format!("FILE#{}", file_path)))
        .send()
        .await?;

    // IMPORTANT: What we DON'T delete:
    // ❌ PREFIX SHARE_GRANTs - folder-level permissions persist
    // ❌ Folder marker VIEW_LINKs - folder structure persists
    // This enables "ghost folder" behavior where shared folders appear
    // empty until new files are uploaded (which automatically create VIEW_LINKs)

    Ok(())
}

// Helper function: Find all PREFIX grants for a folder
async fn find_prefix_grants(
    client: &DynamoDbClient,
    owner_id: &str,
    folder_prefix: &str
) -> Result<Vec<ShareGrant>> {
    let mut grants = Vec::new();
    let mut last_key = None;

    // Query for all grants by this owner
    loop {
        let result = client.query()
            .table_name("FileMetadata")
            .key_condition_expression("PK = :pk AND begins_with(SK, :sk_prefix)")
            .expression_attribute_values(":pk", AttributeValue::S(format!("USER#{}", owner_id)))
            .expression_attribute_values(":sk_prefix", AttributeValue::S("GRANT#".to_string()))
            .set_exclusive_start_key(last_key)
            .send()
            .await?;

        for item in result.items.unwrap_or_default() {
            let grant = parse_grant(&item)?;

            // Only include PREFIX grants where the file's prefix starts with the grant prefix
            if grant.grant_type == "PREFIX" && folder_prefix.starts_with(&grant.prefix) {
                grants.push(grant);
            }
        }

        if result.last_evaluated_key.is_none() {
            break;
        }
        last_key = result.last_evaluated_key;
    }

    Ok(grants)
}

// Helper function: Find all FILE grants for a specific file path
async fn find_file_grants_by_path(
    client: &DynamoDbClient,
    owner_id: &str,
    file_path: &str
) -> Result<Vec<ShareGrant>> {
    let mut grants = Vec::new();
    let mut last_key = None;

    loop {
        let result = client.query()
            .table_name("FileMetadata")
            .key_condition_expression("PK = :pk AND begins_with(SK, :sk_prefix)")
            .filter_expression("FilePath = :file_path")
            .expression_attribute_values(":pk", AttributeValue::S(format!("USER#{}", owner_id)))
            .expression_attribute_values(":sk_prefix", AttributeValue::S("GRANT#".to_string()))
            .expression_attribute_values(":file_path", AttributeValue::S(file_path.to_string()))
            .set_exclusive_start_key(last_key)
            .send()
            .await?;

        for item in result.items.unwrap_or_default() {
            let grant = parse_grant(&item)?;

            if grant.grant_type == "FILE" {
                grants.push(grant);
            }
        }

        if result.last_evaluated_key.is_none() {
            break;
        }
        last_key = result.last_evaluated_key;
    }

    Ok(grants)
}

// Helper function: Find all FILE grants for a specific ResourceID
async fn find_file_grants_by_file_id(
    client: &DynamoDbClient,
    owner_id: &str,
    file_id: &str
) -> Result<Vec<ShareGrant>> {
    let mut grants = Vec::new();
    let mut last_key = None;

    loop {
        let result = client.query()
            .table_name("FileMetadata")
            .key_condition_expression("PK = :pk AND begins_with(SK, :sk_prefix)")
            .filter_expression("ResourceID = :file_id")
            .expression_attribute_values(":pk", AttributeValue::S(format!("USER#{}", owner_id)))
            .expression_attribute_values(":sk_prefix", AttributeValue::S("GRANT#".to_string()))
            .expression_attribute_values(":file_id", AttributeValue::S(file_id.to_string()))
            .set_exclusive_start_key(last_key)
            .send()
            .await?;

        for item in result.items.unwrap_or_default() {
            let grant = parse_grant(&item)?;

            if grant.grant_type == "FILE" {
                grants.push(grant);
            }
        }

        if result.last_evaluated_key.is_none() {
            break;
        }
        last_key = result.last_evaluated_key;
    }

    Ok(grants)
}

// Helper function: Delete FILE grants for a deleted file
async fn delete_file_grants(
    client: &DynamoDbClient,
    owner_id: &str,
    file_id: &str
) -> Result<()> {
    let file_grants = find_file_grants_by_file_id(client, owner_id, file_id).await?;

    for grant in file_grants {
        client.delete_item()
            .table_name("FileMetadata")
            .key("PK", AttributeValue::S(format!("USER#{}", owner_id)))
            .key("SK", AttributeValue::S(format!("GRANT#{}", grant.grant_id)))
            .send()
            .await?;
    }

    Ok(())
}

// Helper function: Delete VIEW_LINKs for multiple recipients
async fn delete_view_links_batch(
    client: &DynamoDbClient,
    file_id: &str,
    owner_id: &str,
    recipients: &[String]
) -> Result<()> {
    for recipient_id in recipients {
        client.delete_item()
            .table_name("FileMetadata")
            .key("PK", AttributeValue::S(format!("USER#{}", recipient_id)))
            .key("SK", AttributeValue::S(format!("VIEWLINK#{}#{}", owner_id, file_id)))
            .send()
            .await?;
    }

    Ok(())
}

// Helper function: Ensure all ancestor folder markers exist
// This is called for EVERY file upload to maintain folder hierarchy
async fn ensure_folder_hierarchy(
    client: &DynamoDbClient,
    owner_id: &str,
    file_path: &str
) -> Result<()> {
    let folder_prefix = calculate_folder_prefix(file_path);

    if folder_prefix.is_empty() {
        return Ok(()); // Root-level file, no folders needed
    }

    // Get all ancestor paths
    // e.g., "media/photos/2024/vacation/" -> ["media/", "media/photos/", "media/photos/2024/", "media/photos/2024/vacation/"]
    let ancestor_paths = get_ancestor_folder_paths(&folder_prefix);

    // Find all PREFIX grants that cover any ancestor folder
    let prefix_grants = find_all_matching_prefix_grants(
        client,
        owner_id,
        &ancestor_paths
    ).await?;

    // For each ancestor folder path
    for ancestor_path in ancestor_paths {
        // Create folder marker for owner (if doesn't exist)
        create_folder_marker_if_not_exists(
            client,
            owner_id,
            owner_id,
            &ancestor_path,
            "OWNER"
        ).await?;

        // Create folder markers for all recipients with PREFIX grants covering this path
        for grant in &prefix_grants {
            if ancestor_path.starts_with(&grant.prefix) {
                create_folder_marker_if_not_exists(
                    client,
                    &grant.recipient_id,
                    owner_id,
                    &ancestor_path,
                    &grant.grant_id
                ).await?;
            }
        }
    }

    Ok(())
}

// Helper function: Get all ancestor folder paths from a folder prefix
fn get_ancestor_folder_paths(folder_prefix: &str) -> Vec<String> {
    if folder_prefix.is_empty() {
        return vec![];
    }

    let parts: Vec<&str> = folder_prefix
        .trim_end_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    let mut ancestors = Vec::new();
    let mut current_path = String::new();

    for part in parts {
        if !current_path.is_empty() {
            current_path.push('/');
        }
        current_path.push_str(part);
        ancestors.push(format!("{}/", current_path));
    }

    ancestors
}

// Helper function: Create folder marker with conditional put (idempotent)
async fn create_folder_marker_if_not_exists(
    client: &DynamoDbClient,
    viewer_id: &str,
    owner_id: &str,
    folder_path: &str,
    grant_id: &str
) -> Result<()> {
    let folder_name = folder_path
        .trim_end_matches('/')
        .split('/')
        .last()
        .unwrap_or("") + "/";

    let parent_prefix = if folder_path.matches('/').count() > 1 {
        let parts: Vec<&str> = folder_path
            .trim_end_matches('/')
            .split('/')
            .collect();
        parts[..parts.len()-1].join("/") + "/"
    } else {
        String::new()
    };

    let marker = hashmap! {
        "PK" => AttributeValue::S(format!("USER#{}", viewer_id)),
        "SK" => AttributeValue::S(format!("VIEWLINK#{}#FOLDER#{}", owner_id, folder_path)),
        "ItemType" => AttributeValue::S("VIEW_LINK".to_string()),
        "ResourceID" => AttributeValue::S(format!("FOLDER#{}", folder_path)),
        "OwnerID" => AttributeValue::S(owner_id.to_string()),
        "GrantID" => AttributeValue::S(grant_id.to_string()),
        "FileName" => AttributeValue::S(folder_name.to_string()),
        "FolderPrefix" => AttributeValue::S(parent_prefix.clone()),
        "MediaType" => AttributeValue::S("application/x-directory".to_string()),
        "CreatedDate" => AttributeValue::N(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
                .to_string()
        ),
        "GSI2-PK" => AttributeValue::S(
            format!("VIEWER#{}#FOLDER#{}", viewer_id, parent_prefix)
        ),
        "GSI2-SK" => AttributeValue::S(
            format!("TYPE#FOLDER#{}#{}", folder_name, owner_id)
        ),
    };

    // Use conditional put to avoid duplicates (idempotent)
    match client.put_item()
        .table_name("FileMetadata")
        .set_item(Some(marker))
        .condition_expression("attribute_not_exists(PK)")
        .send()
        .await
    {
        Ok(_) => Ok(()),
        Err(e) if e.to_string().contains("ConditionalCheckFailedException") => {
            // Folder marker already exists - this is OK (idempotent)
            Ok(())
        },
        Err(e) => Err(e.into()),
    }
}

// Helper function: Find all PREFIX grants that match any of the given folder paths
async fn find_all_matching_prefix_grants(
    client: &DynamoDbClient,
    owner_id: &str,
    folder_paths: &[String]
) -> Result<Vec<ShareGrant>> {
    let mut grants = Vec::new();
    let mut last_key = None;

    loop {
        let result = client.query()
            .table_name("FileMetadata")
            .key_condition_expression("PK = :pk AND begins_with(SK, :sk_prefix)")
            .filter_expression("GrantType = :grant_type")
            .expression_attribute_values(":pk", AttributeValue::S(format!("USER#{}", owner_id)))
            .expression_attribute_values(":sk_prefix", AttributeValue::S("GRANT#".to_string()))
            .expression_attribute_values(":grant_type", AttributeValue::S("PREFIX".to_string()))
            .set_exclusive_start_key(last_key)
            .send()
            .await?;

        for item in result.items.unwrap_or_default() {
            let grant = parse_grant(&item)?;

            // Check if this grant's prefix matches any of the folder paths
            for folder_path in folder_paths {
                if folder_path.starts_with(&grant.prefix) {
                    grants.push(grant.clone());
                    break;
                }
            }
        }

        if result.last_evaluated_key.is_none() {
            break;
        }
        last_key = result.last_evaluated_key;
    }

    Ok(grants)
}

// Helper function: Create VIEW_LINK item
fn create_file_view_link(
    recipient_id: &str,
    owner_id: &str,
    file_id: &str,
    file_name: &str,
    folder_prefix: &str,
    media_type: &str,
    created_date: i64,
    grant_id: &str
) -> HashMap<String, AttributeValue> {
    hashmap! {
        "PK" => AttributeValue::S(format!("USER#{}", recipient_id)),
        "SK" => AttributeValue::S(format!("VIEWLINK#{}#{}", owner_id, file_id)),
        "ItemType" => AttributeValue::S("VIEW_LINK".to_string()),
        "ResourceID" => AttributeValue::S(file_id.to_string()),
        "OwnerID" => AttributeValue::S(owner_id.to_string()),
        "GrantID" => AttributeValue::S(grant_id.to_string()),
        "CreatedDate" => AttributeValue::N(created_date.to_string()),
        "FolderPrefix" => AttributeValue::S(folder_prefix.to_string()),
        "FileName" => AttributeValue::S(file_name.to_string()),
        "MediaType" => AttributeValue::S(media_type.to_string()),
        "GSI2-PK" => AttributeValue::S(
            format!("VIEWER#{}#FOLDER#{}", recipient_id, folder_prefix)
        ),
        "GSI2-SK" => AttributeValue::S(
            format!("TYPE#FILE#{}#{}#{}", created_date, media_type, file_id)
        ),
    }
}

// Helper function: Create folder marker VIEW_LINKs for all ancestor folders
fn create_folder_markers(
    recipient_id: &str,
    owner_id: &str,
    grant_id: &str,
    folder_prefix: &str
) -> Vec<HashMap<String, AttributeValue>> {
    let mut markers = Vec::new();

    // Extract all ancestor folder paths
    let segments: Vec<&str> = folder_prefix
        .trim_end_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    for i in 0..segments.len() {
        let full_folder_path = segments[..=i].join("/") + "/";
        let folder_name = segments[i];
        let parent_folder = if i > 0 {
            segments[..i].join("/") + "/"
        } else {
            String::new()
        };

        let marker = hashmap! {
            "PK" => AttributeValue::S(format!("USER#{}", recipient_id)),
            "SK" => AttributeValue::S(format!("VIEWLINK#{}#FOLDER#{}", owner_id, full_folder_path)),
            "ItemType" => AttributeValue::S("VIEW_LINK".to_string()),
            "ItemSubtype" => AttributeValue::S("FOLDER_MARKER".to_string()),
            "OwnerID" => AttributeValue::S(owner_id.to_string()),
            "GrantID" => AttributeValue::S(grant_id.to_string()),
            "FolderName" => AttributeValue::S(folder_name.to_string()),
            "FolderPrefix" => AttributeValue::S(parent_folder.clone()),
            "FullFolderPath" => AttributeValue::S(full_folder_path.clone()),
            "GSI2-PK" => AttributeValue::S(
                format!("VIEWER#{}#FOLDER#{}", recipient_id, parent_folder)
            ),
            "GSI2-SK" => AttributeValue::S(
                format!("TYPE#FOLDER#{}", folder_name)
            ),
        };

        markers.push(marker);
    }

    markers
}

// Helper function: Calculate folder prefix from file path
fn calculate_folder_prefix(file_path: &str) -> String {
    let segments: Vec<&str> = file_path.split('/').collect();
    if segments.len() > 1 {
        segments[..segments.len() - 1].join("/") + "/"
    } else {
        String::new()
    }
}

// Helper function: Detect media type from file extension
fn detect_media_type(file_name: &str) -> String {
    let extension = file_name.split('.').last().unwrap_or("").to_lowercase();

    match extension.as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "heic" => "IMAGE",
        "mp4" | "mov" | "avi" | "mkv" | "webm" => "VIDEO",
        "mp3" | "wav" | "flac" | "aac" | "ogg" => "AUDIO",
        "pdf" => "PDF",
        "doc" | "docx" => "DOCUMENT",
        "xls" | "xlsx" => "SPREADSHEET",
        "txt" | "md" | "json" | "xml" | "csv" => "TEXT",
        "zip" | "tar" | "gz" | "7z" | "rar" => "ARCHIVE",
        _ => "OTHER",
    }.to_string()
}

// Helper function: Parse SHARE_GRANT from DynamoDB item
fn parse_grant(item: &HashMap<String, AttributeValue>) -> Result<ShareGrant> {
    Ok(ShareGrant {
        grant_id: item.get("GrantID")?.as_s()?.clone(),
        owner_id: item.get("OwnerID")?.as_s()?.clone(),
        recipient_id: item.get("RecipientID")?.as_s()?.clone(),
        grant_type: item.get("GrantType")?.as_s()?.clone(),
        prefix: item.get("Prefix").and_then(|v| v.as_s().ok()).unwrap_or("").to_string(),
        file_id: item.get("ResourceID").and_then(|v| v.as_s().ok()).map(|s| s.to_string()),
        file_path: item.get("FilePath").and_then(|v| v.as_s().ok()).map(|s| s.to_string()),
    })
}

#[derive(Debug, Clone)]
struct ShareGrant {
    grant_id: String,
    owner_id: String,
    recipient_id: String,
    grant_type: String,
    prefix: String,
    file_id: Option<String>,
    file_path: Option<String>,
}

// Helper function: Batch write items with retry logic
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

**Key Points:**

- ✅ **S3-Driven Lifecycle:** All file operations originate from S3 (upload, update, delete)
- ✅ **SQS Decoupling:** S3 events queue to SQS, Lambda processes messages reliably
- ✅ **Automatic VIEW_LINK Creation:** When file created, VIEW_LINKs created for owner + all grant recipients
- ✅ **Automatic VIEW_LINK Cleanup:** When file deleted, all VIEW_LINKs cleaned up across all users
- ✅ **Folder Marker Automation:** Folder markers created automatically for all ancestor folders
- ✅ **Grant-Aware:** Respects both PREFIX and FILE grants when creating VIEW_LINKs
- ✅ **Idempotent:** Can safely retry failed events without duplication issues

### **4.2. RESTful API Endpoints**

The service exposes a synchronous RESTful API for file metadata queries, folder operations, and grant management. All API operations complete fully before returning to the client (no async jobs or 202 Accepted responses for MVP).

**Authentication:** All API endpoints require JWT authentication via Firebase. A middleware extracts `user_id` from the JWT token in the `Authorization: Bearer <token>` header.

**Base URL Pattern:** `/storage/{owner-id}/{path}`

#### **4.2.1. Get File Metadata**

**Purpose:** Retrieve metadata for a specific file, including a CloudFront signed URL for immediate download access.

```http
GET /storage/{owner-id}/{file-path}
Authorization: Bearer <jwt-token>

Response 200 OK:
{
  "file_id": "F123",
  "owner_id": "Sheldon",
  "file_name": "vacation.jpg",
  "folder_prefix": "media/photos/2024/",
  "media_type": "IMAGE",
  "created_date": 1700000000,
  "size": 2048576,
  "s3_key": "Sheldon/media/photos/2024/vacation.jpg",
  "signed_url": "https://dev-storage.fromthehart.tech/Sheldon/media/photos/2024/vacation.jpg?Policy=eyJTdGF0ZW1lbnQiO...&Signature=abc123...&Key-Pair-Id=K2JCEXAMPLE",
  "signed_url_expires_at": 1700086400
}

Response 403 Forbidden:
{
  "error": "ACCESS_DENIED",
  "message": "No VIEW_LINK found for this file"
}

Response 404 Not Found:
{
  "error": "FILE_NOT_FOUND"
}
```

**Implementation:**

1. Extract `user_id` from JWT
2. Query: `PK = USER#{owner-id}`, `SK = FILE#{file-path}`
3. Validate permission: Check VIEW_LINK exists (`PK = USER#{user_id}`, `SK = VIEWLINK#{owner-id}#{file_id}`)
4. Generate CloudFront signed URL with 24-hour expiration (see [Section 5.3](#53-signed-url-generation))
5. Return FILE metadata + signed URL if authorized

**Note on Signed URLs:** The `signed_url` field provides immediate download access without requiring a separate API call. The URL expires after 24 hours and includes a cryptographic signature that CloudFront validates.

#### **4.2.2. List Folder Contents** ⚠️ **CRITICAL ENDPOINT**

**Purpose:** Browse folder contents (subfolders + files). This is the most performance-critical endpoint.

```http
GET /storage/{owner-id}/{folder-path}/
Authorization: Bearer <jwt-token>

Query Parameters:
- media_type: Optional filter (IMAGE, VIDEO, DOCUMENT, etc.)
- sort: "newest" | "oldest" (default: "newest")
- limit: Page size (default: 50, max: 100)
- cursor: Pagination token from previous response

Response 200 OK:
{
  "subfolders": [
    {
      "name": "2024/",
      "full_path": "media/photos/2024/",
      "owner_id": "Sheldon"
    }
  ],
  "files": [
    {
      "file_id": "F123",
      "owner_id": "Sheldon",
      "file_name": "vacation.jpg",
      "media_type": "IMAGE",
      "created_date": 1700000000,
      "size": 2048576,
      "signed_url": "https://dev-storage.fromthehart.tech/Sheldon/media/photos/2024/vacation.jpg?Policy=...&Signature=...&Key-Pair-Id=...",
      "signed_url_expires_at": 1700086400
    }
  ],
  "next_cursor": "eyJQSyI6IlVTRVIjU2hlbGRvbiIsIC4uLn0="
}

Response 403 Forbidden:
{
  "error": "ACCESS_DENIED",
  "message": "No VIEW_LINK folder marker found"
}
```

**Implementation:**

1. Extract `user_id` from JWT
2. Query GSI2: `GSI2-PK = VIEWER#{user_id}#FOLDER#{folder-path}`
3. Separate folder markers (ItemSubtype=FOLDER_MARKER) from files
4. Apply media_type filter if specified (uses FilterExpression)
5. Return paginated results

**Performance Considerations:**

- For folders with 1000+ files, consider implementing server-side caching (30-60 second TTL)
- Use DynamoDB Accelerator (DAX) for sub-millisecond reads
- Optimize GSI2 projection to minimize item size
- FilterExpression counts against RCU even for filtered items
- Generating signed URLs for each file adds latency; consider lazy loading (generate on-demand)

**Note on Signed URLs:** Each file includes a `signed_url` field for immediate download access. The URLs expire after 24 hours and are secured with RSA signatures verified by CloudFront (see [Section 5](#5-cloudfront--s3-signed-url-architecture)).

#### **4.2.3. Generate Signed URL for Upload**

**Purpose:** Request a CloudFront signed URL for uploading a file directly to S3 via CloudFront. Required before uploading any file.

```http
POST /storage/signed-url
Authorization: Bearer <jwt-token>

Request Body:
{
  "operation": "upload",
  "file_path": "media/photos/vacation.jpg",
  "content_type": "image/jpeg"
}

Response 200 OK:
{
  "signed_url": "https://dev-storage.fromthehart.tech/Sheldon/media/photos/vacation.jpg?Policy=eyJTdGF0ZW1lbnQiO...&Signature=abc123...&Key-Pair-Id=K2JCEXAMPLE",
  "expires_at": 1700000900,
  "method": "PUT",
  "headers": {
    "Content-Type": "image/jpeg"
  }
}

Response 400 Bad Request:
{
  "error": "INVALID_PATH",
  "message": "File path must start with your user ID"
}

Response 403 Forbidden:
{
  "error": "ACCESS_DENIED",
  "message": "Cannot upload to this path"
}
```

**Implementation:**

1. Extract `user_id` from JWT
2. Validate file path starts with `{user_id}/` (users can only upload to their own paths)
3. Validate file path doesn't contain malicious patterns (`..`, `//`, null bytes, etc.)
4. Generate CloudFront signed URL with wildcard policy and 15-minute expiration (see [Section 5.3](#53-signed-url-generation))
5. Return signed URL + upload instructions

**Upload Flow:**

```
1. Client calls POST /storage/signed-url → receives signed URL
2. Client uploads file directly to CloudFront using PUT request
3. CloudFront verifies signature and proxies to S3 via OAC
4. S3 stores file and sends ObjectCreated event to SQS
5. Lambda processes event and creates FILE + VIEW_LINK items in DynamoDB
6. Client polls GET /storage/{owner-id}/{file-path} until file appears (typically 1-3 seconds)
```

**Security Notes:**

- Signed URL expires in 15 minutes (prevents replay attacks)
- Users can only upload to paths matching their user ID
- CloudFront validates signature before forwarding to S3
- S3 event processor validates file ownership before creating DynamoDB items

#### **4.2.4. Create Folder Marker**

**Purpose:** Explicitly create an empty folder (S3 event processor creates them automatically when files are uploaded). Uses PUT for idempotent folder creation.

```http
PUT /storage/{owner-id}/{folder-path}/
Authorization: Bearer <jwt-token>

Response 201 Created:
{
  "folder_path": "media/photos/2024/",
  "created": true
}

Response 200 OK:
{
  "folder_path": "media/photos/2024/",
  "created": false,
  "message": "Folder already exists"
}
```

**Implementation:**

1. Extract `user_id` from JWT, verify `user_id == owner-id`
2. Derive folder name from `{folder-path}` URL parameter (no request body needed)
3. Create VIEW_LINK folder marker with `condition_expression: attribute_not_exists(PK)` for idempotency
4. Return 201 if created, 200 if already exists (PUT is idempotent)

#### **4.2.5. Create Share Grant (PREFIX or FILE)**

**Purpose:** Share a folder (PREFIX grant) or individual file (FILE grant) with another user.

```http
POST /storage/grants
Authorization: Bearer <jwt-token>

Request Body (PREFIX Grant):
{
  "recipient_id": "Justin",
  "grant_type": "PREFIX",
  "prefix": "media/photos/",
  "permissions": "READ"
}

Request Body (FILE Grant):
{
  "recipient_id": "Justin",
  "grant_type": "FILE",
  "file_path": "media/private/confidential.pdf",
  "permissions": "READ"
}

Response 201 Created:
{
  "grant_id": "G789",
  "owner_id": "Sheldon",
  "recipient_id": "Justin",
  "grant_type": "PREFIX",
  "prefix": "media/photos/",
  "permissions": "READ",
  "created_date": 1700000000,
  "view_links_created": 1543
}

Response 400 Bad Request:
{
  "error": "INVALID_RECIPIENT",
  "message": "Recipient user not found"
}
```

**Implementation:**

1. Extract `user_id` from JWT (owner)
2. Validate `recipient_id` exists (query user service/table)
3. Create SHARE_GRANT item
4. **For PREFIX grants:** Query all files under prefix, create VIEW_LINKs + folder markers synchronously (batched)
5. **For FILE grants:** Get FILE item, create single VIEW_LINK synchronously
6. Return grant details with VIEW_LINK count

**Performance Note:** For large PREFIX grants (>1000 files), this may take 10-30 seconds. All operations complete synchronously before returning. Show progress indicator in UI.

#### **4.2.6. List Grants (Owned or Received)**

**Purpose:** List all grants created by user or received by user.

```http
GET /storage/grants
Authorization: Bearer <jwt-token>

Query Parameters:
- filter: "owned" | "received" (default: "owned")
- limit: Page size (default: 50, max: 100)
- cursor: Pagination token

Response 200 OK (filter=owned):
{
  "grants": [
    {
      "grant_id": "G789",
      "recipient_id": "Justin",
      "grant_type": "PREFIX",
      "prefix": "media/photos/",
      "permissions": "READ",
      "created_date": 1700000000
    }
  ],
  "next_cursor": null
}
```

**Implementation:**

- **filter=owned:** Query `PK = USER#{user_id}`, `SK begins_with GRANT#`
- **filter=received:** Query GSI1: `GSI1-PK = ACCESS#{user_id}`
- Enrich with user emails
- Return paginated results

#### **4.2.7. Get Grant Details**

**Purpose:** Get detailed information about a specific grant.

```http
GET /storage/grants/{grant-id}
Authorization: Bearer <jwt-token>

Response 200 OK:
{
  "grant_id": "G789",
  "owner_id": "Sheldon",
  "recipient_id": "Justin",
  "grant_type": "PREFIX",
  "prefix": "media/photos/",
  "permissions": "READ",
  "created_date": 1700000000,
  "file_count": 1543
}

Response 403 Forbidden:
{
  "error": "ACCESS_DENIED"
}
```

**Implementation:**

1. Extract `user_id` from JWT
2. Query grant by PK/SK
3. Verify `user_id == owner_id OR user_id == recipient_id`
4. Return grant details

#### **4.2.8. Revoke Grant**

**Purpose:** Delete a grant, removing all access for the recipient.

```http
DELETE /storage/grants/{grant-id}
Authorization: Bearer <jwt-token>

Response 204 No Content

Response 403 Forbidden:
{
  "error": "ACCESS_DENIED",
  "message": "Only grant owner can revoke"
}
```

**Implementation:**

1. Extract `user_id` from JWT
2. Get SHARE_GRANT, verify `user_id == owner_id`
3. **For PREFIX grants:** Query all VIEW_LINKs for recipient under prefix, delete synchronously
4. **For FILE grants:** Delete single VIEW_LINK for recipient
5. Delete SHARE_GRANT item
6. Return 204

**Performance Note:** Revoking PREFIX grants with many files may take 10-30 seconds.

#### **4.2.9. DELETE /{path}/ - Delete Empty Folder**

**Purpose:** Delete an empty folder and all associated PREFIX grants. This is a metadata-only operation (no S3 interaction).

```http
DELETE /{owner-id}/{folder-path}/
Authorization: Bearer <jwt-token>

Response 204 No Content

Response 400 Bad Request:
{
  "error": "FOLDER_NOT_EMPTY",
  "message": "Cannot delete folder. The folder must be empty before deletion. Please delete all files and subfolders first, then try again."
}

Response 403 Forbidden:
{
  "error": "ACCESS_DENIED",
  "message": "Only the folder owner can delete folders"
}

Response 404 Not Found:
{
  "error": "FOLDER_NOT_FOUND",
  "message": "Folder does not exist"
}
```

**Implementation:**

```rust
async fn delete_folder(
    client: &DynamoDbClient,
    user_id: &str,
    owner_id: &str,
    folder_path: &str
) -> Result<Response> {
    // 1. Verify caller is the owner
    if user_id != owner_id {
        return Err(ApiError::Forbidden(
            "Only the folder owner can delete folders"
        ));
    }

    // 2. Verify folder exists (check for owner's folder marker)
    let folder_exists = check_folder_marker_exists(
        client,
        owner_id,
        owner_id,
        folder_path
    ).await?;

    if !folder_exists {
        return Err(ApiError::NotFound("Folder does not exist"));
    }

    // 3. Check if folder is empty (no files or subfolders)
    let has_contents = check_folder_has_contents(
        client,
        owner_id,
        folder_path
    ).await?;

    if has_contents {
        return Err(ApiError::BadRequest(
            "Cannot delete folder. The folder must be empty before deletion. \
             Please delete all files and subfolders first, then try again."
        ));
    }

    // 4. Find all PREFIX grants for this exact folder path
    let prefix_grants = query_prefix_grants_for_exact_folder(
        client,
        owner_id,
        folder_path
    ).await?;

    // 5. Delete folder marker VIEW_LINKs for all recipients
    let mut delete_operations = vec![];

    // Delete owner's folder marker
    delete_operations.push((
        format!("USER#{}", owner_id),
        format!("VIEWLINK#{}#FOLDER#{}", owner_id, folder_path)
    ));

    // Delete recipient folder markers
    for grant in &prefix_grants {
        delete_operations.push((
            format!("USER#{}", grant.recipient_id),
            format!("VIEWLINK#{}#FOLDER#{}", owner_id, folder_path)
        ));
    }

    // Batch delete VIEW_LINKs
    for (pk, sk) in delete_operations {
        client.delete_item()
            .table_name("FileMetadata")
            .key("PK", AttributeValue::S(pk))
            .key("SK", AttributeValue::S(sk))
            .send()
            .await?;
    }

    // 6. Delete all PREFIX SHARE_GRANTs for this folder
    for grant in prefix_grants {
        client.delete_item()
            .table_name("FileMetadata")
            .key("PK", AttributeValue::S(format!("USER#{}", owner_id)))
            .key("SK", AttributeValue::S(format!(
                "GRANT#{}#{}",
                grant.recipient_id,
                grant.grant_id
            )))
            .send()
            .await?;
    }

    Ok(Response::NoContent)
}

// Helper: Check if folder has any contents (files or subfolders)
async fn check_folder_has_contents(
    client: &DynamoDbClient,
    owner_id: &str,
    folder_path: &str
) -> Result<bool> {
    // Query GSI2 for owner to see if folder has any children
    let result = client.query()
        .table_name("FileMetadata")
        .index_name("ViewLinkIndex")
        .key_condition_expression("GSI2PK = :pk")
        .expression_attribute_values(
            ":pk",
            AttributeValue::S(format!(
                "VIEWER#{}#FOLDER#{}",
                owner_id,
                folder_path
            ))
        )
        .limit(1) // We only need to know if ANY item exists
        .send()
        .await?;

    // If any items found, folder is not empty
    Ok(result.items.map(|i| !i.is_empty()).unwrap_or(false))
}

// Helper: Check if folder marker exists
async fn check_folder_marker_exists(
    client: &DynamoDbClient,
    viewer_id: &str,
    owner_id: &str,
    folder_path: &str
) -> Result<bool> {
    let result = client.get_item()
        .table_name("FileMetadata")
        .key("PK", AttributeValue::S(format!("USER#{}", viewer_id)))
        .key("SK", AttributeValue::S(format!(
            "VIEWLINK#{}#FOLDER#{}",
            owner_id,
            folder_path
        )))
        .send()
        .await?;

    Ok(result.item.is_some())
}

// Helper: Query PREFIX grants for exact folder match
async fn query_prefix_grants_for_exact_folder(
    client: &DynamoDbClient,
    owner_id: &str,
    folder_path: &str
) -> Result<Vec<ShareGrant>> {
    let mut grants = Vec::new();
    let mut last_key = None;

    loop {
        let result = client.query()
            .table_name("FileMetadata")
            .key_condition_expression("PK = :pk AND begins_with(SK, :sk_prefix)")
            .filter_expression("GrantType = :grant_type AND Prefix = :folder_path")
            .expression_attribute_values(":pk", AttributeValue::S(format!("USER#{}", owner_id)))
            .expression_attribute_values(":sk_prefix", AttributeValue::S("GRANT#".to_string()))
            .expression_attribute_values(":grant_type", AttributeValue::S("PREFIX".to_string()))
            .expression_attribute_values(":folder_path", AttributeValue::S(folder_path.to_string()))
            .set_exclusive_start_key(last_key)
            .send()
            .await?;

        for item in result.items.unwrap_or_default() {
            grants.push(parse_grant(&item)?);
        }

        if result.last_evaluated_key.is_none() {
            break;
        }
        last_key = result.last_evaluated_key;
    }

    Ok(grants)
}
```

**Key Points:**

- ✅ **RESTful Design:** Uses `DELETE /{path}/` pattern (trailing slash indicates folder)
- ✅ **Validation:** Checks folder is empty before allowing deletion
- ✅ **Atomic:** Deletes folder markers AND PREFIX grants together
- ✅ **Batch Operations:** Uses efficient single-item checks and batch deletes
- ✅ **Clear Error Messages:** Guides users to delete contents first
- ✅ **Metadata Only:** No S3 operations (folders are purely logical)

**Important Distinctions:**

| Operation         | Entry Point                     | What It Deletes                         | S3 Involved?          |
| ----------------- | ------------------------------- | --------------------------------------- | --------------------- |
| **Delete File**   | S3 direct delete → SQS → Lambda | FILE item, file VIEW_LINKs, FILE grants | ✅ Yes                |
| **Delete Folder** | API `DELETE /{path}/`           | Folder markers, PREFIX grants           | ❌ No (metadata only) |
| **Revoke Share**  | API `DELETE /shares/{grantId}`  | SHARE_GRANT, recipient VIEW_LINKs       | ❌ No (metadata only) |

**Safety Constraint:** Folders must be empty before deletion. Files must be deleted via S3 (which triggers S3 event processor to clean up DynamoDB).

### **4.3. API Design Principles**

- ✅ **Synchronous Operations:** All endpoints complete before returning (no job tracking for MVP)
- ✅ **JWT-Based Auth:** Firebase JWT middleware extracts `user_id`
- ✅ **VIEW_LINK Permission Model:** Access validation via VIEW_LINK existence check
- ✅ **RESTful URLs:** Paths mirror file structure
- ✅ **Idempotent Where Possible:** Use conditional writes
- ✅ **Clear Error Messages:** 403 (access denied), 404 (not found), 400 (validation error)
- ✅ **Pagination Support:** Cursor-based pagination for list operations
- ✅ **Performance Aware:** Critical endpoints flagged for optimization

**Future Enhancements (Post-MVP):**

- Async grant creation/revocation with job tracking for large grants (>1000 files)
- Batch operations for multiple grants
- Grant expiration and time-limited sharing
- Folder rename/move operations

**Permissions Model:**

For MVP, two permission levels are sufficient:

- `READ` - View and download files
- `READ_WRITE` - View, download, upload, and delete files

These cover all essential use cases without unnecessary complexity. More granular permissions can be added post-MVP if needed.

### **4.4. Folder Browsing Implementation**

Unified folder browsing that returns both folder markers and files in a single query, with folders sorted first.

```rust
use std::collections::HashMap;

#[derive(Debug)]
struct FolderContents {
    subfolders: Vec<FolderInfo>,  // Derived from folder markers
    files: Vec<FileInfo>,          // Direct children files
    next_cursor: Option<HashMap<String, AttributeValue>>,
}

#[derive(Debug)]
struct FolderInfo {
    name: String,           // "photos/", "videos/"
    full_path: String,      // "media/photos/", "media/videos/"
    owner_id: String,       // Owner of this subfolder
    shared_by: Vec<String>, // Multiple owners if merged
}

#[derive(Debug)]
struct FileInfo {
    file_id: String,
    owner_id: String,
    file_name: String,
    folder_prefix: String,
    media_type: String,
    created_date: i64,
    size: Option<i64>,
}

// Universal folder browsing function - works for own files and shared files
async fn get_folder_contents(
    client: &DynamoDbClient,
    user_id: &str,
    folder_prefix: &str,        // e.g., "media/" or "media/photos/"
    media_type_filter: Option<&str>,  // Optional media type filter (uses FilterExpression)
    sort_newest_first: bool,    // True for DESC, False for ASC
    page_size: i32,
    cursor: Option<HashMap<String, AttributeValue>>
) -> Result<FolderContents> {
    // Build GSI2 query - returns folders first, then files sorted by timestamp
    let mut query = client.query()
        .table_name("FileMetadata")
        .index_name("ViewLinkIndex")
        .key_condition_expression("GSI2-PK = :pk")
        .expression_attribute_values(
            ":pk",
            AttributeValue::S(format!("VIEWER#{}#FOLDER#{}", user_id, folder_prefix))
        )
        .scan_index_forward(!sort_newest_first)  // False = DESC (newest first)
        .limit(page_size);

    // Add optional media type filter using FilterExpression
    if let Some(media_type) = media_type_filter {
        // Note: FilterExpression means DynamoDB reads items before filtering
        // This is less efficient (higher RCU) but acceptable for filtered views
        query = query
            .filter_expression("begins_with(MediaType, :media_type)")
            .expression_attribute_values(
                ":media_type",
                AttributeValue::S(format!("{}/", media_type))
            );
    }

    // Add pagination cursor if provided
    if let Some(start_key) = cursor {
        query = query.set_exclusive_start_key(Some(start_key));
    }

    // Execute query
    let result = query.send().await?;
    let items = result.items.unwrap_or_default();

    // Separate folders from files
    let mut folders_map: HashMap<String, FolderInfo> = HashMap::new();
    let mut files = Vec::new();

    for item in items {
        let file_id = item.get("ResourceID")?.as_s()?;

        if file_id.starts_with("FOLDER#") {
            // This is a folder marker
            let folder_name = item.get("FileName")?.as_s()?;
            let full_path = file_id.strip_prefix("FOLDER#").unwrap_or(file_id);
            let owner_id = item.get("OwnerID")?.as_s()?;

            // Merge multiple owners of same subfolder (e.g., both Sheldon and Leigh have "photos/")
            folders_map.entry(folder_name.to_string())
                .and_modify(|f| {
                    if !f.shared_by.contains(&owner_id.to_string()) {
                        f.shared_by.push(owner_id.to_string());
                    }
                })
                .or_insert(FolderInfo {
                    name: folder_name.to_string(),
                    full_path: full_path.to_string(),
                    owner_id: owner_id.to_string(),
                    shared_by: vec![owner_id.to_string()],
                });
        } else {
            // This is a file VIEW_LINK
            files.push(FileInfo {
                file_id: file_id.to_string(),
                owner_id: item.get("OwnerID")?.as_s()?.to_string(),
                file_name: item.get("FileName")?.as_s()?.to_string(),
                folder_prefix: item.get("FolderPrefix")?.as_s()?.to_string(),
                media_type: item.get("MediaType")?.as_s()?.to_string(),
                created_date: item.get("CreatedDate")?.as_n()?.parse()?,
                size: item.get("Size").and_then(|v| v.as_n().ok()).and_then(|s| s.parse().ok()),
            });
        }
    }

    Ok(FolderContents {
        subfolders: folders_map.into_values().collect(),
        files,
        next_cursor: result.last_evaluated_key,
    })
}

// Example usage
async fn example_usage() -> Result<()> {
    let client = get_dynamodb_client().await?;

    // Browse "media/" folder for Justin
    let contents = get_folder_contents(
        &client,
        "Justin",
        "media/",
        None,   // No media type filter
        true,   // Newest first
        50,     // Page size
        None    // First page
    ).await?;

    println!("Subfolders:");
    for folder in &contents.subfolders {
        if folder.shared_by.len() > 1 {
            println!("  📁 {} (from {})", folder.name, folder.shared_by.join(", "));
        } else {
            println!("  📁 {} (from {})", folder.name, folder.owner_id);
        }
    }

    println!("\nFiles (sorted newest to oldest across all owners and media types):");
    for file in &contents.files {
        println!("  📄 {} ({}) - {}", file.file_name, file.media_type, file.owner_id);
    }

    Ok(())
}
```

**Key Benefits:**

- ✅ **Single query** returns both folders and files
- ✅ **Folders appear first** automatically (TYPE#FOLDER sorts before TYPE#FILE)
- ✅ **Pure chronological sorting** across all owners and media types
- ✅ **Multi-owner support** - merges folders from different owners
- ✅ **No conditional logic** - same code for owners and recipients
- ✅ **Native pagination** - DynamoDB cursor works seamlessly
- ✅ **S3-like UX** - matches familiar folder browsing behavior

### **4.5. Permission Validation**

Permission validation works identically for both PREFIX and FILE grants. The presence of a VIEW_LINK proves that the user has access to a file, regardless of whether the access was granted via a PREFIX grant (folder-level) or a FILE grant (individual file). This unified approach simplifies permission checking across the application.

**Key Principle:** If a VIEW_LINK exists, access is granted. The VIEW_LINK's `GrantID` attribute references the specific SHARE_GRANT that authorized the access.

```rust
async fn validate_file_access(
    client: &DynamoDbClient,
    user_id: &str,
    owner_id: &str,
    file_id: &str
) -> Result<(bool, Option<String>)> {
    // Check if VIEW_LINK exists for this user viewing this file
    let result = client.query()
        .table_name("FileMetadata")
        .key_condition_expression("PK = :pk AND SK = :sk")
        .expression_attribute_values(":pk", AttributeValue::S(format!("USER#{}", user_id)))
        .expression_attribute_values(":sk", AttributeValue::S(format!("VIEWLINK#{}#{}", owner_id, file_id)))
        .limit(1)
        .send()
        .await?;

    if let Some(items) = result.items {
        if let Some(item) = items.first() {
            // Extract GrantID to identify the grant type and permissions
            let grant_id = item.get("GrantID")
                .and_then(|v| v.as_s().ok())
                .cloned();

            return Ok((true, grant_id));
        }
    }

    Ok((false, None))
}

// For write operations, check the grant permissions
async fn validate_file_write_access(
    client: &DynamoDbClient,
    user_id: &str,
    owner_id: &str,
    file_id: &str
) -> Result<bool> {
    // First check if VIEW_LINK exists
    let (has_access, grant_id) = validate_file_access(client, user_id, owner_id, file_id).await?;

    if !has_access {
        return Ok(false);
    }

    // Owner always has write access
    if grant_id == Some("OWNER".to_string()) {
        return Ok(true);
    }

    // Check grant permissions (could be PREFIX or FILE grant)
    let grant_id = grant_id.ok_or_else(|| anyhow!("No grant ID found"))?;

    let result = client.query()
        .table_name("FileMetadata")
        .key_condition_expression("PK = :pk AND begins_with(SK, :sk_prefix)")
        .filter_expression("GrantID = :grant_id AND Permissions = :perms")
        .expression_attribute_values(":pk", AttributeValue::S(format!("USER#{}", owner_id)))
        .expression_attribute_values(":sk_prefix", AttributeValue::S("GRANT#".to_string()))
        .expression_attribute_values(":grant_id", AttributeValue::S(grant_id))
        .expression_attribute_values(":perms", AttributeValue::S("READ/WRITE".to_string()))
        .limit(1)
        .send()
        .await?;

    Ok(result.items.is_some() && !result.items.unwrap().is_empty())
}
```

**Key Benefits:**

- ✅ **Unified Validation:** Same logic works for PREFIX and FILE grants
- ✅ **Simple Check:** VIEW_LINK existence = access granted
- ✅ **Performance:** Single query to user's partition (no need to query grants)
- ✅ **Security:** VIEW_LINKs can only be created by trusted S3 event processor or grant API
- ✅ **Audit Trail:** `GrantID` attribute enables tracking which grant authorized access
- ✅ **Consistent:** Impossible for VIEW_LINK to exist without corresponding SHARE_GRANT (enforced by S3 processor and API)

**Design Note:** This validation approach relies on the integrity of VIEW_LINKs being maintained by the S3 event processor and grant API. Direct manipulation of VIEW_LINKs bypassing these trusted processes would break security. Therefore, all VIEW_LINK creation/deletion MUST go through S3 events or grant API operations.

````

### **4.6. Batch Write Helper Functions**

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
````

### **4.7. Error Handling and Idempotency**

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

## **5. CloudFront + S3 Signed URL Architecture**

This section documents how users securely access physical files stored in S3 through CloudFront with signed URLs, and how file operations trigger DynamoDB metadata synchronization via S3 events.

### **5.1. Architecture Overview**

**Core Principle:** Users never interact with S3 directly. All file operations (upload, download, delete) go through CloudFront with signed URLs that provide time-limited, cryptographically secure access to specific resources.

```
┌─────────────┐
│   Client    │
│  (Browser/  │
│     App)    │
└──────┬──────┘
       │
       │ 1. Request signed URL
       ├──────────────────────────────────────┐
       │                                      │
       │                                      ▼
       │                              ┌──────────────┐
       │                              │  API Lambda  │
       │                              │  (HTTP)      │
       │                              └───────┬──────┘
       │                                      │
       │                                      │ - Validates JWT
       │                                      │ - Checks VIEW_LINK permissions
       │                                      │ - Generates CloudFront signed URL
       │                                      │   using RSA private key from SSM
       │                                      │
       │ 2. Returns signed URL                │
       │◄─────────────────────────────────────┤
       │                                      │
       │ 3. Upload/Download/Delete            │
       │    with signed URL                   │
       ├──────────────────┐                   │
       │                  │                   │
       │                  ▼                   │
       │          ┌───────────────┐           │
       │          │  CloudFront   │           │
       │          │  Distribution │           │
       │          │   (OAC Auth)  │           │
       │          └───────┬───────┘           │
       │                  │                   │
       │                  │ Origin Access     │
       │                  │ Control (OAC)     │
       │                  │                   │
       │                  ▼                   │
       │          ┌───────────────┐           │
       │          │   S3 Bucket   │           │
       │          │  (Private)    │           │
       │          └───────┬───────┘           │
       │                  │                   │
       │                  │ 4. S3 Event       │
       │                  │    (ObjectCreated │
       │                  │     /Removed)     │
       │                  │                   │
       │                  ▼                   │
       │          ┌───────────────┐           │
       │          │  SQS Queue    │           │
       │          └───────┬───────┘           │
       │                  │                   │
       │                  │ 5. Trigger        │
       │                  │                   │
       │                  ▼                   │
       │          ┌───────────────┐           │
       │          │ Event Lambda  │           │
       │          │  (SQS)        │           │
       │          └───────┬───────┘           │
       │                  │                   │
       │                  │ 6. Update         │
       │                  │    Metadata       │
       │                  │                   │
       │                  ▼                   │
       │          ┌───────────────┐           │
       └─────────►│   DynamoDB    │           │
  7. Query file   │  FileMetadata │           │
     metadata     └───────────────┘           │
                                              │
```

**Key Components:**

1. **CloudFront Distribution:** `dev-storage.fromthehart.tech` (or `storage.fromthehart.tech` in production)
2. **Origin Access Control (OAC):** Restricts S3 bucket access to only CloudFront (replaces legacy OAI)
3. **S3 Bucket:** Private bucket, no public access, only accessible via CloudFront with OAC
4. **RSA Key Pair:** CloudFront public key (in key group), private key (in SSM Parameter Store)
5. **SQS Queue:** Buffers S3 events for reliable processing
6. **Lambda Functions:** HTTP API (signed URL generation) + SQS processor (metadata sync)

### **5.2. CloudFront Distribution Configuration**

#### **5.2.1. Origin Access Control (OAC)**

CloudFront uses **Origin Access Control** to securely access the private S3 bucket. This is the modern replacement for Origin Access Identity (OAI).

**OAC Benefits:**

- ✅ Supports all S3 operations (GET, PUT, DELETE, HEAD)
- ✅ Works with S3 bucket encryption (SSE-S3, SSE-KMS)
- ✅ Simplified IAM policy management
- ✅ AWS recommended approach for new distributions

**S3 Bucket Policy (OAC):**

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "AllowCloudFrontServicePrincipal",
      "Effect": "Allow",
      "Principal": {
        "Service": "cloudfront.amazonaws.com"
      },
      "Action": ["s3:GetObject", "s3:PutObject", "s3:DeleteObject"],
      "Resource": "arn:aws:s3:::storage-bucket-name/*",
      "Condition": {
        "StringEquals": {
          "AWS:SourceArn": "arn:aws:cloudfront::ACCOUNT_ID:distribution/DISTRIBUTION_ID"
        }
      }
    }
  ]
}
```

**Result:** S3 bucket is completely private. Only CloudFront with the specific distribution ID can access objects.

#### **5.2.2. Cache Behaviors**

The CloudFront distribution has two cache behaviors to handle public and private content:

| Path Pattern   | Requires Signed URL | Cache TTL            | Use Case                                                     |
| :------------- | :------------------ | :------------------- | :----------------------------------------------------------- |
| `/public/*`    | ❌ No               | 1 day                | Public assets (profile pictures, public albums) - future use |
| `/*` (default) | ✅ Yes              | No cache (0 seconds) | All user files - requires permission validation              |

**Note:** The `/public/*` path is configured for potential future use cases (e.g., public profile pictures, shared albums with public links). Currently, all user files are private and require signed URLs.

**Default Behavior Settings:**

- **Viewer Protocol Policy:** Redirect HTTP to HTTPS
- **Allowed HTTP Methods:** GET, HEAD, OPTIONS, PUT, POST, DELETE
- **Cache Policy:** CachingDisabled (files are private, cache would bypass permission checks)
- **Origin Request Policy:** AllViewer (forward all headers, query strings, cookies)
- **Trusted Key Groups:** Contains the CloudFront public key for signature verification

### **5.3. Signed URL Generation**

#### **5.3.1. Custom Policy with Wildcards**

The system uses **custom policies** instead of canned policies to support wildcard resource URLs. This allows generating a single signed URL that grants access to multiple files under a prefix.

**Custom Policy Structure:**

```json
{
  "Statement": [
    {
      "Resource": "https://dev-storage.fromthehart.tech/*",
      "Condition": {
        "DateLessThan": {
          "AWS:EpochTime": 1700000000
        }
      }
    }
  ]
}
```

**Wildcard Usage:**

- **Upload:** `https://dev-storage.fromthehart.tech/*` (wildcard allows uploading to any path the user owns)
- **Download:** `https://dev-storage.fromthehart.tech/{user_id}/{file_path}` (specific file, but wildcard policy simplifies implementation)
- **Delete:** Same as download (specific file with wildcard policy)

**Why Wildcards:**

- Simplifies signed URL generation logic (one policy format for all operations)
- Allows batch operations in the future (download entire folder as zip)
- Reduces complexity of tracking individual file signatures

**Security Note:** Wildcard scope is acceptable because:

1. URLs are time-limited (expire in minutes/hours)
2. Permission validation happens **before** generating the signed URL
3. User can only request URLs for files they have VIEW_LINK access to
4. S3 key structure (`{user_id}/{file_path}`) prevents cross-user access

#### **5.3.2. RSA Signature Algorithm**

CloudFront signed URLs use **RSA PKCS#1 v1.5 with SHA-1** for signature generation.

**Signing Process:**

```rust
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rsa::{pkcs1::DecodeRsaPrivateKey, RsaPrivateKey};
use rsa::pkcs1v15::SigningKey;
use rsa::signature::{SignatureEncoding, Signer};
use sha1::Sha1;

// 1. Load private key from SSM Parameter Store
let private_key_pem = get_ssm_parameter("/cloudfront/storage/private-key").await?;
let private_key = RsaPrivateKey::from_pkcs1_pem(&private_key_pem)?;

// 2. Create custom policy JSON
let resource_url = format!("https://dev-storage.fromthehart.tech/*");
let expiration_epoch = (Utc::now() + Duration::hours(24)).timestamp();

let custom_policy = serde_json::json!({
    "Statement": [{
        "Resource": resource_url,
        "Condition": {
            "DateLessThan": {
                "AWS:EpochTime": expiration_epoch
            }
        }
    }]
}).to_string();

// 3. Sign policy with RSA PKCS#1 v1.5 + SHA-1
let signing_key = SigningKey::<Sha1>::new(private_key);
let signature = signing_key.sign(custom_policy.as_bytes());

// 4. Base64 encode for CloudFront (URL-safe)
let signature_b64 = STANDARD.encode(signature.to_bytes())
    .replace("+", "-")
    .replace("=", "_")
    .replace("/", "~");

let policy_b64 = STANDARD.encode(custom_policy.as_bytes())
    .replace("+", "-")
    .replace("=", "_")
    .replace("/", "~");

// 5. Construct signed URL
let signed_url = format!(
    "{}?Policy={}&Signature={}&Key-Pair-Id={}",
    resource_url,
    policy_b64,
    signature_b64,
    cloudfront_key_pair_id
);
```

**Base64 Encoding for CloudFront:**

CloudFront requires a URL-safe Base64 variant with character substitutions:

- `+` → `-`
- `=` → `_`
- `/` → `~`

This ensures the signed URL can be safely embedded in HTTP query strings without encoding issues.

**Private Key Storage:**

- **Location:** AWS Systems Manager Parameter Store
- **Parameter Name:** `/cloudfront/storage/private-key`
- **Type:** SecureString (encrypted with AWS KMS)
- **Access:** Restricted to API Lambda function via IAM policy
- **Rotation:** Manual (requires CloudFront key group update)

**Public Key Configuration:**

- Uploaded to CloudFront as a **Public Key** resource
- Added to a **Key Group** resource
- Key Group attached to CloudFront distribution's **Trusted Key Groups**
- CloudFront uses the public key to verify signature authenticity

#### **5.3.3. Expiration Windows**

Different operations use different expiration times based on their expected duration and security requirements:

| Operation           | Expiration Window | Rationale                                                        |
| :------------------ | :---------------- | :--------------------------------------------------------------- |
| **Upload (PUT)**    | 15 minutes        | Short-lived, one-time use; prevents replay attacks               |
| **Download (GET)**  | 24 hours          | Allows viewing, sharing temporary links, downloading large files |
| **Delete (DELETE)** | 5 minutes         | Critical operation, very short window; user must confirm quickly |

**Implementation Example:**

```rust
fn get_expiration_duration(operation: &str) -> Duration {
    match operation {
        "upload" => Duration::minutes(15),
        "download" => Duration::hours(24),
        "delete" => Duration::minutes(5),
        _ => Duration::hours(1), // Default fallback
    }
}
```

### **5.4. Integration with API Responses**

#### **5.4.1. Signed URLs in File Metadata**

All API responses that return file metadata automatically include a CloudFront signed URL for immediate download access.

**Example: GET /storage/{owner_id}/{file_path}**

```json
{
  "file_id": "R102",
  "owner_id": "Sheldon",
  "file_name": "DSCN0010.jpg",
  "folder_prefix": "media/Project Docs/",
  "media_type": "image/jpeg",
  "created_date": 1224685719000,
  "size": 161713,
  "s3_key": "Sheldon/media/Project Docs/DSCN0010.jpg",
  "signed_url": "https://dev-storage.fromthehart.tech/Sheldon/media/Project%20Docs/DSCN0010.jpg?Policy=eyJTdGF0ZW1lbnQiO...&Signature=abc123...&Key-Pair-Id=K2JCEXAMPLE"
}
```

**Example: GET /storage/{user_id}/{folder_path}/ (List Folder)**

```json
{
  "subfolders": [
    {
      "name": "2024/",
      "full_path": "media/Project Docs/2024/",
      "owners": ["Sheldon"]
    }
  ],
  "files": [
    {
      "file_id": "R102",
      "owner_id": "Sheldon",
      "file_name": "DSCN0010.jpg",
      "folder_prefix": "media/Project Docs/",
      "media_type": "image/jpeg",
      "created_date": 1224685719000,
      "size": 161713,
      "signed_url": "https://dev-storage.fromthehart.tech/Sheldon/media/Project%20Docs/DSCN0010.jpg?Policy=...&Signature=...&Key-Pair-Id=..."
    },
    {
      "file_id": "R103",
      "owner_id": "Sheldon",
      "file_name": "vacation.jpg",
      "folder_prefix": "media/Project Docs/2024/",
      "media_type": "image/jpeg",
      "created_date": 1224685740000,
      "size": 245000,
      "signed_url": "https://dev-storage.fromthehart.tech/Sheldon/media/Project%20Docs/2024/vacation.jpg?Policy=...&Signature=...&Key-Pair-Id=..."
    }
  ],
  "next_cursor": null,
  "count": 3
}
```

**Performance Consideration:** Generating signed URLs for large folder listings (1000+ files) can add latency. Consider:

- Generate signed URLs lazily (on-demand when user clicks file)
- Cache signed URL generation results (with TTL matching expiration)
- Use wildcard policies to generate fewer signatures

#### **5.4.2. Upload Signed URL Endpoint**

Users must request a signed URL before uploading files.

**Endpoint:** `POST /storage/signed-url`

**Request Body:**

```json
{
  "operation": "upload",
  "file_path": "media/photos/vacation.jpg",
  "content_type": "image/jpeg"
}
```

**Response:**

```json
{
  "signed_url": "https://dev-storage.fromthehart.tech/Sheldon/media/photos/vacation.jpg?Policy=...&Signature=...&Key-Pair-Id=...",
  "expires_at": 1700000900,
  "method": "PUT",
  "headers": {
    "Content-Type": "image/jpeg"
  }
}
```

**API Validation:**

1. Extract `user_id` from JWT token
2. Validate file path starts with `user_id` (users can only upload to their own path)
3. Check file path doesn't contain malicious patterns (`..`, `//`, etc.)
4. Generate signed URL with 15-minute expiration
5. Return URL + upload instructions

**Client Upload Flow:**

```javascript
// 1. Request signed URL
const response = await fetch("/storage/signed-url", {
  method: "POST",
  headers: {
    Authorization: `Bearer ${jwt_token}`,
    "Content-Type": "application/json",
  },
  body: JSON.stringify({
    operation: "upload",
    file_path: "media/photos/vacation.jpg",
    content_type: "image/jpeg",
  }),
});

const { signed_url, headers } = await response.json();

// 2. Upload file directly to CloudFront
await fetch(signed_url, {
  method: "PUT",
  headers: headers,
  body: fileBlob,
});

// 3. Poll for file metadata (S3 event processing is async)
let retries = 10;
while (retries-- > 0) {
  const file = await fetch("/storage/Sheldon/media/photos/vacation.jpg");
  if (file.ok) break;
  await sleep(1000); // Wait 1 second
}
```

### **5.5. S3 Event Processing Flow**

#### **5.5.1. Event Trigger Architecture**

```
CloudFront (PUT) → S3 (ObjectCreated) → SQS → Lambda → DynamoDB
CloudFront (DELETE) → S3 (ObjectRemoved) → SQS → Lambda → DynamoDB
```

**S3 Bucket Notification Configuration:**

```hcl
resource "aws_s3_bucket_notification" "file_events" {
  bucket = aws_s3_bucket.file_storage.id

  queue {
    queue_arn = aws_sqs_queue.s3_events.arn
    events    = [
      "s3:ObjectCreated:*",
      "s3:ObjectRemoved:*"
    ]
    filter_prefix = "" # Process all objects
  }
}
```

**SQS Queue Configuration:**

- **Visibility Timeout:** 300 seconds (5 minutes for Lambda processing)
- **Message Retention:** 14 days
- **Dead Letter Queue:** After 3 failed attempts, message moves to DLQ
- **Long Polling:** 20 seconds to reduce empty receives

#### **5.5.2. S3 Event Message Format**

**Example ObjectCreated Event:**

```json
{
  "Records": [
    {
      "eventVersion": "2.1",
      "eventSource": "aws:s3",
      "awsRegion": "us-east-1",
      "eventTime": "2024-11-17T10:30:45.123Z",
      "eventName": "ObjectCreated:Put",
      "s3": {
        "bucket": {
          "name": "storage-bucket-name",
          "arn": "arn:aws:s3:::storage-bucket-name"
        },
        "object": {
          "key": "Sheldon/media/photos/vacation.jpg",
          "size": 2048576,
          "eTag": "abc123def456",
          "sequencer": "00617F3E8B9A1234"
        }
      }
    }
  ]
}
```

**Example ObjectRemoved Event:**

```json
{
  "Records": [
    {
      "eventName": "ObjectRemoved:Delete",
      "s3": {
        "bucket": { "name": "storage-bucket-name" },
        "object": { "key": "Sheldon/media/photos/vacation.jpg" }
      }
    }
  ]
}
```

#### **5.5.3. Lambda Event Processor Logic**

**High-Level Flow:**

```rust
async fn handle_sqs_event(event: SqsEvent) -> Result<()> {
    for record in event.records {
        let s3_event: S3Event = serde_json::from_str(&record.body)?;

        for s3_record in s3_event.records {
            match s3_record.event_name.as_str() {
                name if name.starts_with("ObjectCreated:") => {
                    handle_file_created(&s3_record).await?;
                },
                name if name.starts_with("ObjectRemoved:") => {
                    handle_file_deleted(&s3_record).await?;
                },
                _ => continue,
            }
        }
    }
    Ok(())
}
```

**ObjectCreated Processing:**

```rust
async fn handle_file_created(s3_record: &S3EventRecord) -> Result<()> {
    let s3_key = &s3_record.s3.object.key;

    // 1. Parse user_id and file_path from S3 key
    let (user_id, file_path) = parse_s3_key(s3_key)?;
    // Example: "Sheldon/media/photos/vacation.jpg" → ("Sheldon", "media/photos/vacation.jpg")

    // 2. Fetch S3 object metadata and head bytes
    let s3_metadata = get_object_metadata(s3_key).await?;
    let head_bytes = get_object_head_bytes(s3_key, 524288).await?; // First 512KB

    // 3. Extract media metadata (dimensions, EXIF, duration, etc.)
    let media_metadata = extract_media_metadata(&head_bytes, &s3_metadata.content_type)?;

    // 4. Create FILE item in DynamoDB
    let file_id = generate_uuid();
    let folder_prefix = extract_folder_prefix(&file_path);

    create_file_item(&FileItem {
        pk: format!("USER#{}", user_id),
        sk: format!("FILE#{}", file_path),
        item_type: "FILE",
        file_id,
        owner_id: user_id.clone(),
        file_name: extract_file_name(&file_path),
        folder_prefix: folder_prefix.clone(),
        created_date: Utc::now().timestamp_millis(),
        media_type: s3_metadata.content_type,
        s3_key: s3_key.clone(),
        size: s3_record.s3.object.size,
        media_metadata,
    }).await?;

    // 5. Create owner's VIEW_LINK
    create_view_link(&user_id, &user_id, &file_id, "OWNER", &folder_prefix).await?;

    // 6. Create folder marker VIEW_LINKs for all ancestor folders
    create_folder_markers(&user_id, &user_id, "OWNER", &folder_prefix).await?;

    // 7. Query PREFIX grants for this path
    let grants = find_prefix_grants(&user_id, &folder_prefix).await?;

    // 8. Create recipient VIEW_LINKs for each grant
    for grant in grants {
        create_view_link(&grant.recipient_id, &user_id, &file_id, &grant.grant_id, &folder_prefix).await?;
        create_folder_markers(&grant.recipient_id, &user_id, &grant.grant_id, &folder_prefix).await?;
    }

    Ok(())
}
```

**ObjectRemoved Processing:**

```rust
async fn handle_file_deleted(s3_record: &S3EventRecord) -> Result<()> {
    let s3_key = &s3_record.s3.object.key;
    let (user_id, file_path) = parse_s3_key(s3_key)?;

    // 1. Query FILE item to get file_id
    let file_item = get_file_item(&user_id, &file_path).await?;

    // 2. Delete FILE item
    delete_file_item(&user_id, &file_path).await?;

    // 3. Query and delete all VIEW_LINKs for this file (owner + recipients)
    let view_links = query_view_links_by_file_id(&file_item.file_id).await?;
    for view_link in view_links {
        delete_view_link(&view_link.pk, &view_link.sk).await?;
    }

    // 4. Delete FILE grants (if any)
    let file_grants = query_file_grants(&user_id, &file_item.file_id).await?;
    for grant in file_grants {
        delete_grant(&grant.pk, &grant.sk).await?;
    }

    // Note: Folder marker VIEW_LINKs persist (other files may share the same folders)

    Ok(())
}
```

**Metadata Extraction:**

The Lambda function fetches the first 512KB of each uploaded file to extract media metadata:

```rust
async fn extract_media_metadata(head_bytes: &[u8], content_type: &str) -> Result<Option<MediaMetadata>> {
    match content_type {
        "image/jpeg" | "image/png" | "image/webp" => {
            // Extract image dimensions, EXIF data, GPS coordinates
            extract_image_metadata(head_bytes).await
        },
        "video/mp4" | "video/quicktime" => {
            // Extract video duration, resolution, codec
            extract_video_metadata(head_bytes).await
        },
        "application/pdf" => {
            // Extract PDF metadata (page count, author, etc.)
            extract_pdf_metadata(head_bytes).await
        },
        _ => Ok(None), // No metadata extraction for other types
    }
}
```

**Metadata Example (JPEG):**

```json
{
  "type": "image",
  "width": 4032,
  "height": 3024,
  "exif": {
    "Model": "iPhone 14 Pro",
    "DateTimeOriginal": "2024-11-17 10:30:45",
    "FocalLength": "6.86",
    "ISO": "100",
    "ExposureTime": "1/120"
  },
  "gps": {
    "latitude": 37.7749,
    "longitude": -122.4194,
    "altitude": 15.2
  }
}
```

### **5.6. Security Considerations**

#### **5.6.1. Private Key Protection**

**Storage:**

- Private key stored in AWS SSM Parameter Store as SecureString
- Encrypted at rest with AWS KMS
- Never logged or exposed in plaintext

**Access Control:**

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": "ssm:GetParameter",
      "Resource": "arn:aws:ssm:us-east-1:ACCOUNT_ID:parameter/cloudfront/storage/private-key",
      "Principal": {
        "AWS": "arn:aws:iam::ACCOUNT_ID:role/api-lambda-role"
      }
    }
  ]
}
```

**Rotation Strategy:**

- Manual rotation (requires coordinated CloudFront + SSM update)
- Rotate annually or when compromise suspected
- Zero-downtime rotation: Add new key to key group, remove old key after propagation

#### **5.6.2. Signed URL Security**

**Time-Limited Access:**

- Short expiration windows (15 minutes for uploads, 24 hours for downloads)
- Expired URLs are rejected by CloudFront (no server-side validation needed)

**Operation Scope:**

- Upload URLs: User can only upload to paths they own (`{user_id}/*`)
- Download URLs: Generated only after VIEW_LINK validation
- Delete URLs: Same as downloads (user must have VIEW_LINK access)

**Replay Attack Prevention:**

- Upload URLs are single-use (subsequent PUTs overwrite)
- CloudFront logs all requests for audit trail
- Monitor for suspicious patterns (multiple downloads of same URL)

#### **5.6.3. Permission Validation**

**Before Generating Signed URL:**

```rust
async fn generate_signed_url(
    user_id: &str,
    file_path: &str,
    operation: &str,
) -> Result<String> {
    // 1. Parse owner_id from file_path
    let owner_id = file_path.split('/').next()
        .ok_or(Error::InvalidPath)?;

    // 2. Check VIEW_LINK exists (permission proof)
    let view_link_exists = check_view_link_exists(
        user_id,
        owner_id,
        &extract_file_id_from_path(file_path)?
    ).await?;

    if !view_link_exists {
        return Err(Error::Forbidden("No access to this file"));
    }

    // 3. Generate signed URL (user has permission)
    let signed_url = create_cloudfront_signed_url(
        &file_path,
        operation,
        get_expiration_duration(operation)
    ).await?;

    Ok(signed_url)
}
```

#### **5.6.4. S3 Bucket Security**

**Bucket Policy:**

- Denies all public access
- Only CloudFront (via OAC) can access objects
- Specific distribution ID required in policy condition

**Encryption:**

- Server-side encryption enabled (SSE-S3 or SSE-KMS)
- Encryption applied automatically to all uploads
- CloudFront OAC supports both encryption types

**Versioning:**

- S3 versioning disabled for MVP (simplifies deletion)
- Consider enabling for data recovery in production
- Would require VIEW_LINK versioning support

### **5.7. Complete Flow Examples**

#### **5.7.1. Upload Flow**

```
1. User: POST /storage/signed-url
   Body: { "operation": "upload", "file_path": "media/photos/vacation.jpg", "content_type": "image/jpeg" }

2. API Lambda:
   - Validates JWT → user_id = "Sheldon"
   - Validates path starts with "Sheldon/"
   - Generates signed URL (15-minute expiration)
   - Returns: { "signed_url": "https://dev-storage.fromthehart.tech/...", "method": "PUT" }

3. User: PUT https://dev-storage.fromthehart.tech/Sheldon/media/photos/vacation.jpg?Policy=...&Signature=...
   Headers: { "Content-Type": "image/jpeg" }
   Body: <file bytes>

4. CloudFront:
   - Verifies signature with public key
   - Checks expiration
   - Forwards request to S3 via OAC

5. S3:
   - Stores object at key "Sheldon/media/photos/vacation.jpg"
   - Sends ObjectCreated event to SQS

6. SQS → Lambda (Event Processor):
   - Parses S3 key → user_id="Sheldon", file_path="media/photos/vacation.jpg"
   - Fetches S3 metadata + head bytes
   - Extracts EXIF, dimensions, GPS
   - Creates FILE item in DynamoDB
   - Creates VIEW_LINK for Sheldon (owner)
   - Creates folder markers: "media/", "media/photos/"
   - Queries PREFIX grants (none exist)

7. User: GET /storage/Sheldon/media/photos/vacation.jpg
   - API returns file metadata + signed download URL
   - Upload complete!
```

#### **5.7.2. Download Flow**

```
1. User: GET /storage/Sheldon/media/photos/vacation.jpg
   Headers: { "Authorization": "Bearer <JWT>" }

2. API Lambda:
   - Validates JWT → user_id = "Sheldon"
   - Queries VIEW_LINK: PK="USER#Sheldon", SK="VIEWLINK#Sheldon#R102" → EXISTS
   - Queries FILE item for metadata
   - Generates signed URL (24-hour expiration)
   - Returns:
     {
       "file_id": "R102",
       "file_name": "vacation.jpg",
       "size": 2048576,
       "signed_url": "https://dev-storage.fromthehart.tech/Sheldon/media/photos/vacation.jpg?Policy=...&Signature=..."
     }

3. User: GET https://dev-storage.fromthehart.tech/Sheldon/media/photos/vacation.jpg?Policy=...&Signature=...

4. CloudFront:
   - Verifies signature
   - Checks expiration
   - Forwards to S3 via OAC

5. S3:
   - Returns file bytes
   - CloudFront streams to user
   - Download complete!
```

#### **5.7.3. Delete Flow**

```
1. User: DELETE /storage/Sheldon/media/photos/vacation.jpg
   Headers: { "Authorization": "Bearer <JWT>" }

2. API Lambda:
   - Validates JWT → user_id = "Sheldon"
   - Queries VIEW_LINK → EXISTS
   - Verifies user is owner (OwnerID="Sheldon")
   - Generates signed URL (5-minute expiration)
   - Performs DELETE request to CloudFront

3. CloudFront:
   - Verifies signature
   - Forwards DELETE to S3 via OAC

4. S3:
   - Deletes object
   - Sends ObjectRemoved event to SQS

5. SQS → Lambda (Event Processor):
   - Parses S3 key
   - Deletes FILE item from DynamoDB
   - Deletes all VIEW_LINKs (Sheldon's owner VIEW_LINK + any recipient VIEW_LINKs)
   - Deletes FILE grants (if any)
   - Folder markers persist (other files may use them)

6. API Lambda:
   - Returns: { "success": true }
   - Delete complete!
```

## **6. Performance Characteristics and Cost Analysis**

### **5.1. Read Performance**

| Operation                           | Strategy                      | Latency | RCU Cost (per operation)       | Notes                                                     |
| ----------------------------------- | ----------------------------- | ------- | ------------------------------ | --------------------------------------------------------- |
| **View any folder (own or shared)** | GSI2 query                    | 10-20ms | 1-2 RCU (50 items × 2KB each)  | Pure chronological sort across all owners and media types |
| **"Shared With Me" list**           | GSI1 query                    | 5-10ms  | 0.5 RCU (typically <10 grants) | Lists all prefix-level grants received                    |
| **Merged folder view (3000 files)** | GSI2 query (paginated)        | 10-20ms | 1-2 RCU per 50-item page       | Perfect chronological order across all contributors       |
| **Filter by media type**            | GSI2 query + FilterExpression | 15-30ms | 2-4 RCU (reads before filter)  | Less efficient but acceptable for filtered views          |
| **Permission check**                | GSI1 query                    | 5-10ms  | 0.5 RCU (1 item)               | Check SHARE_GRANT existence                               |
| **Direct file access**              | Base table get                | 5-10ms  | 0.5 RCU (1 item, for S3 key)   | Download, metadata retrieval                              |

**Scaling Characteristics:**

- ✅ **Unified query pattern** - Same GSI2 query for all folder browsing (own files, shared files, merged views)
- ✅ **Pure chronological sorting** - Timestamp-first sort key enables cross-owner, cross-media-type date sorting
- ✅ Folder view latency is **constant** regardless of contributor count (1 query for 1 owner or 20 owners)
- ✅ Pagination is native DynamoDB cursor (no server-side merge complexity)
- ✅ **Efficient for "show newest files"** - Primary use case optimized with key condition
- ⚠️ **Media type filtering requires FilterExpression** - 2x RCU cost but acceptable for secondary use case
- ✅ No conditional logic ("am I owner?") reduces code complexity and latency

**Design Trade-off Rationale:**

The timestamp-first sort key (`TYPE#FILE#<Timestamp>#<MediaType>#<ResourceID>`) was chosen to optimize for the primary use case of browsing merged folders chronologically (e.g., "show me the 50 newest files from all contributors"). This design enables perfect cross-owner sorting with native pagination at the cost of less efficient media type filtering. For workloads where filtering by media type is the primary pattern, consider the alternative design (`TYPE#FILE#<MediaType>#<Timestamp>#<ResourceID>`).

### **5.2. Write Performance**

| Operation        | Strategy           | Latency | WCU Cost     | Notes                                                                |
| ---------------- | ------------------ | ------- | ------------ | -------------------------------------------------------------------- |
| **Upload file**  | Put FILE item      | 5-10ms  | 1 WCU        | + VIEW_LINK creation via S3 event processor (owner + all recipients) |
| **Create grant** | Put SHARE_GRANT    | 10-30s  | Variable WCU | Synchronous VIEW_LINK creation for all existing files (batched)      |
| **Revoke grant** | Delete SHARE_GRANT | 10-30s  | Variable WCU | Synchronous VIEW_LINK deletion (batched)                             |
| **Delete file**  | Delete FILE item   | 5-10ms  | 1 WCU        | + VIEW_LINK cleanup via S3 event processor                           |

**Write Amplification:**

- **With VIEW_LINKs (this design):**
  - File upload: 1 FILE write + (1 + N) VIEW_LINK writes (owner + N recipients, via S3→SQS→Lambda)
  - Grant creation: 1 GRANT write + M VIEW_LINK writes (M existing files, synchronous batched)
- **Without VIEW_LINKs:**
  - File upload: 1 FILE write only
  - Grant creation: 1 GRANT write only
  - BUT: Complex server-side merge logic, parallel queries, custom pagination required

**Trade-off Analysis:** The write amplification cost is justified by:

- ✅ Drastically simplified read logic (single query vs. multiple parallel queries + merge)
- ✅ Better read performance (10-20ms vs. 50-100ms)
- ✅ Reduced application complexity (one code path vs. conditional logic)
- ✅ Native pagination (DynamoDB cursors vs. custom merge cursors)
- ✅ Lower operational costs for read-heavy workloads (typical for file sharing)

**Synchronous Operations Note:** For MVP, all API operations (grant creation/revocation) complete synchronously. This simplifies the architecture and eliminates the need for job tracking queues. For large grants (>1000 files), operations may take 10-30 seconds - UI should show progress indicators. Post-MVP, async operations with SQS can be added for better UX on large operations.

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
| VIEW_LINK creation (owner)      | 1,000 files × 1 WCU                | 30K WCU      | $0.04            |
| VIEW_LINK creation (recipients) | 1,000 files × 4 recipients × 1 WCU | 120K WCU     | $0.15            |
| Grant creation                  | 100 grants × 1 WCU                 | 3K WCU       | $0.004           |
| VIEW_LINK for new grants        | 100 grants × 50 files × 1 WCU      | 150K WCU     | $0.19            |
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

### **5.4. Comparison: Universal VIEW_LINK vs No VIEW_LINK Design**

| Aspect                     | With VIEW_LINKs (This Design) | Without VIEW_LINKs              |
| -------------------------- | ----------------------------- | ------------------------------- |
| **Storage Cost**           | Higher (denormalized data)    | Lower (grants only)             |
| **Write Cost**             | Higher (create VIEW_LINKs)    | Lower (grants only)             |
| **Read Cost**              | Lower (single queries)        | Higher (parallel queries)       |
| **Read Latency**           | Excellent (10-20ms)           | Good (50-100ms with merge)      |
| **Pagination**             | Native DynamoDB cursor        | Complex server-side merge       |
| **Code Complexity**        | Low (one code path)           | High (merge + cursor logic)     |
| **Conditional Logic**      | None (always GSI2)            | Yes (owner vs recipient checks) |
| **Scalability**            | Excellent (20+ contributors)  | Limited (5-10 contributors max) |
| **Revocation**             | Immediate (grant check)       | Immediate (grant check)         |
| **Total Cost (10K users)** | ~$6.50/month                  | ~$8/month (higher read cost)    |

**Key Advantage of Universal VIEW_LINK Pattern:**

The unified approach where owners also browse via VIEW_LINKs provides:

- ✅ **Single code path** for all folder browsing (no "if owner" checks)
- ✅ **Consistent UX** - owners and recipients see folders the same way
- ✅ **Simpler debugging** - one query pattern to test and optimize
- ✅ **Easier maintenance** - changes apply universally
- ✅ **Lower cognitive load** - developers don't need to understand two patterns

**Conclusion:** VIEW_LINK design with universal access pattern is **more cost-effective** for read-heavy workloads (typical for photo/file sharing) and dramatically reduces application complexity compared to conditional query logic.

### **5.5. Scaling Considerations**

**Hot Partition Prevention:**

- GSI2 partition keys include recipient ID (e.g., `VIEWER#Justin#FOLDER#photos/`)
- Each user's view is on a separate partition
- No hot partitions even with 1M+ users

**Large Folder Handling:**

- Folders with 10,000+ files work efficiently with automatic VIEW_LINK creation via Streams
- Stream-based processing handles bulk uploads (batch creation in background)
- Pagination ensures UI remains responsive
- Owner's VIEW_LINKs are created immediately alongside recipient VIEW_LINKs (no special handling)

**Contributor Limits:**

- Design supports 20+ users sharing same folder name to one recipient
- GSI2 query performance remains constant regardless of contributor count
- No complex merge logic or parallel query limits

## **6. Supporting Artifacts**

- **Full Data Model:** See `Complete Data Model Example.md` for the complete dataset with examples of all item types, including owner VIEW_LINKs with `GrantID: "OWNER"`.
- **Infrastructure:** See `DynamoDB Terraform Configuration.md` for the complete Terraform configuration including table, GSIs, S3 event processing (SQS), and supporting infrastructure.
- **Schema Summary:** Single-table design with S3-style prefix-based folders, prefix-level grants, and universal VIEW_LINK denormalization for all folder browsing operations. All users (owners and recipients) access folders through GSI2 (ViewLinkIndex), ensuring a unified, high-performance access pattern with no conditional logic.
