use async_trait::async_trait;

use crate::service::models::File;

#[async_trait]
pub trait MetadataExtractor: Send + Sync {
    fn can_handle(&self, extension: &str, content_type: Option<&str>) -> bool;

    async fn extract_and_add_to_file(&self, head_bytes: &[u8], file: &mut File);
}
