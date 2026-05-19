# From The Hart Storage — Domain Glossary

> Canonical terms specific to the storage service. Extends [master CONTEXT.md](../CONTEXT.md).
> Code conventions: [AGENTS.md](./AGENTS.md).

---

## Resources

### ViewLink

A permission-granted projection of a file or folder visible to a specific viewer. The central access-control record in the storage domain. Each ViewLink pairs a viewer with a resource (a **File** or **Folder**) and a grant indicating the access level. Currently all ViewLinks carry an owner grant — only the resource owner can see their own files; sharing is not yet implemented.

- _Avoid:_ "storage item" (website-side name)
- _Relationships:_ A **ViewLink** references exactly one **File** or **Folder** (via **ResourceId**).
  A **File** or **Folder** may be visible to zero or more viewers via **ViewLink**s.

---

## Storage Objects

### File

A stored file with associated metadata. The source of truth for file data. Never exposed directly to clients — always projected through a **ViewLink** first.

- _Avoid:_ "S3 object", "upload" (implementation detail)
- _Relationships:_ A **File** is owned by exactly one **Principal** (currently a Firebase UID — will migrate to **Identity** ID).
  A **File** produces exactly one owner **ViewLink** and zero or more ancestor-folder **ViewLink**s when created.
  A **File** belongs to exactly one **Folder** (or root).

### Folder

A virtual container within a user's storage namespace. A metadata-only entry, not a stored object. Must end with `/`, must not start with `/`, must not contain `//`, `.`, or `..` segments. Parent folders must exist before creating nested folders. Idempotent: creating an existing folder returns its metadata.

- _Avoid:_ "directory" (unix-centric), "bucket prefix" (implementation detail)
- _Relationships:_ A **Folder** has zero or more **File**s and/or child **Folder**s.
  A **Folder** has zero or one parent **Folder** (except root).
  A **Folder** is owned by exactly one **Principal**.

### ResourceId

A tagged union identifying either a **File** or a **Folder**. The domain-level type discriminator for all storage operations.

- _Relationships:_ Used by **ViewLink** to identify which resource is visible.

---

## Access

### Signed Access

Time-limited access to files in a user's directory, granted through a signed capability.

- _Avoid:_ "signed URL", "access token" (signed access covers multiple delivery mechanisms)
- _Relationships:_ **Signed Access** is granted for a specific **Principal**.
  Required before any file can be downloaded from **Storage**.
  Called by the **Website** during file navigation.

---

## Metadata

### MediaType

A categorization of a **File** by its content format. Variants include: Image, Video, Audio, Document, Unknown. Determined during **Metadata Extraction** based on the file's content type and extension.

- _Avoid:_ "content type" (that's the MIME type); "file type" (too generic)
- _Relationships:_ A **File** has exactly one **MediaType**.

### MediaMetadata

Content-derived metadata extracted from a **File** during **Metadata Extraction**. Varies by **MediaType** (e.g., image dimensions and EXIF data for Images).

- _Avoid:_ "EXIF data" (too narrow — MediaMetadata covers non-EXIF metadata too)
- _Relationships:_ A **File** has zero or one **MediaMetadata** (optional — depends on whether extraction succeeded).

---

## Flagged Ambiguities

- **owner_id / viewer_id uses Principal ID, not Identity ID:** These fields currently store Firebase UIDs. Should become **Identity** IDs when the Identity service is built. → See [TODO.md](../TODO.md#27-storage--migrate-owner_idviewer_id-from-firebase-uids-to-identity-ids).
- **"FOLDER#" prefix encoding:** The folder prefix in resource IDs is an implementation detail leaked to the wire format. Should be abstracted away. → See [TODO.md](../TODO.md#9-storage--folder-wire-prefix-cleanup).
- **DTO ViewLink name collision:** The DTO and domain model share the name `ViewLink` with different shapes. Should follow `{Name}Data` pattern. → See [TODO.md](../TODO.md#8-storage-dto--viewlink-name-collision).
- **`X-From-The-Hart-Authorization` custom header:** Custom header name used instead of standard `Authorization`. → See [TODO.md](../TODO.md#28-storage--replace-x-from-the-hart-authorization-custom-header-with-standard-authorization).
- **ShareGrant model exists but is unused:** Scaffolding defined in `src/service/models/share_grant.rs` with no consumers. → See [TODO.md](../TODO.md#29-storage--implement-or-remove-unused-sharegrant-model).
