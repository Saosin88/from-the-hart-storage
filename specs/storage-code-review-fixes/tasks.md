# Tasks: Storage Code Review Fixes

> **Execution order is mandatory.** Each phase gates the next.

---

## Phase 0 — Baseline Verification

### T0.1 — Verify all existing tests pass
- **Command:** `cargo test --all-features`
- **Expected:** All tests pass. If any fail, STOP and fix before proceeding.
- **Output:** Record the test count for comparison after changes.

### T0.2 — Record current clippy state
- **Command:** `cargo clippy --all-features 2>&1`
- **Expected:** No warnings. If warnings exist, record them for comparison (no new warnings allowed in final check).

### T0.3 — Verify all build configurations
- **Command:** `cargo build --all-features`
- **Command:** `cargo build --no-default-features --features http`
- **Command:** `cargo build --no-default-features --features sqs`
- **Expected:** All three succeed.

---

## Phase 1 — Safety-Net Tests (R-PRE)

> **Goal:** Add tests to currently untested code paths BEFORE making changes.
> **Gate:** All new tests MUST pass before proceeding to Phase 2.

### T1.1 — Add DynamoDB serialization round-trip tests
- **File:** `src/repository/utils.rs`
- **Location:** End of file, new `#[cfg(test)] mod tests { ... }` block
- **Tests to add:**
  - `test_file_to_dynamo_and_back` — Create a `File` with all fields populated (bucket_key, bucket, owner_id, file_id, file_name, file_path, folder_prefix, created_date, size_bytes, content_type, media_type: Image, media_metadata: Some(ImageMetadata with width/height/exif/gps)). Call `file_to_dynamo_item`, then `dynamo_item_to_file`. Assert all fields equal.
  - `test_view_link_file_to_dynamo_and_back` — Create a file-type `ViewLink` (is_folder: false, resource_id: "some-file-id", ...). Call `view_link_to_dynamo_item`, then `dynamo_item_to_view_link`. Assert equal.
  - `test_view_link_folder_to_dynamo_and_back` — Create a folder-type `ViewLink` (is_folder: true, resource_id: "FOLDER#some/path/", ...). Call round-trip. Assert equal including is_folder.
  - `test_dynamo_key_json_roundtrip` — Create a `HashMap<String, AttributeValue>` with S, N, Bool variants. Call `dynamo_key_to_json`, then `json_to_dynamo_key`. Assert the round-tripped map has the same keys and equivalent values.
- **Verify:** `cargo test --all-features` — all existing + new tests pass.

### T1.2 — Add DTO From-impl tests
- **File:** `src/handler/http/dto.rs`
- **Location:** End of file, new `#[cfg(test)] mod tests { ... }` block
- **Tests to add:**
  - `test_health_status_to_response` — Create `HealthStatus { uptime: 42, timestamp: 123456 }`. Convert via `HealthResponse::from(status)`. Assert `data.status == "ok"`, `data.uptime == 42`, `data.timestamp == 123456`.
  - `test_view_link_file_to_dto` — Create domain `ViewLink` (file type, all fields populated). Convert to DTO `ViewLink`. Assert field-by-field equality.
  - `test_view_link_folder_to_dto` — Same as above but folder type. Assert `is_folder == true` and all fields match.
  - `test_view_link_to_folder_data` — Create domain `ViewLink` (folder type). Convert via `FolderData::from(view_link)`. Assert `folder_path`, `folder_name`, `parent_path`, `created_date`, `owner_id` match.
  - `test_file_to_file_response` — **Requires CloudFront config to be initialized.** Use `config::init_config()` with env vars or mock config. Create a full `File` with media_metadata. Convert via `FileResponse::from(file)`. Assert `file_url` starts with `https://`, `file_id`, `file_name`, `media_metadata` present. **This is the safety net for R1.**
- **Verify:** `cargo test --all-features` — all tests pass.

