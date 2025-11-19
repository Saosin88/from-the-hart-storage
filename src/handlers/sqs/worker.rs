use crate::{
    error::StorageError,
    service::{file::create::handle_file_created, File},
    utils::string,
};
use aws_lambda_events::event::{
    s3::S3Event,
    sqs::{BatchItemFailure, SqsBatchResponse, SqsEvent},
};
use tracing::{error, info};

pub async fn handle_sqs_event(event: SqsEvent) -> Result<SqsBatchResponse, StorageError> {
    info!("Received SQS batch with {} messages", event.records.len());

    let mut failures = Vec::new();

    for record in event.records {
        let message_id = record.message_id.as_deref().unwrap_or("");

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
                error!(message_id = %message_id, "S3 record missing bucket name");
                continue;
            };

            let bucket = if let Some(b) = rec.s3.bucket.name.as_deref() {
                b
            } else {
                error!(message_id = %message_id, "S3 record missing bucket name");
                continue;
            };

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

            let file = File::new(key, bucket.to_string());

            match event {
                name if name.starts_with("ObjectCreated:") => {
                    handle_file_created(file).await?;
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
