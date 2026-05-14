# Design: Storage Code Review Fixes

## Architecture Decisions

### AD-1: `From<File>` → `TryFrom<File>` for FileResponse

The `From` impl panics on missing CloudFront config. Two options:

| Option | Pros | Cons |
|--------|------|------|
| A) Convert to `TryFrom` returning `StorageError` | Idiomatic fallible conversion; caller gets error type | Adds error handling at all conversion sites |
| B) Move URL construction to handler, pass `file_url` into `FileData` | `From` stays infallible; handler owns HTTP concern | DTO no longer self-contained |

**Decision: Option B.** The handler (`list.rs`) already has access to `AppState` and can construct the URL. This keeps `From` infallible (matching Rust conventions) and makes the DTO a pure data structure with no config dependency. `FileResponse::from_file(file, cf_domain)` becomes a constructor that takes the file and the pre-resolved domain.

### AD-2: `ResourceId` Enum Design

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum ResourceId {
    File(String),
    Folder(String),
}
```

- The `Folder` variant stores the raw folder path (no `FOLDER#` prefix).
- DynamoDB serialization (`view_link_to_dynamo_item`) checks `matches!(resource_id, ResourceId::Folder(_))` instead of `is_folder`.
- `dynamo_item_to_view_link` reconstructs `ResourceId::Folder(path)` when `item_type == "FOLDER_VIEW_LINK"`, and `ResourceId::File(id)` otherwise.
- The `is_folder` convenience is preserved as a derived method: `pub fn is_folder(&self) -> bool { matches!(self.resource_id, ResourceId::Folder(_)) }` — added as an inherent method on `ViewLink` for call sites that need it (DTO conversion, etc.).

### AD-3: `AppState::start_time` — Type Choice

The current `START_TIME` is `OnceLock<SystemTime>`. Two replacement options:

| Option | Type | Uptime calc | Pros |
|--------|------|-------------|------|
| A) `std::time::Instant` | Monotonic clock | `elapsed().as_secs()` | Immune to wall clock changes |
| B) `tokio::time::Instant` | Tokio-aware monotonic | Same | Works in async context |

**Decision: Option A — `std::time::Instant`.** Health check is synchronous (`get_health_status` is not async), and `std::time::Instant` is simpler with no dependency. The `timestamp` field in `HealthStatus` continues to use `SystemTime::now()` for the wall-clock timestamp.

### AD-4: Mock Refactoring — `VecDeque` Strategy

Current pattern:
```rust
put_file_response: Arc<Mutex<Option<Result<(), StorageError>>>>,
```

New pattern:
```rust
put_file_responses: Arc<Mutex<VecDeque<Result<(), StorageError>>>>,
```

Builder methods:
```rust
// Single response — appends to deque
pub fn with_put_file_response(self, response: Result<(), StorageError>) -> Self

// Multiple responses — replaces entire deque
pub fn with_put_file_responses(self, responses: Vec<Result<(), StorageError>>) -> Self
```

When the deque is empty on a call, return a sensible default:
- For `put_file_and_view_links`: `Ok(())`
- For `find_view_links_by_folder`: `Ok((vec![], None))`
- For `get_file`: `Ok(None)`
- For `folder_exists`: `Ok(false)`
- For `create_folder`: `Ok(mock_view_link(...))`
- For `head_object`: `Err(...)` (S3 calls should always be explicitly configured)
- For `fetch_head_bytes`: `Err(...)`
- For `get_parameter`: `Err(...)`

This preserves all existing test behavior (single `with_*` call → one response, then default) while enabling sequences.

### AD-5: File Organization — DTO Split

Following the project's 2018 edition module convention (zero `mod.rs` files):