### T1.3 — Add error conversion tests
- **File:** `src/handler/http/error.rs`
- **Location:** End of file, new `#[cfg(test)] mod tests { ... }` block
- **Tests to add:**
  - `test_not_found_to_http_error` — `StorageError::NotFound { context: "x".into() }` → `HttpError`. Assert `status == 404`, `response.error.code == "NOT_FOUND"`.
  - `test_not_initialized_to_http_error` — `StorageError::NotInitialized { ... }` → `HttpError`. Assert `status == 503`, code `"SERVICE_UNAVAILABLE"`.
  - `test_invalid_request_to_http_error` — `StorageError::InvalidRequest { ... }` → `HttpError`. Assert `status == 400`, code `"BAD_REQUEST"`.
  - `test_jwt_parse_to_http_error` — `StorageError::JwtParse { ... }` → `HttpError`. Assert `status == 401`, code `"UNAUTHORIZED"`.
  - `test_internal_errors_to_http_error` — Test `StorageError::Internal`, `StorageError::S3`, `StorageError::DynamoDb`, `StorageError::Metadata`. Each → `HttpError`. Assert `status == 500`, code `"INTERNAL_ERROR"`.
- **Verify:** `cargo test --all-features` — all tests pass.

### T1.4 — Verify mock S3 trait coverage
- **File:** `src/repository/mock.rs`
- **Check:** `MockS3Repository` implements `S3RepositoryTrait`. The trait has 2 methods; both are implemented. No additional tests needed for the mock itself — it's exercised by `service/file/create.rs` tests.
- **Action:** None required. Confirm this in task notes.

---

## Phase 2 — Production Changes (R1–R14)

> **Gate:** All Phase 1 tests pass before starting any Phase 2 task.
> **Order:** Tasks are ordered to minimize conflicts. R4+R5 (file splits) come first to avoid merge conflicts on subsequent changes.

### T2.1 — R4: Split `handler/http/dto.rs`
- **Keep:** `src/handler/http/dto.rs` — convert to forwarding module with `pub mod common; pub mod health; pub mod access; pub mod storage;` and re-exports (`pub use common::*; pub use health::*; ...`). This follows the project convention (e.g., `storage.rs` + `storage/` directory).
- **Create:** `src/handler/http/dto/common.rs` — move `DataResponse<T>` here
- **Create:** `src/handler/http/dto/health.rs` — move `HealthData`, `HealthResponse`, `From<HealthStatus>` here
- **Create:** `src/handler/http/dto/access.rs` — move `SignedAccessData`, `SignedAccessResponse` here
- **Create:** `src/handler/http/dto/storage.rs` — move `ViewLink` (DTO), `StorageListData`, `StorageListResponse`, `FileData`, `FileResponse`, `From<File>`, `From<ViewLink>`, `FolderData`, `CreateFolderRequest`, `CreateFolderResponse`, `From<ViewLink> for FolderData` here
- **Note:** `src/handler/http.rs` already has `pub mod dto;` — no change needed (Rust resolves `dto.rs` + `dto/` directory automatically).
- **No `mod.rs` file is created** — consistent with the project's 2018 edition convention.
- **Move tests:** Each submodule file gets its relevant tests from the original `dto.rs` test block
- **Verify:** `cargo build --all-features` + `cargo test --all-features`

### T2.2 — R5: Extract OpenAPI docs from routes.rs
- **Create:** `src/handler/http/openapi.rs` — move `create_api_docs` function here, plus all its `use` imports
- **Update:** `src/handler/http/routes.rs` — import and call `create_api_docs` from `openapi.rs`
- **Update:** `src/handler/http.rs` — add `pub mod openapi;`
- **Verify:** `cargo build --all-features` + `cargo test --all-features`

