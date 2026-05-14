# Requirements: Storage Code Review Fixes

> **Source:** Comprehensive code audit of `from-the-hart-storage` (43 `.rs` files, ~4,000 LOC).
> **Goal:** Address all findings from the review while ensuring zero regression.

---

## R0 — Absolute Gating Constraint (MUST)

**Before any code change is made, the following sequence MUST be followed and verified at each step:**

1. **R0.1 — Baseline pass:** Run `cargo test --all-features`. All existing tests MUST pass. If any fail, the entire spec is blocked until they are fixed.
2. **R0.2 — Safety-net tests added:** Write and commit the prerequisite tests described in R-PRE. These tests MUST pass.
3. **R0.3 — Changes applied:** Implement all changes described in R1–R14.
4. **R0.4 — Final pass:** Run `cargo test --all-features`. All tests (existing + new) MUST pass. Run `cargo clippy --all-features`. Zero new warnings.
5. **R0.5 — Build verification:** `cargo build --all-features` and `cargo build --no-default-features --features http` and `cargo build --no-default-features --features sqs` all succeed.

**No API contract, route path, response shape, or business logic may change.** Only internal structure, types, and error handling may be modified.

---

## R-PRE — Prerequisite Safety-Net Tests

These tests MUST be written and pass **before** any production code changes in R1–R14.

### R-PRE.1 — DynamoDB Serialization Round-Trip Tests

**File:** `src/repository/utils.rs` (add `#[cfg(test)] mod tests`)

- **R-PRE.1.1** `File → dynamo_item → File` round-trip: Create a fully-populated `File` (all fields including `MediaMetadata::Image` with GPS), serialize via `file_to_dynamo_item`, deserialize via `dynamo_item_to_file`, assert equality on all fields.
- **R-PRE.1.2** `ViewLink (file) → dynamo_item → ViewLink` round-trip: A file-type `ViewLink`, serialize and deserialize, assert equality.
- **R-PRE.1.3** `ViewLink (folder) → dynamo_item → ViewLink` round-trip: A folder-type `ViewLink`, serialize and deserialize, assert equality including `is_folder = true`.
- **R-PRE.1.4** `dynamo_key_to_json` / `json_to_dynamo_key` round-trip: Create a DynamoDB key map, convert to JSON and back, assert the round-trip yields equivalent keys.

### R-PRE.2 — DTO `From` Implementation Tests

**File:** `src/handler/http/dto.rs` (add `#[cfg(test)] mod tests`)

- **R-PRE.2.1** `From<HealthStatus> for HealthResponse`: Construct a `HealthStatus`, convert, assert `data.status == "ok"` and `data.uptime` / `data.timestamp` match.
- **R-PRE.2.2** `From<ViewLink> for ViewLink` (DTO): Construct a domain `ViewLink` (file type), convert to DTO `ViewLink`, assert all fields match.
- **R-PRE.2.3** `From<ViewLink> for ViewLink` (folder type): Same as above but for folder-type ViewLink.
- **R-PRE.2.4** `From<ViewLink> for FolderData`: Construct a domain `ViewLink`, convert to `FolderData`, assert `folder_path`, `folder_name`, `parent_path`, `created_date`, `owner_id` match.
- **R-PRE.2.5** `From<File> for FileResponse`: Construct a fully-populated domain `File` with media metadata AND ensure CloudFront config is initialized. Convert to `FileResponse`, assert `file_url`, `file_id`, `file_name`, `media_metadata` all present and correct.

### R-PRE.3 — Error Conversion Tests

**File:** `src/handler/http/error.rs` (add `#[cfg(test)] mod tests`)

- **R-PRE.3.1** `NotFound` → `HttpError`: assert `status == 404`, `code == "NOT_FOUND"`.
- **R-PRE.3.2** `NotInitialized` → `HttpError`: assert `status == 503`, `code == "SERVICE_UNAVAILABLE"`.
- **R-PRE.3.3** `InvalidRequest` → `HttpError`: assert `status == 400`, `code == "BAD_REQUEST"`.
- **R-PRE.3.4** `JwtParse` → `HttpError`: assert `status == 401`, `code == "UNAUTHORIZED"`.
- **R-PRE.3.5** `Internal` / `S3` / `DynamoDb` / `Metadata` → `HttpError`: assert `status == 500`, `code == "INTERNAL_ERROR"`.

---

## R1 — Fix `From<File> for FileResponse` Panic 🔴

**Current:** `impl From<File> for FileResponse` calls `config().cloudfront.as_ref().expect(...)` — panics if CloudFront config is missing.

**Required change:**
- Either convert to `TryFrom` returning a `StorageError`, OR
- Move URL construction out of the `From` impl into the handler (`list.rs`) where a proper HTTP error can be returned.
- The DTO `FileData` should accept `file_url` as a field set by the caller, not computed inside the conversion.

**Constraint:** `FileResponse` JSON shape MUST remain identical.

---

## R2 — Document `insecure_decode` Security Boundary 🔴

**Current:** `src/utils/jwt.rs:26` uses `jsonwebtoken::dangerous::insecure_decode` with no explanation.

