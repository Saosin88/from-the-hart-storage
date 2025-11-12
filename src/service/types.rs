use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Generic file record - source-agnostic representation of a file
/// Maps from S3 events or other sources to domain model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    /// File name / object key
    pub file_name: String,
    
    /// Bucket / storage location
    pub bucket: String,
    
    /// File size in bytes
    pub file_size: i64,
    
    /// Content type / MIME type
    pub content_type: Option<String>,
    
    /// Last modified timestamp
    pub last_modified: Option<DateTime<Utc>>,
}

impl FileRecord {
    /// Create a new FileRecord with minimal information
    pub fn new(bucket: String, file_name: String, file_size: i64) -> Self {
        Self {
            bucket,
            file_name,
            file_size,
            content_type: None,
            last_modified: None,
        }
    }
    
    /// Create a FileRecord with full metadata
    pub fn with_metadata(
        bucket: String,
        file_name: String,
        file_size: i64,
        content_type: Option<String>,
        last_modified: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            bucket,
            file_name,
            file_size,
            content_type,
            last_modified,
        }
    }
}
