use anyhow::Result;
use async_trait::async_trait;

use super::types::MediaMetadata;
use crate::service::FileRecord;

/// Trait for extracting metadata from different media types
#[async_trait]
pub trait MetadataExtractor: Send + Sync {
    /// Check if this extractor can handle the given file
    /// Based on extension or content type
    fn can_handle(&self, extension: &str, content_type: Option<&str>) -> bool;

    /// Extract metadata from file bytes and metadata
    /// Uses file header bytes to avoid processing entire file
    async fn extract(
        &self,
        head_bytes: &[u8],
        file_record: &FileRecord,
    ) -> Result<MediaMetadata>;
}