**Required change:**
- Add a module-level doc comment (`//!`) explaining the zero-trust architecture: the Cloudflare Worker API Gateway validates JWT signatures before forwarding requests. This service only extracts `user_id` from a pre-verified token.
- The comment MUST mention that signature verification is intentionally skipped and why that is safe.

---

## R3 — Add Context to Error Catch-All 🟡

**Current:** `From<StorageError> for HttpError` catch-all `_ =>` arm sets `details: None` and a generic message.

**Required change:**
- The `details` field SHOULD include the error variant name (e.g., `"StorageError::S3"`) and the `context` string from the error (these are already sanitized by the `thiserror` formatting). Do NOT expose raw `source` chains.
- Example: for `StorageError::S3 { context: "Failed to fetch...", source: ... }`, the `details` should be `"S3: Failed to fetch..."`.
- The original error is still logged via `tracing::error!` (keep this).

---

## R4 — Split `handler/http/dto.rs` 🟡

**Current:** Single file mixing DTOs for health, access, list, file, folder, and error responses.

**Required change:**
- Split `dto.rs` into a forwarding module with `pub mod` declarations, plus submodules in `dto/` following the 2018 edition convention (no `mod.rs`):
  - `handler/http/dto/common.rs` — `DataResponse<T>` (shared wrapper)
  - `handler/http/dto/health.rs` — `HealthData`, `HealthResponse`, `From<HealthStatus>`
  - `handler/http/dto/access.rs` — `SignedAccessData`, `SignedAccessResponse`
  - `handler/http/dto/storage.rs` — `ViewLink`, `StorageListData`, `StorageListResponse`, `FileData`, `FileResponse`, `From<File>`, `From<ViewLink>`, `FolderData`, `CreateFolderRequest`, `CreateFolderResponse`, `From<ViewLink> for FolderData`
- All existing imports throughout the codebase must be updated.
- Error DTOs (`ErrorData`, `HttpErrorResponse`) stay in `error.rs` (they already are).

---

## R5 — Split `handler/http/routes.rs` 🟡

**Current:** Route registration, OpenAPI doc generation (`create_api_docs`), and integration tests all in one file.

**Required change:**
- Extract `create_api_docs` function into `handler/http/openapi.rs`.
- Keep route configuration in `routes.rs`.
- Keep existing integration tests in `routes.rs` (they exercise route-level behavior).
- Ensure all `pub fn *_docs(op: TransformOperation)` functions remain in their respective handler files (they already are).

---

## R6 — Replace `resource_id` String Prefix + `is_folder` with `ResourceId` Enum 🟡

**Current:** `ViewLink.resource_id: Arc<str>` uses `"FOLDER#..."` prefix convention, and `is_folder: bool` duplicates this information.