```
src/handler/http/dto.rs    # forwarding module: pub mod common; pub mod health; ...
                           # re-exports: pub use common::*, health::*, ...
src/handler/http/dto/
├── common.rs              # DataResponse<T>
├── health.rs              # HealthData, HealthResponse, From<HealthStatus>
├── access.rs              # SignedAccessData, SignedAccessResponse
└── storage.rs             # ViewLink, StorageListData, StorageListResponse,
                           # FileData, FileResponse, FolderData,
                           # CreateFolderRequest, CreateFolderResponse,
                           # From<ViewLink>, From<File>, From<ViewLink> for FolderData
```

Existing imports like `use crate::handler::http::dto::HealthResponse` remain unchanged since `dto.rs` re-exports everything. No `mod.rs` file is created — consistent with the project convention used by `src/handler/http/storage.rs` + `src/handler/http/storage/`.

### AD-6: OpenAPI Extraction

`create_api_docs` moves to `src/handler/http/openapi.rs` as a public function. `routes.rs` calls it:

```rust
// routes.rs
use super::openapi::create_api_docs;

pub fn configure_routes(state: AppState) -> Router {
    let mut api = OpenApi::default();
    // ... route registration ...
    ApiRouter::new()
        .nest("/storage", storage_router)
        .finish_api_with(&mut api, create_api_docs)
        .layer(Extension(api))
        .with_state(state)
}
```

### AD-7: Error Catch-All — What Details to Include

The `StorageError` `Display` impl (via `thiserror`) produces strings like:
- `"S3 error: Failed to fetch byte range..."` (for `StorageError::S3`)
- `"DynamoDB error: Failed to query view links"` (for `StorageError::DynamoDb`)
- `"Internal error: Unexpected state"` (for `StorageError::Internal`)

The catch-all `_ =>` arm should capture this via `err.to_string()` and include it in `details`:

```rust
_ => {
    let context = err.to_string();
    HttpError::new(
        StatusCode::INTERNAL_SERVER_ERROR,
        "INTERNAL_ERROR".to_string(),
        "An unexpected error occurred".to_string(),
        Some(context),  // sanitized by thiserror Display impl
    )
}
```

This gives operators diagnostic info without leaking raw source chains.

---

## File Change Map

### Phase 1: Safety-Net Tests (R-PRE)

| File | Action | Lines (est.) |
|------|--------|-------------|
| `src/repository/utils.rs` | Add `#[cfg(test)] mod tests` with 4 round-trip tests | +80 |
| `src/handler/http/dto.rs` | Add `#[cfg(test)] mod tests` with 5 From-impl tests | +60 |
| `src/handler/http/error.rs` | Add `#[cfg(test)] mod tests` with 5 error-conversion tests | +50 |
| `src/repository/mock.rs` | Verify S3 mock trait coverage is adequate | 0 |

### Phase 2: Production Changes (R1-R14)

| # | File(s) | Action |
|---|---------|--------|
| R1 | `dto/storage.rs`, `storage/list.rs` | Move URL construction to handler; remove `.expect()` |
| R2 | `utils/jwt.rs` | Add module-level doc comment |
| R3 | `handler/http/error.rs` | Add `err.to_string()` to catch-all `details` |
| R4 | `dto/*.rs` (5 new files) | Split `dto.rs` into submodules |
| R5 | `handler/http/routes.rs`, `handler/http/openapi.rs` (new) | Extract `create_api_docs` |
| R6 | `service/models.rs`, `repository/utils.rs`, `repository/dynamodb.rs`, `service/file/create.rs`, `service/folder/create.rs`, `handler/http/dto/storage.rs`, `handler/http/storage/list.rs`, `repository/mock.rs` | `ResourceId` enum; remove `is_folder` |
| R7 | `service.rs`, all import sites (~8 files) | Remove selective re-export; add explicit imports |
| R8 | `repository/mock.rs` | `Option` → `VecDeque`; add `with_*_sequence` methods; update ~15 test sites |
| R9 | `service/models.rs`, all construction sites (~12 files) | `Arc<str>` → `String` |
| R10 | `handler/http/storage/list.rs` | Remove commented debug scaffolding |
| R11 | (none) | Decision documented in this spec |
| R12 | (none) | Decision documented in this spec |
| R13 | `state.rs`, `main.rs`, `bin/bootstrap_http.rs`, `bin/bootstrap_sqs.rs`, `service/health.rs`, `utils/time.rs`, test files (~6) | Move `start_time` to `AppState` |
| R14 | `Cargo.toml` | Caret ranges for ~13 crates |

