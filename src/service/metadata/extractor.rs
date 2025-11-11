use anyhow::Result;
use async_trait::async_trait;
use aws_sdk_s3::Client as S3Client;

use super::types::MediaMetadata;

/// Trait for extracting metadata from different media types
#[async_trait]
pub trait MetadataExtractor: Send + Sync {
    /// Check if this extractor can handle the given file
    /// Based on extension or content type
    fn can_handle(&self, extension: &str, content_type: Option<&str>) -> bool;

    /// Extract metadata from an S3 object
    /// Uses streaming/range requests to avoid downloading entire file
    async fn extract(
        &self,
        s3_client: &S3Client,
        bucket: &str,
        key: &str,
        file_size: i64,
    ) -> Result<MediaMetadata>;
}