### T2.3 — R10: Remove debug scaffolding
- **File:** `src/handler/http/storage/list.rs`
- **Change:** In the `#[cfg(test)] mod tests` block, find the `test_handle_file_request_get_file_success` test. Remove the commented-out AI scaffolding block (lines ~180-188 starting with `// Wait, MockDynamoDbRepository...`). Replace with: `// MockDynamoDbRepository returns Ok(None) by default for get_file — testing NOT_FOUND path.`
- **Verify:** `cargo test --all-features`

### T2.4 — R7: Clean up service.rs re-exports
- **File:** `src/service.rs`
- **Change:** Remove line `pub use models::{File, HealthStatus};`
- **Update imports in all files that use `crate::service::File` or `crate::service::HealthStatus`:**
  - `src/handler/http/dto/storage.rs` (was `dto.rs`) — change `crate::service::models::File` → already imports this path (verify)
  - `src/handler/http/error.rs` — change any imports
  - `src/handler/http/storage/list.rs` — verify imports
  - `src/handler/sqs/worker.rs` — change `crate::service::File` → `crate::service::models::File`
  - `src/repository.rs` — change `use crate::service::{..., File}` → `use crate::service::models::{..., File}`
  - `src/repository/utils.rs` — verify imports
  - `src/repository/mock.rs` — verify imports
  - `src/repository/dynamodb.rs` — verify imports
  - `src/service/file/create.rs` — verify imports
  - `src/service/metadata.rs` — verify imports
  - `src/service/metadata/image.rs` — verify imports
  - `src/service/metadata/extractor.rs` — verify imports
  - `src/service/folder/create.rs` — verify imports
  - `src/service/file/get.rs` — verify imports
  - `src/service/file/list.rs` — verify imports
  - `src/service/file.rs` — already has `pub use models::File;`? No — verify. The `service/file.rs` currently has `#[cfg(feature = "sqs")] pub mod create; pub mod get; pub mod list; pub mod utils;` — no re-exports.
- **Verify:** `cargo build --all-features` + `cargo test --all-features`

### T2.5 — R1: Fix From<File> for FileResponse panic
- **File:** `src/handler/http/dto/storage.rs`
- **Change:** Replace `impl From<File> for FileResponse` with a constructor function:
  ```rust
  impl FileResponse {
      pub fn from_file(file: File, cloudfront_domain: &str) -> Self {
          let file_url = format!("https://{}/{}", cloudfront_domain, file.bucket_key);
          // ... rest same as current From impl but without .expect()
      }
  }
  ```
- **File:** `src/handler/http/storage/list.rs`
- **Change:** In `handle_file_request`, before calling the conversion, extract `cloudfront_domain` from config (matching the existing pattern in `access.rs`). If config is missing, return `HttpError` 503. Call `FileResponse::from_file(file, &domain)`.
- **Update test:** `src/handler/http/dto/storage.rs` — update `test_file_to_file_response` to use the new constructor.
- **Verify:** `cargo build --all-features` + `cargo test --all-features`

### T2.6 — R2: Document insecure_decode security boundary
- **File:** `src/utils/jwt.rs`
- **Change:** Add module-level doc comment at top of file:
  ```rust
  //! JWT utility for extracting user identity from pre-verified tokens.
  //!
  //! **Security Note:** This service uses `jsonwebtoken::dangerous::insecure_decode`
  //! to extract the `user_id` claim WITHOUT verifying the JWT signature. This is
  //! intentional and safe because:
  //!
  //! 1. The Cloudflare Worker API Gateway (the sole entry point) validates all JWT
  //!    signatures BEFORE forwarding requests to this service.
  //! 2. This service is not publicly accessible — all traffic flows through the gateway.
  //! 3. Signature verification here would be redundant and add latency.
  //!
  //! If this service is ever exposed directly, signature verification MUST be added.
  ```
