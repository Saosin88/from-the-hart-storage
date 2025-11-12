use anyhow::{Context, Result};
use async_trait::async_trait;
use aws_sdk_s3::Client as S3Client;
use mp4parse::read_mp4;

use super::extractor::MetadataExtractor;
use super::types::{MediaMetadata, MediaType, VideoMetadata};
use crate::utils::s3;

pub struct VideoMetadataExtractor;

impl VideoMetadataExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Parse MP4 metadata from raw bytes
    fn parse_mp4_metadata(&self, bytes: &[u8]) -> Option<VideoMetadata> {
        let mut cursor = std::io::Cursor::new(bytes);

        match read_mp4(&mut cursor) {
            Ok(context) => {
                let mut video_meta = VideoMetadata {
                    width: None,
                    height: None,
                    duration: None,
                    codec: None,
                    frame_rate: None,
                    bitrate: None,
                };

                // Calculate duration from timescale
                if let Some(timescale) = context.timescale {
                    // Duration is in timescale units, convert to seconds
                    // Note: mp4parse doesn't directly expose duration in newer versions
                    // We'll need to calculate it from track durations
                    for track in &context.tracks {
                        if let Some(ref tkhd) = track.tkhd {
                            let duration = tkhd.duration;
                            let duration_secs = duration as f64 / timescale.0 as f64;
                            video_meta.duration = Some(duration_secs);
                            break; // Use first track duration
                        }
                    }
                }

                // Extract video track information
                for track in &context.tracks {
                    if let Some(ref tkhd) = track.tkhd {
                        video_meta.width = Some(tkhd.width);
                        video_meta.height = Some(tkhd.height);
                    }

                    // Try to get codec information
                    if let Some(ref stsd) = track.stsd {
                        if let Some(mp4parse::SampleEntry::Video(ref video)) =
                            stsd.descriptions.first()
                        {
                            video_meta.codec = Some(format!("{:?}", video.codec_type));
                        }
                    }
                }

                Some(video_meta)
            }
            Err(e) => {
                tracing::warn!("Failed to parse MP4 metadata: {:?}", e);
                None
            }
        }
    }
}

#[async_trait]
impl MetadataExtractor for VideoMetadataExtractor {
    fn can_handle(&self, extension: &str, content_type: Option<&str>) -> bool {
        let ext = extension.to_lowercase();
        let is_video_ext = matches!(
            ext.as_str(),
            "mp4" | "m4v" | "mov" | "avi" | "mkv" | "webm" | "flv" | "wmv" | "3gp"
        );

        let is_video_mime = content_type
            .map(|ct| ct.to_lowercase().starts_with("video/"))
            .unwrap_or(false);

        is_video_ext || is_video_mime
    }

    async fn extract(
        &self,
        s3_client: &S3Client,
        bucket: &str,
        key: &str,
        file_size: i64,
    ) -> Result<MediaMetadata> {
        // Fetch S3 metadata first
        let (content_type, last_modified) = s3::get_object_metadata(s3_client, bucket, key)
            .await
            .context("Failed to get S3 object metadata")?;

        // Create basic metadata
        let mut metadata = MediaMetadata::new_basic(
            bucket.to_string(),
            key.to_string(),
            file_size,
            last_modified,
        );
        metadata.media_type = MediaType::Video;
        metadata.content_type = content_type;

        // Fetch first 1MB for video header parsing
        // MP4 metadata is usually in the first part of the file
        let num_bytes = std::cmp::min(1024 * 1024, file_size as u64);

        match s3::fetch_head_bytes(s3_client, bucket, key, num_bytes).await {
            Ok(bytes) => {
                // Try to parse MP4 metadata
                if let Some(video_meta) = self.parse_mp4_metadata(&bytes) {
                    metadata.video_metadata = Some(video_meta);
                } else {
                    // If parsing fails, still return basic metadata
                    tracing::warn!(
                        "Could not parse video metadata for {}/{}. Using basic metadata only.",
                        bucket,
                        key
                    );
                }
            }
            Err(e) => {
                tracing::error!(
                    "Failed to fetch video bytes for {}/{}: {:?}. Using basic metadata only.",
                    bucket,
                    key,
                    e
                );
            }
        }

        Ok(metadata)
    }
}
