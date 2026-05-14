# Requirements: Storage Upgrade Test Coverage

## Summary

Add comprehensive test coverage to all currently untested business-logic source files in the `from-the-hart-storage` Rust microservice, establish a `cargo test` gate in CI, upgrade Rust from 1.91 to the latest stable (1.95+) with aligned Dockerfiles, and upgrade all Cargo dependencies from their pinned `=` versions to the latest stable semver-compatible versions — without breaking any existing functionality. The OpenAPI specification must be verified before and after changes via golden-file comparison. All existing 76 tests must pass at baseline before any work begins.

## User Stories

1. **As a developer,** I want every source file with business logic to have test coverage, so that dependency upgrades can be performed safely without fear of silent regressions.

2. **As a developer,** I want `cargo test` to run in CI before Docker image build, so that broken code never reaches deployment and P10 compliance is achieved.

3. **As a developer,** I want Rust and all Cargo dependencies upgraded to the latest stable versions, so that the project benefits from security patches, performance improvements, and ecosystem compatibility.

4. **As a developer,** I want the OpenAPI specification verified before and after all changes, so that the `aide` crate upgrade (from alpha to stable) does not silently alter the API contract.

5. **As a developer,** I want local Rust toolchain and Dockerfile Rust versions to align on the same latest stable version, so that local development and CI produce identical results.

## Acceptance Criteria

### Baseline (must complete first)

- **AC-B1:** All 76 existing tests pass with zero failures on the current codebase and current dependencies.
- **AC-B2:** `cargo test --all-features` passes.
- **AC-B3:** `cargo test --no-default-features --features http` passes.
- **AC-B4:** `cargo test --no-default-features --features sqs` passes.

### Test Coverage

- **AC-T1:** Every source file listed below has a `#[cfg(test)] mod tests` block with at least one test function. Module re-exports (`pub mod` only files) are exempt.

  | File | Minimum test surface |
  |------|---------------------|
  | `src/config.rs` | `AppConfig::load()` with valid env, missing required var, invalid port |
  | `src/error.rs` | Each variant's `Display` / `Error` impl |
  | `src/state.rs` | `AppState::new()` with all feature combinations |
  | `src/logging.rs` | `init_logging()` (verify it doesn't panic) |
  | `src/utils/string.rs` | `url_decode` (valid, invalid, empty), `clean_value` (all branches), `sha256_hash` |
  | `src/utils/time.rs` | `current_timestamp_millis`, `parse_media_datetime_with_offset`, `parse_naive_datetime`, `now_as_unix_millis`, `get_timezone` |
  | `src/service/metadata/image.rs` | `can_handle`, `parse_exif`, `extract_creation_date`, `extract_gps_coordinates`, `extract_and_add_to_file` |
  | `src/service/health.rs` | `get_health_status` success and error paths |
  | `src/handler/http/openapi.rs` | `create_api_docs` output shape verification |
  | `src/handler/http/dto/access.rs` | Serialization/deserialization of DTOs |
  | `src/handler/http/dto/common.rs` | Serialization/deserialization of common DTOs |

- **AC-T2:** Every `pub fn` in `src/utils/string.rs`, `src/utils/time.rs`, `src/service/metadata/image.rs`, `src/config.rs`, and `src/service/health.rs` is exercised by at least one test.

- **AC-T3:** New tests must pass after dependency upgrades (i.e., tests are forward-compatible with upgraded deps).

### CI Pipeline

- **AC-C1:** A `cargo test` job runs in CI before the Docker build job.
- **AC-C2:** The CI test job runs all three feature combinations: `--all-features`, `--no-default-features --features http`, `--no-default-features --features sqs`.
- **AC-C3:** Failing tests block the build/deploy steps.

### OpenAPI Verification

- **AC-O1:** A golden file `tests/fixtures/openapi.json` captures the current OpenAPI 3.0 specification output.
- **AC-O2:** A test compares the runtime-generated OpenAPI against the golden file and fails on any difference.
- **AC-O3:** After `aide` upgrade, the golden file is updated if differences are intentional and non-breaking, or the upgrade is rolled back if breaking.

### Rust & Dependency Upgrades

- **AC-U1:** Rust is upgraded to the latest stable version (≥1.95.0 as of May 2026).
- **AC-U2:** All `=` exact-version pins are removed from `Cargo.toml`; dependencies use `"X.Y"` semver-compatible ranges targeting the latest stable version of each crate.
- **AC-U3:** `cargo build` succeeds with zero errors and zero new warnings.
- **AC-U4:** `cargo test --all-features` passes after upgrades.
- **AC-U5:** `cargo clippy` passes with zero new warnings.
- **AC-U6:** `cargo fmt --check` passes.

### Docker Alignment

- **AC-D1:** `Dockerfile` uses `rust:1.XX-slim` matching the locally installed Rust version.
- **AC-D2:** `Dockerfile.lambda.http` and `Dockerfile.lambda.sqs` use the latest `amazonlinux:2023` base image with the same Rust version via rustup.
- **AC-D3:** All Dockerfiles build successfully with `docker build`.

### `aide` Alpha → Stable

- **AC-A1:** `aide` is upgraded from `=0.16.0-alpha.1` to the latest stable version (investigate whether this is 0.15.1 or if a >0.16 stable exists).
- **AC-A2:** If `aide` upgrade causes API contract changes, they are documented and approved before proceeding.

## Constraints

- **C1:** All existing 76 tests MUST pass before any code or dependency changes are made.
- **C2:** The OpenAPI golden file test is the gatekeeper for `aide` upgrade — any diff is a failure unless explicitly approved.
- **C3:** No business logic changes except those required by dependency API changes.
- **C4:** Rust 2021 edition remains unchanged.
- **C5:** `Cargo.lock` remains committed for deterministic builds.
- **C6:** Feature flags (`http`, `sqs`, `default`) must remain functional with the same feature-gated dependency structure.

## Non-Goals

- **NG1:** Line/branch coverage metrics or coverage tooling (`cargo-llvm-cov`, `tarpaulin`).
- **NG2:** Integration tests against real AWS services (S3, DynamoDB, SSM).
- **NG3:** Refactoring existing code (no structural changes except upgrade-required API adjustments).
- **NG4:** Performance benchmarking or optimization.
- **NG5:** Adding a `rust-toolchain.toml` file (toolchain is managed via Dockerfiles and local install).
- **NG6:** Fixing pre-existing `clippy` warnings unrelated to upgrades.
- **NG7:** Upgrading to nightly Rust or beta channel.