- **Verify:** `cargo build --all-features` (doc comments don't affect compilation)

### T2.7 — R3: Add context to error catch-all
- **File:** `src/handler/http/error.rs`
- **Change:** In the `From<StorageError> for HttpError` impl, modify the catch-all `_ =>` arm:
  ```rust
  _ => {
      let context = err.to_string(); // sanitized by thiserror Display impl
      HttpError::new(
          StatusCode::INTERNAL_SERVER_ERROR,
          "INTERNAL_ERROR".to_string(),
          "An unexpected error occurred".to_string(),
          Some(context),
      )
  }
  ```
- **Update test:** `src/handler/http/error.rs` — update `test_internal_errors_to_http_error` to assert `details` is `Some(...)` and non-empty.
- **Verify:** `cargo build --all-features` + `cargo test --all-features`

### T2.8 — R9: Arc<str> → String in domain models
- **File:** `src/service/models.rs`
- **Change:** Replace all `Arc<str>` with `String` in `File` and `ViewLink` structs.
- **Update all construction sites** (search for `.into()` calls that produce `Arc<str>`):
  - `src/service/models.rs` — `File::new()`: change `bucket_key: bucket_key.into()` → `bucket_key: bucket_key` (already String), etc. Actually, `.into()` on `String` → `Arc<str>` needs to change to just using the String directly.
  - `src/service/models.rs` — `ViewLink::for_owner()`: change `.into()` calls to direct assignment.
  - `src/service/models.rs` — `ViewLink::for_owner_folder()`: same.
  - `src/repository/dynamodb.rs` — `create_folder()`: ViewLink construction uses `.into()` — change to String.
  - `src/handler/http/dto/storage.rs` — `From<ViewLink>` impl: change `.to_string()` calls (they already produce String, so may need to remove `Arc::from` wrappers).
  - `src/handler/http/storage/list.rs` — line ~143: `&*file.owner_id` → `&file.owner_id`
  - All test code constructing `File` or `ViewLink` with `.into()` calls.
- **Pattern:** Where code does `field: some_string.into()` to produce `Arc<str>`, change to `field: some_string` (if some_string is already String) or `field: some_string.to_string()`.
- **Verify:** `cargo build --all-features` + `cargo test --all-features`

### T2.9 — R6: ResourceId enum (replaces is_folder + string prefix)
- **WARNING:** This is the highest-risk change. R-PRE.1 tests MUST be passing before this task.
- **File:** `src/service/models.rs`
  - Add `ResourceId` enum (see AD-2 in design doc)
  - Replace `ViewLink.resource_id: Arc<str>` → `ViewLink.resource_id: ResourceId`
  - Remove `ViewLink.is_folder: bool`
  - Add `impl ViewLink { pub fn is_folder(&self) -> bool { matches!(self.resource_id, ResourceId::Folder(_)) } }`
- **File:** `src/repository/utils.rs`
  - `view_link_to_dynamo_item`: Replace `if view_link.is_folder { ... }` with `match &view_link.resource_id { ResourceId::Folder(path) => { ... }, ResourceId::File(id) => { ... } }`. For Folder, extract path via `path.as_str()`. For File, use `id.as_str()`.
  - `dynamo_item_to_view_link`: Construct `ResourceId::Folder(path)` when `item_type == "FOLDER_VIEW_LINK"`, `ResourceId::File(id)` otherwise. The `is_folder` convenience is now derived.
- **File:** `src/service/models.rs` — `ViewLink::for_owner`: Change `resource_id: file.file_id.clone()` to `resource_id: ResourceId::File(file.file_id.to_string())`. Change `is_folder: false` → removed.
- **File:** `src/service/models.rs` — `ViewLink::for_owner_folder`: Change `resource_id: full_folder_path.into()` to `resource_id: ResourceId::Folder(full_folder_path.to_string())`. Change `is_folder: true` → removed.
- **File:** `src/repository/dynamodb.rs` — `create_folder()`: Change `resource_id: format!("FOLDER#{}", folder_path).into()` to `resource_id: ResourceId::Folder(folder_path.to_string())`.
- **File:** `src/handler/http/dto/storage.rs` — `From<ViewLink>`: Change `.resource_id.to_string()` to access the inner string via match or a helper method. Add `pub fn resource_id_str(&self) -> &str` to `ViewLink` for convenience.
- **File:** `src/handler/http/dto/storage.rs` — `From<ViewLink> for FolderData`: Change `model.resource_id.to_string()` to `model.resource_id_str().to_string()`.
- **File:** `src/handler/sqs/worker.rs` — line where `view_link.resource_id` is used (verify — may be in log format strings).
- **File:** `src/repository/mock.rs` — Update all ViewLink constructions in tests and defaults:
  - `create_folder` expects `resource_id: "FOLDER#media/photos/".into()` → `resource_id: ResourceId::Folder("media/photos/".to_string())`
  - All test fixtures constructing ViewLink.
- **File:** `src/service/folder/create.rs` — `mock_view_link()`: Update resource_id construction.
- **File:** `src/service/file/create.rs` — Tests: Update ViewLink assertions.
- **File:** `src/handler/http/storage/folder.rs` — `mock_view_link()`: Update.
- **File:** `src/repository/dynamodb.rs` — Tests: Update ViewLink constructions.
- **Verify:** `cargo build --all-features` + `cargo test --all-features`. Pay special attention to DynamoDB round-trip tests (R-PRE.1).

### T2.10 — R8: Mock response sequencing
- **File:** `src/repository/mock.rs`
- **Change all mock fields:** Replace `Arc<Mutex<Option<Result<T, E>>>>` with `Arc<Mutex<VecDeque<Result<T, E>>>>`:
  - `MockS3Repository`: `head_object_response`, `fetch_head_bytes_response`
  - `MockDynamoDbRepository`: `put_file_response`, `find_view_links_response`, `get_file_response`, `folder_exists_response`, `create_folder_response`
  - `MockSsmRepository`: `get_parameter_response`
- **Change builder methods:** `with_*` now `push_back` onto the deque.
- **Add sequence methods:** `with_*_responses(self, responses: Vec<Result<T, E>>)` — replaces deque.
- **Change trait impl methods:** `lock.take()` → `lock.pop_front()`. If `None`, return sensible default (see AD-4).
- **Update `Default` impls:** Initialize with empty `VecDeque`.
- **Verify:** `cargo build --all-features` + `cargo test --all-features`. All existing tests must still pass (they set one response, which is popped, then fall through to default on subsequent calls).

### T2.11 — R13: Move START_TIME from global to AppState
- **File:** `src/state.rs`
  - Add field: `pub start_time: std::time::Instant`
  - Initialize in `AppState::new()`: `start_time: std::time::Instant::now()`
- **File:** `src/service/health.rs`
  - Change `get_health_status()` to accept `start_time: std::time::Instant`:
    ```rust
    pub fn get_health_status(start_time: std::time::Instant) -> Result<HealthStatus, StorageError> {
        let uptime = start_time.elapsed().as_secs();
        let timestamp = time::current_timestamp_millis().map_err(|e| StorageError::Time { ... })?;
        Ok(HealthStatus { uptime, timestamp })
    }
    ```
  - Remove `use crate::utils::time::START_TIME`.
- **File:** `src/handler/http/storage/health.rs`
  - Update `health()` handler: extract `state.start_time` and pass to `get_health_status()`.
  - Update tests to pass `std::time::Instant::now()`.
- **File:** `src/main.rs`
  - Remove `time::init_start_time()` call.
- **File:** `src/bin/bootstrap_http.rs`
  - Remove `time::init_start_time()` call.
- **File:** `src/bin/bootstrap_sqs.rs`
  - Remove `time::init_start_time()` call.
- **File:** `src/utils/time.rs`
  - Remove `pub static START_TIME: OnceLock<SystemTime>`.
  - Remove `pub fn init_start_time()`.
  - Keep `current_timestamp_millis()`, `now_as_unix_millis()`, `uptime_in_secs()` (may be used elsewhere — check. If only health used `uptime_in_secs`, can remove it too).
- **Update all tests that call `init_start_time()`:**
  - `src/handler/http/storage/health.rs` — tests: remove `init_start_time()` calls
  - `src/handler/http/routes.rs` — `test_health_endpoint_*` tests: remove `init_start_time()` calls; pass `AppState` with initialized `Instant`
- **Verify:** `cargo build --all-features` + `cargo test --all-features`

### T2.12 — R14: Dependency version strategy
- **File:** `Cargo.toml`
- **Change exact pins to caret ranges for these crates:**
  - `serde = "1.0"` (was `"1.0.228"`)
  - `serde_json = "1.0"` (was `"1.0.145"`)
  - `tokio = "1"` (was `"1.48.0"`)
  - `tracing = "0.1"` (was `"0.1.43"`)
  - `tracing-subscriber = "0.3"` (was `"0.3.22"`)
  - `tracing-error = "0.2"` (was `"0.2.1"`)
  - `thiserror = "2"` (was `"2.0.17"`)
  - `anyhow = "1"` (was `"1.0.100"`)
  - `async-trait = "0.1"` (was `"0.1.89"`)
  - `base64 = "0.22"` (was `"0.22.1"`)
  - `sha2 = "0.10"` (was `"0.10.9"`)
  - `sha1 = "0.10"` (was `"0.10"`)
  - `urlencoding = "2"` (was `"2.1.3"`)
  - `schemars = "1"` (was `"1.1.0"`)
  - `aws-config = "1"` (was `"1.8.11"`)
  - `aws-sdk-s3 = "1"` (was `"1.115.0"`)
  - `aws-sdk-dynamodb = "1"` (was `"1.98.0"`)
  - `aws-sdk-ssm = "1"` (was `"1.98.0"`)
- **Keep exact pins for:**
  - `config = "0.15.19"`, `dotenvy = "0.15.7"`
  - `chrono = "0.4.42"`, `chrono-tz = "0.10.4"`
  - `aide = "0.16.0-alpha.1"`
  - `jsonwebtoken = "10.2.0"`, `rsa = "0.9"`
  - `kamadak-exif = "0.6.1"`, `imagesize = "0.14.0"`
  - `lambda_http = "1.0.1"`, `lambda_runtime = "1.0.1"`, `aws_lambda_events = "1.0.1"`
  - `axum = "0.8.7"`, `tower = "0.5.2"`, `tower-http = "0.6.7"`
  - `axum-test = "18.3.0"`
- **Command:** `cargo update`
- **Verify:** `cargo build --all-features` + `cargo test --all-features`. If `cargo update` pulls breaking changes, revert and pin the problematic crate.

### T2.13 — R11: Document chrono decision
- **File:** `specs/storage-code-review-fixes/decisions.md` (create if not exists)
- **Add:** Decision entry: "Keep chrono v0.4.42. Migration to `time` or `jiff` not justified at this time — `chrono` is stable, well-tested, and the codebase's timezone handling (EXIF date parsing with chrono_tz) works correctly. Re-evaluate if chrono releases a breaking v0.5."
- **No code changes.**

### T2.14 — R12: Document async-trait decision
- **File:** `specs/storage-code-review-fixes/decisions.md`
- **Add:** Decision entry: "Keep `async-trait` v0.1. Native async traits are stable (Rust 1.75+) and available (project uses 1.91), but `async_trait` provides better ergonomics for `Send` bounds on returned futures. The proc-macro overhead is minimal. Re-evaluate when the ecosystem consensus shifts."
- **No code changes.**

---

## Phase 3 — Final Verification

### T3.1 — Run all tests
- **Command:** `cargo test --all-features`
- **Expected:** All tests pass. Count should be ≥ the Phase 0 baseline count (original tests + ~14 new safety-net tests).
- **If any test fails:** Debug and fix. Do not proceed until all pass.

### T3.2 — Run clippy
- **Command:** `cargo clippy --all-features`
- **Expected:** Zero new warnings compared to Phase 0 baseline. Fix any new warnings.

### T3.3 — Verify all build configurations
- **Command:** `cargo build --all-features`
- **Command:** `cargo build --no-default-features --features http`
- **Command:** `cargo build --no-default-features --features sqs`
- **Expected:** All three succeed.

### T3.4 — Run fmt check
- **Command:** `cargo fmt -- --check`
- **Expected:** No formatting differences. If any, run `cargo fmt`.

---

## Task Dependency Graph

```
T0.1─┬─T0.2─┬─T0.3          (baseline)
     │       │
     ▼       ▼
T1.1 T1.2 T1.3 T1.4          (safety-net tests — can run in parallel)
     │       │
     └───┬───┘
         ▼
   T2.1 ─► T2.2              (file splits first — no conflicts)
         ▼
   T2.3 ─► T2.4              (low-risk cleanups)
         ▼
   T2.5 ─► T2.6 ─► T2.7     (error handling improvements)
         ▼
   T2.8 ─► T2.9              (Arc<str> first, then ResourceId — Arc<str> is prerequisite for clean ResourceId)
         ▼
   T2.10 ─► T2.11            (mock refactor, then START_TIME)
         ▼
   T2.12 ─► T2.13 ─► T2.14   (Cargo.toml, then decisions)
         ▼
   T3.1 ─► T3.2 ─► T3.3 ─► T3.4  (final verification)
```

---

## Estimated Effort

| Phase | Tasks | Est. Time |
|-------|-------|-----------|
| Phase 0 | 3 verification commands | 5 min |
| Phase 1 | 4 test-writing tasks (~190 lines) | 30 min |
| Phase 2.1-2.5 | File splits + R1 fix | 30 min |
| Phase 2.6-2.7 | Documentation + error improvement | 10 min |
| Phase 2.8-2.9 | Arc<str> + ResourceId enum | 45 min |
| Phase 2.10-2.11 | Mock refactor + START_TIME move | 30 min |
| Phase 2.12-2.14 | Cargo.toml + decisions | 15 min |
| Phase 3 | 4 verification commands | 10 min |
| **Total** | | **~2.75 hours** |

---

## Errata from Adversarial Review

### T0.1.a — Baseline failure recording (NEW)
- If any test fails in T0.1, record the failure in `specs/storage-code-review-fixes/baseline-failures.md` with the failing test name, error message, and a note that it is pre-existing.
- Only fix failures caused by subsequent changes. Pre-existing failures are exempt from the gating constraint.

### T0.2.a — Clippy baseline diff (NEW)
- Save baseline: `cargo clippy --all-features 2>&1 | tee specs/storage-code-review-fixes/clippy_baseline.txt`
- In T3.2, diff against this file. Only new warnings count as violations.

### T1.1.a — ResourceId enum serialization test (NEW, R-PRE.1.5)
- **File:** `src/service/models.rs` (in existing `#[cfg(test)]` block, or new test module)
- **Test:** `test_resource_id_enum_serde_roundtrip`
  - Serialize `ResourceId::File("abc123".into())` to JSON. Assert output is `{"type":"File","value":"abc123"}`. Deserialize back. Assert equality.
  - Serialize `ResourceId::Folder("media/photos/".into())` to JSON. Assert output is `{"type":"Folder","value":"media/photos/"}`. Deserialize back. Assert equality.

### T1.2.a — R-PRE.2.5 hermetic test setup (REVISED)
- The `test_file_to_file_response` test must NOT depend on env vars or global config.
- Instead, call `FileResponse::from_file(file, "d123.cloudfront.net")` directly with a hard-coded domain.
- This requires R1 to be implemented differently (see T2.5.a).
- **Action during Phase 1:** Because T1.2 runs BEFORE Phase 2 changes, this test will initially use the existing `From<File>` impl (which accesses global config).
  - **Workaround for T1.2:** Set env var `APP_CLOUDFRONT_DOMAIN=d123.cloudfront.net` and `APP_CLOUDFRONT_KEY_PAIR_ID=K123` before calling `config::init_config()` in the test, OR mark this test with `#[ignore]` in Phase 1 and un-ignore it after T2.5.
  - **Decision:** Use the env var approach for Phase 1. After T2.5 refactors the conversion, update the test to use the new constructor directly without env vars.

### T2.4.a — Use cargo check for import fixes (REVISED)
- Instead of pre-enumerating all files, remove the re-export in `service.rs`, then run `cargo check --all-features 2>&1`. Fix every "unresolved import" error that appears. This is more reliable than a manual file list.

### T2.5.a — R1 with AppState cloudfront_domain (REVISED)
- **File:** `src/state.rs` — add `pub cloudfront_domain: Option<String>` to `AppState`.
- **File:** `src/main.rs` — populate from `config().cloudfront.map(|c| c.domain.clone())`.
- **File:** `src/bin/bootstrap_http.rs` — same.
- **File:** `src/handler/http/dto/storage.rs` — change `FileResponse`:
  ```rust
  impl FileResponse {
      pub fn from_file(file: File, cloudfront_domain: &str) -> Self { ... }
  }
  ```
- **File:** `src/handler/http/storage/list.rs` — in `handle_file_request`:
  ```rust
  let domain = match &state.cloudfront_domain {
      Some(d) => d.clone(),
      None => return HttpError::from(StorageError::NotInitialized {
          context: "CloudFront domain not configured".into()
      }).into_response(),
  };
  let response = FileResponse::from_file(file, &domain);
  ```
- **Update test:** T1.2.a test now calls `FileResponse::from_file(file, "d123.cloudfront.net")`.

### T2.6 — R2 JWT docs location (ADDED)
- Also add a one-line comment at the `insecure_decode` call site: `// SAFETY: token signature verified by Cloudflare Worker API Gateway before reaching this service`

### T2.9 — R6 DTO backward compatibility (REVISED)
- In addition to the domain model changes, the DTO `ViewLink` in `dto/storage.rs` MUST keep:
  - `resource_id: String` (with `FOLDER#` prefix for folders)
  - `is_folder: bool`
- The `From<domain::ViewLink> for dto::ViewLink` impl converts:
  - `ResourceId::File(id)` → `resource_id: id.clone()`, `is_folder: false`
  - `ResourceId::Folder(path)` → `resource_id: format!("FOLDER#{}", path)`, `is_folder: true`
- Add a convenience method `ViewLink::resource_id_str(&self) -> &str` that returns the inner string for both variants.

### T2.10.a — R8 mock exhaustion warning (REVISED)
- When deque is empty and default is returned, emit: `tracing::warn!("Mock <name>::<method> exhausted, returning default")`
- Add method: `pub fn set_strict_mode(&self, strict: bool)` — sets a `strict: Arc<AtomicBool>` field. When `true`, panics on exhaustion instead of returning default.

### T2.12.a — R14 cargo audit (REVISED)
- After `cargo update`: run `cargo audit` if installed. If any new vulnerabilities are found, pin the affected crate to the previous version and open a separate issue.
- If `cargo update` causes a build failure, pin the problematic crate to its previous exact version in `Cargo.toml` with comment `# pinned: breaking change in X.Y.Z`.

### T3.2.a — Clippy diff (REVISED)
- Run: `cargo clippy --all-features 2>&1 | diff - specs/storage-code-review-fixes/clippy_baseline.txt`
- Only lines prefixed with `>` (additions) in the diff output are violations. Lines prefixed with `<` are removed warnings (acceptable).
