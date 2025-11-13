mod extractor;
mod image;

use crate::service::File;
use extractor::MetadataExtractor;
use image::ImageMetadataExtractor;

use std::path::Path;
use tracing::error;

pub struct MetadataService {
    extractors: Vec<Box<dyn MetadataExtractor>>,
}

impl MetadataService {
    pub fn new() -> Self {
        let extractors: Vec<Box<dyn MetadataExtractor>> =
            vec![Box::new(ImageMetadataExtractor::new())];

        Self { extractors }
    }

    pub async fn extract_metadata(&self, head_bytes: &[u8], file: &mut File) {
        let extension = Path::new(&file.file_name)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_string();

        let content_type = file.content_type.clone();
        let file_name = file.file_name.clone();
        let bucket = file.bucket.clone();

        for extractor in &self.extractors {
            if extractor.can_handle(&extension, Some(&content_type)) {
                extractor.extract_and_add_to_file(head_bytes, file).await;
            } else {
                error!(
                    file = %file_name,
                    bucket = %bucket,
                    content_type = ?content_type,
                    extension = %extension,
                    "no metadata extractor matched this file"
                );
            }
        }
    }
}

impl Default for MetadataService {
    fn default() -> Self {
        Self::new()
    }
}
