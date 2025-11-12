//! # Deprecated S3 utilities
//!
//! **This module is deprecated.** Use `crate::repository::s3` instead.
//!
//! The functions in this module have been moved to the repository layer
//! to properly separate infrastructure concerns from business logic.
//!
//! - `get_object_metadata` → `crate::repository::s3::get_object_metadata`
//! - `fetch_head_bytes` → `crate::repository::s3::fetch_head_bytes`
//!
//! This module is kept for backward compatibility but should not be used
//! in new code.

use anyhow::{Context, Result};
use aws_sdk_s3::Client as S3Client;
use std::io::Cursor;

/// Fetch a byte range from an S3 object
/// This allows us to download only the portion of the file we need
/// without downloading the entire file
pub async fn fetch_byte_range(
    s3_client: &S3Client,
    bucket: &str,
    key: &str,
    start: u64,
    end: u64,
) -> Result<Vec<u8>> {
    let range = format!("bytes={}-{}", start, end);

    let response = s3_client
        .get_object()
        .bucket(bucket)
        .key(key)
        .range(&range)
        .send()
        .await
        .with_context(|| {
            format!(
                "Failed to fetch byte range {} from S3 object s3://{}/{}",
                range, bucket, key
            )
        })?;

    let bytes = response
        .body
        .collect()
        .await
        .with_context(|| {
            format!(
                "Failed to read S3 response body for s3://{}/{}",
                bucket, key
            )
        })?
        .into_bytes();

    Ok(bytes.to_vec())
}

/// Fetch the first N bytes of an S3 object
/// Useful for reading file headers to determine format and extract metadata
pub async fn fetch_head_bytes(
    s3_client: &S3Client,
    bucket: &str,
    key: &str,
    num_bytes: u64,
) -> Result<Vec<u8>> {
    fetch_byte_range(s3_client, bucket, key, 0, num_bytes - 1).await
}

/// Create a Cursor (in-memory reader) from bytes
/// This allows us to use standard Read/Seek traits with byte slices
pub fn bytes_to_cursor(bytes: Vec<u8>) -> Cursor<Vec<u8>> {
    Cursor::new(bytes)
}

/// Get the content type and last modified date from S3 object metadata
pub async fn get_object_metadata(
    s3_client: &S3Client,
    bucket: &str,
    key: &str,
) -> Result<(Option<String>, chrono::DateTime<chrono::Utc>)> {
    let response = s3_client
        .head_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .with_context(|| {
            format!(
                "Failed to get S3 object metadata for s3://{}/{}",
                bucket, key
            )
        })?;

    let content_type = response.content_type().map(|s| s.to_string());

    let last_modified = response
        .last_modified()
        .and_then(|dt| {
            chrono::DateTime::parse_from_rfc3339(&dt.to_string())
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        })
        .unwrap_or_else(chrono::Utc::now);

    Ok((content_type, last_modified))
}
