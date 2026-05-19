# From The Hart Storage

> **Hierarchy:** Service-specific rules for storage. Extends [master AGENTS.md](../AGENTS.md).
> Rules here take precedence over both master and personal AGENTS.md.
> **Stack:** Rust + Axum + AWS Lambda. When reading the master AGENTS.md, TS/Vue sections apply to other services.
>
> Rust microservice for file storage, metadata extraction, and S3/DynamoDB operations.
> Domain glossary: [CONTEXT.md](./CONTEXT.md).

## Commands

```bash
cargo test --all-targets             # All tests (unit + doc)
cargo clippy --all-targets --all-features  # Lint
cargo fmt --check                    # Format check (CI)
RUST_LOG=info APP_ENVIRONMENT=local cargo run  # Local dev
cargo build --profile release-dev --features http  # Fast CI build for Lambda
```

## Project Structure

| Directory | What | Convention |
|-----------|------|------------|
| `src/handler/http/` | Axum route handlers + OpenAPI docs | One file per endpoint group; `openapi.rs` separate from `routes.rs` |
| `src/handler/http/dto/` | API DTOs (wire shapes) | `From<domain::Model>` impls; injected config via params, not globals |
| `src/handler/sqs/` | Lambda SQS event handler | |
| `src/service/` | Business logic | Never touches HTTP types |
| `src/service/models/` | Domain models | Split into per-type files (`file.rs`, `view_link.rs`, etc.) |
| `src/repository/` | AWS SDK wrappers (trait + impl + mock) | Shared `SdkConfig` in `aws_config.rs`; all mocks in `mock.rs` |
| `src/utils/` | Pure utility functions | |
| `src/bin/` | Lambda entry points | `bootstrap_http.rs`, `bootstrap_sqs.rs` |
| `terraform/` | Per-project IaC | Separate `prod/` and `dev/` dirs |

No `mod.rs` files anywhere — 2018 edition convention (`foo.rs` + `foo/` directory).

## Architecture

### Metadata Extraction Pipeline

The SQS worker's process of enriching a File with content-derived metadata. Triggered by S3 `ObjectCreated:*` events.

Currently handles Image files: extracts dimensions, EXIF data, and GPS coordinates. The extractor chain pattern (`MetadataExtractor` trait) allows adding support for Video, Audio, and Document types without changing existing code.

Gated by the `sqs` feature flag. Runs after S3 object creation; writes the enriched File and its ViewLinks to DynamoDB.

## Conventions

- **`required-features`** on every `[[bin]]` in Cargo.toml — prevents building without needed features.
- **Domain types:** See [CONTEXT.md](./CONTEXT.md) for domain definitions of `ResourceId`, `ViewLink`, `File`, `Folder`, `MediaType`, `MediaMetadata`.
- **`ResourceId` enum** instead of `is_folder: bool` — type-safe, impossible to have inconsistent state.
- **ResourceId Encoding:** The domain `ResourceId` enum is flattened into two DTO fields for the wire: `resource_id: String` and `is_folder: bool`. If `is_folder` is `false`, `resource_id` is the bare file SHA-256 hash. If `is_folder` is `true`, `resource_id` is `"FOLDER#{folder_path}"`. The `is_folder` boolean is the authoritative type discriminator; the `FOLDER#` prefix on the string is a DynamoDB persistence detail leaked to the wire format (see TODO #9).
- **Dependency injection** — `start_time`, `cloudfront_domain`, and `timezone` are injected via `AppState` or function params, never read from global `OnceLock`.
- **`AppState`** holds all runtime dependencies as `Arc<dyn Trait>` with feature-gated `Option` fields. The `new()` constructor is called in each entry point (`main.rs`, `bootstrap_http.rs`, `bootstrap_sqs.rs`).
- **Handler flattening** — handlers live directly under `handler/http/`, not nested under `handler/http/storage/`.
- **ViewLink** — central domain type; see CONTEXT.md for definition. `for_owner()` and `for_owner_folder()` are the canonical constructors.
- **Trailing-Slash Routing:** The wildcard `GET /storage/{user_id}/{*path}` uses path-based routing to distinguish folder listing from file retrieval:
  - Path **ends with `/`** or is **empty** → Folder Listing
  - Path **does NOT end with `/`** → File Retrieval
  - Hitting a folder path without a trailing slash → 404
  - This is a deliberate zero-guessing design: the client controls the interpretation by appending or omitting the trailing slash.
- **DTO mapping** — DTOs implement `From<domain::Model>`. Constructors needing config take it as a parameter (e.g., `FileResponse::from_file(model, cloudfront_domain)`). DTO files include `#[cfg(test)]` roundtrip tests.
- **OpenAPI docs** in dedicated `openapi.rs` — never inlined in route definitions.
- **Config** uses `dotenvy` + custom `ConfigLoadError` — no `config` crate dependency.
- **`RUST_LOG`** via `EnvFilter::try_from_default_env()` — never manually check the env var.
- **Mock `VecDeque` pattern** — mocks queue responses with `pop_front()`. Strict mode panics on exhaustion (catches unconfigured calls). See `repository/mock.rs`.
- **No `Arc<str>`** in models — use `String`. Simpler, and the sharing isn't worth it at this scale.
- **Terraform** — use block `key_schema { ... }` syntax for DynamoDB GSIs, not inline `hash_key`/`range_key` shorthand.
- Lint rules, edition, formatting, and `#[must_use]` are enforced by `Cargo.toml` and `rust-toolchain.toml` — see those files for the authoritative config.

## Boundaries

- ✅ **Always:** Run `cargo test --all-targets` + `cargo clippy` before considering work done. Co-locate unit tests with source (`#[cfg(test)]`).
- ⚠️ **Ask first:** Adding dependencies, changing DynamoDB schema, modifying OpenAPI doc structure, touching Terraform.
- 🚫 **Never:** Commit `.env` files, use `mod.rs` files, create global `OnceLock` for runtime state, silently return defaults from exhausted mocks.