**Required change:**
- Add to `src/service/models.rs`:
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
  #[serde(tag = "type", content = "value")]
  pub enum ResourceId {
      File(String),
      Folder(String),
  }
  ```
- Remove `ViewLink.is_folder` field.
- `ViewLink.resource_id` becomes `ResourceId` (not `Arc<ResourceId>`).
- Update `view_link_to_dynamo_item` in `repository/utils.rs` to branch on `ResourceId` variant instead of checking `is_folder`.
- Update `dynamo_item_to_view_link` to reconstruct the correct `ResourceId` variant.
- Update all call sites: `ViewLink::for_owner`, `ViewLink::for_owner_folder`, mock constructors, `dynamodb.rs` `create_folder`, `folder/create.rs` tests.

---

## R7 — Clean Up `service.rs` Re-Exports 🟡

**Current:** `pub use models::{File, HealthStatus};` re-exports only 2 of 6+ types.

**Required change:**
- Remove the selective `pub use`.
- Add explicit imports of `crate::service::models::File` and `crate::service::models::HealthStatus` at all call sites that were relying on the re-export.
- Alternatively, change to `pub use models::*;` to re-export all consistently. Either approach is acceptable.

---

## R8 — Mock Response Sequencing 🟢

**Current:** Mocks use `Arc<Mutex<Option<Result<T, E>>>>` — single response, consumed on first call.

**Required change:**
- Replace `Option<Result<T, E>>` with `VecDeque<Result<T, E>>` in all mock fields.
- `with_*` builder methods append to the `VecDeque`.
- Add `with_*_sequence` methods accepting `Vec<Result<T, E>>` for bulk configuration.
- Default behavior when queue is empty: return a sensible default or "mock exhausted" error.
- All existing tests must continue to pass unchanged.

---

## R9 — `Arc<str>` → `String` in Domain Models 🟢

**Current:** `File` and `ViewLink` fields use `Arc<str>`.

**Required change:**
- Change all `Arc<str>` fields in `src/service/models.rs` to `String`.
- Update all construction sites and test code accordingly.
- The DTO layer already uses `String` — conversions simplify.

---

## R10 — Remove Debug Scaffolding 🟢

**Current:** `src/handler/http/storage/list.rs` tests contain commented-out AI-generated reasoning.

**Required change:**
- Remove the commented block and replace with a concise comment.

---

## R11 — Evaluate `chrono` vs `time`/`jiff` (Decision Only) 🟢

**Not a code change.** Evaluate and document decision. Default: keep `chrono`.

---

## R12 — Evaluate Native Async Traits (Decision Only) 🟢

**Not a code change.** Evaluate and document decision. Default: keep `async-trait`.

---

## R13 — Move `START_TIME` from Global to `AppState` 🟢

**Current:** `pub static START_TIME: OnceLock<SystemTime>` in `src/utils/time.rs`.

**Required change:**
- Add `start_time: tokio::time::Instant` field to `AppState`.
- Initialize it in `AppState::new()`.
- `health::get_health_status()` takes start_time as parameter or becomes an `AppState` method.
- Remove the global `START_TIME` and `init_start_time()` calls from all binaries and tests.
- All health tests must be updated to pass `AppState` with initialized start time.

---

## R14 — Dependency Version Strategy 🟢

**Required change:**
- Change to caret ranges for trusted semver-stable crates: `serde = "1.0"`, `serde_json = "1.0"`, `tokio = "1"`, `tracing = "0.1"`, `tracing-subscriber = "0.3"`, `thiserror = "2"`, `anyhow = "1"`, `async-trait = "0.1"`, `base64 = "0.22"`, `sha2 = "0.10"`, `sha1 = "0.10"`, `urlencoding = "2"`, `schemars = "1"`.
- Keep exact pins for AWS SDK crates, `config`, `dotenvy`, `chrono`, `chrono-tz`, `aide`, `jsonwebtoken`, `rsa`, `kamadak-exif`, `imagesize`.
- Run `cargo update` and `cargo build --all-features` after changes.

---

## Constraints Summary

| Constraint | Description |
|-----------|-------------|
| C1 | No API contract changes — routes, status codes, request/response shapes unchanged |
| C2 | No business logic changes — folder creation, file listing, signed access work identically |
| C3 | R0 gating sequence enforced — baseline tests → safety-net tests → changes → final tests |
| C4 | `cargo clippy --all-features` must produce zero new warnings |
| C5 | All three build configurations must succeed: `--all-features`, `--features http`, `--features sqs` |
| C6 | R11/R12 are decision-only; no code change unless explicitly chosen |

---

## Errata from Adversarial Review

### E1 — R0.1: Baseline failure guidance
If any pre-existing test fails that is NOT caused by these changes, record the failure in `specs/storage-code-review-fixes/baseline-failures.md` and explicitly exempt it. Only failures related to code touched by R-PRE or R1-R14 block progress.

### E2 — R-PRE.1.1: Fully-populated File definition
A "fully-populated" File for round-trip testing means:
- `bucket_key`: "user123/photos/vacation.jpg"
- `bucket`: "test-bucket"
- `owner_id`: "user123"
- `file_id`: "sha256-hash-of-bucket+key"
- `file_name`: "vacation.jpg"
- `file_path`: "photos/vacation.jpg"
- `folder_prefix`: "photos/"
- `created_date`: 1700000000000
- `size_bytes`: 1048576
- `content_type`: "image/jpeg"
- `media_type`: MediaType::Image
- `media_metadata`: Some(MediaMetadata::Image(ImageMetadata { width: 1920, height: 1080, exif: Some(HashMap from [("Make".into(), "Canon".into())]), gps: Some(GpsCoordinates { latitude: 37.7749, longitude: -122.4194, altitude: Some(30.0) }) }))

### E3 — R-PRE.1.5: ResourceId enum serialization test (NEW)
Add: `ResourceId` enum JSON round-trip:
- `ResourceId::File("abc123")` → serde_json → assert JSON is `{"type":"File","value":"abc123"}` → deserialize → assert equals original.
- `ResourceId::Folder("media/photos/")` → serde_json → assert JSON is `{"type":"Folder","value":"media/photos/"}` → deserialize → assert equals original.

### E4 — R6 API contract preservation: DTO resource_id stays String
The DTO `ViewLink` (in `dto/storage.rs`) MUST keep `resource_id: String` with the `FOLDER#` prefix for folders. The internal `ResourceId` enum is for the domain model only. The DTO `From<ViewLink>` impl converts `ResourceId::Folder(path)` → `format!("FOLDER#{}", path)` for backward compatibility.

### E5 — R1: AppState must hold cloudfront_domain
Add `cloudfront_domain: Option<String>` to `AppState`. Populated from config at startup. Handler checks `state.cloudfront_domain` instead of global config. If `None`, returns 503. Decouples tests from env vars.

### E6 — R8: Mock exhaustion warning
When a mock falls through to its default, emit `tracing::warn!("Mock <name>::<method> exhausted, returning default")`. Add `set_strict_mode(true)` method that panics instead of returning default on exhaustion.

### E7 — R14: cargo update safety
After `cargo update`, run `cargo audit` (if available). If `cargo update` causes a build failure, pin the problematic crate to its previous exact version in `Cargo.toml` with a comment `# pinned due to breaking change in X.Y.Z`.