---

## Risk Mitigation

1. **R6 (ResourceId enum)** is the highest-risk change. The R-PRE.1 tests create a safety net by testing DynamoDB serialization round-trips BEFORE the enum change.
2. **R9 (Arc<str> → String)** is broad but mechanical — the compiler catches all mismatches.
3. **R13 (START_TIME)** is the only change affecting runtime behavior. Health endpoint tests in `health.rs` and `routes.rs` cover this.
4. **R1 (From panic)** is fixed by removing the panic path — R-PRE.2.5 tests the fixed conversion.

## Design Decisions Log

| ID | Decision | Rationale |
|----|----------|-----------|
| AD-1 | Option B: move URL to handler | Keeps `From` infallible; DTO stays pure data |
| AD-2 | `ResourceId` enum with serde tag | Type-safe; no string prefix convention |
| AD-3 | `std::time::Instant` for start_time | Same behavior; no extra dependency |
| AD-4 | `VecDeque` with sensible defaults | Preserves all existing single-response tests |
| AD-5 | DTO submodules: `dto.rs` + `dto/` directory | Backward-compatible imports; no `mod.rs` |
| AD-6 | `openapi.rs` as sibling to `routes.rs` | Same module; no import changes |
| AD-7 | `err.to_string()` in catch-all details | Diagnostic value without leaking internals |

---

## Errata from Adversarial Review

### AD-1 (Revised) — `From<File>` → Constructor with AppState

The handler must not access global config. Instead:
- `AppState` gains `cloudfront_domain: Option<String>`, populated from config at startup.
- Handler in `list.rs` checks `state.cloudfront_domain`. If `None`, returns 503.
- `FileResponse::from_file(file, &domain)` constructs the URL from the provided domain string.
- Tests inject a domain string directly, making them hermetic (no env vars needed).

### AD-2 (Revised) — `ResourceId` Enum + DTO Backward Compatibility

The `ResourceId` enum lives in the **domain model only**. The DTO layer MUST NOT expose it.

DTO `ViewLink` keeps:
```rust
pub resource_id: String,  // "FOLDER#path/" for folders, file_id for files
pub is_folder: bool,      // derived from ResourceId variant
```

The `From<service::models::ViewLink> for dto::ViewLink` impl converts:
- `ResourceId::File(id)` → `resource_id: id`, `is_folder: false`
- `ResourceId::Folder(path)` → `resource_id: format!("FOLDER#{}", path)`, `is_folder: true`

This preserves the existing API contract (C1) while gaining type safety in the domain layer.

### AD-4 (Revised) — Mock Exhaustion Handling

When deque is empty:
- Emit `tracing::warn!("Mock <Struct>::<method> exhausted, returning default")`
- Return sensible default as specified
- Add `set_strict_mode(bool)` — when true, panics with `"Mock <name>::<method> called but no responses configured"` instead of returning default. Tests that want fail-fast can enable this.

### AD-7 (Revised) — Error Catch-All + Correlation

In addition to `err.to_string()` in `details`, emit a correlation ID: `tracing::error!(correlation_id = %uuid, error = ?err, ...)` and include `correlation_id` in the `details` string. This connects logs to responses for production debugging.

### AD-8 (New) — AppState CloudFront Domain

```rust
pub struct AppState {
    // ... existing fields ...
    pub cloudfront_domain: Option<String>,
}
```

Populated in `main.rs` / bootstrap files from `config().cloudfront.as_ref().map(|c| c.domain.clone())`. The `cloudfront_signer` field (already in AppState) is separate — the domain is needed for URL construction even when signing is not active.
