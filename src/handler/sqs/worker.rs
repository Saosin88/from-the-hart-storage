use crate::{
    error::StorageError,
    repository::{DynamoDbRepositoryTrait, S3RepositoryTrait},
    service::{file::create::handle_file_created, metadata::MetadataServiceTrait, File},
    utils::string,
};
use aws_lambda_events::event::{
    s3::S3Event,
    sqs::{BatchItemFailure, SqsBatchResponse, SqsEvent},
};
use std::error::Error;
use tracing::{error, info};

pub async fn handle_sqs_event(
    event: SqsEvent,
    s3_repository: &impl S3RepositoryTrait,
    dynamo_db_repository: &impl DynamoDbRepositoryTrait,
    metadata_service: &impl MetadataServiceTrait,
) -> Result<SqsBatchResponse, StorageError> {
    info!("Received SQS batch with {} messages", event.records.len());

    let mut failures = Vec::new();

    for record in event.records {
        let message_id = record.message_id.as_deref().unwrap_or("");
        
        let span = tracing::info_span!(
            "sqs_message",
            message_id = %message_id,
            event_type = tracing::field::Empty,
            bucket = tracing::field::Empty,
            key = tracing::field::Empty,
        );
        let _enter = span.enter();

        let body = if let Some(b) = record.body.as_deref() {
            b
        } else {
            error!(message_id = %message_id, "SQS message has no body");
            failures.push(BatchItemFailure {
                item_identifier: message_id.to_string(),
            });
            continue;
        };

        let s3_event: S3Event = match serde_json::from_str(body) {
            Ok(ev) => ev,
            Err(e) => {
                error!(message_id = %message_id, error = %e, "Failed to parse S3 event from SQS body");
                failures.push(BatchItemFailure {
                    item_identifier: message_id.to_string(),
                });
                continue;
            }
        };

        for rec in s3_event.records {
            let event = if let Some(e) = rec.event_name.as_deref() {
                e
            } else {
                error!(message_id = %message_id, "S3 record missing event name");
                continue;
            };
            
            span.record("event_type", event);

            let bucket = if let Some(b) = rec.s3.bucket.name.as_deref() {
                b
            } else {
                error!(message_id = %message_id, "S3 record missing bucket name");
                continue;
            };
            
            span.record("bucket", bucket);

            let key_raw = if let Some(k) = rec.s3.object.key.as_deref() {
                k
            } else {
                error!(message_id = %message_id, "S3 record missing object key");
                continue;
            };

            let key = match string::url_decode(key_raw) {
                Ok(k) => k,
                Err(e) => {
                    error!(message_id = %message_id, error = %e, "Failed to URL-decode S3 object key");
                    continue;
                }
            };
            
            span.record("key", &key.as_str());

            let file = File::new(key, bucket.to_string());

            match event {
                name if name.starts_with("ObjectCreated:") => {
                    if let Err(e) = handle_file_created(
                        file,
                        s3_repository,
                        dynamo_db_repository,
                        metadata_service,
                    )
                    .await
                    {
                        error!(
                            message_id = %message_id,
                            error = %e,
                            error_source = ?e.source(),
                            "Failed to handle file creation"
                        );
                        failures.push(BatchItemFailure {
                            item_identifier: message_id.to_string(),
                        });
                    }
                }
                // name if name.starts_with("ObjectRemoved:") => {
                //     handle_s3_object_removed(&client, &s3_event).await?;
                // }
                _ => {
                    error!("Unhandled S3 event: {:?}", rec.event_name);
                }
            }
        }
    }

    Ok(SqsBatchResponse {
        batch_item_failures: failures,
    })
}
