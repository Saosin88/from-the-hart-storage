use crate::service::events;
use aws_lambda_events::event::{s3::S3Event, sqs::{BatchItemFailure, SqsBatchResponse, SqsEvent, SqsMessage}};
use tracing::{error, info};

fn url_decode(s: &str) -> anyhow::Result<String> {
    Ok(urlencoding::decode(s)?.into_owned())
}

pub async fn handle_sqs_event(event: SqsEvent) -> Result<SqsBatchResponse, anyhow::Error> {
    info!("Received SQS batch with {} messages", event.records.len());

    let mut failures = Vec::new();

    for record in event.records {
        let message_id = record.message_id.clone().unwrap_or_default();

        // Parse the SQS message body as an S3Event (S3 -> SQS notifications)
        let body = match record.body.as_ref() {
            Some(b) => b,
            None => {
                error!(message_id = %message_id, "SQS message has no body");
                failures.push(BatchItemFailure { item_identifier: message_id });
                continue;
            }
        };

        let s3_event: S3Event = match serde_json::from_str(body) {
            Ok(ev) => ev,
            Err(e) => {
                error!(message_id = %message_id, error = %e, "Failed to parse S3 event from SQS body");
                failures.push(BatchItemFailure { item_identifier: message_id });
                continue;
            }
        };

        // Handle each S3 record in the S3Event by mapping to FileRecord and calling service
        for rec in s3_event.records {
            let bucket = match rec.s3.bucket.name {
                Some(ref b) => b.clone(),
                None => {
                    error!(message_id = %message_id, "S3 record missing bucket name");
                    continue;
                }
            };

            let key_raw = match rec.s3.object.key {
                Some(ref k) => k.clone(),
                None => {
                    error!(message_id = %message_id, "S3 record missing object key");
                    continue;
                }
            };

            let key = match url_decode(&key_raw) {
                Ok(k) => k,
                Err(e) => {
                    error!(message_id = %message_id, error = %e, "Failed to URL-decode S3 object key");
                    continue;
                }
            };

            let size = rec.s3.object.size.unwrap_or(0);

            let file_record = crate::service::FileRecord::with_metadata(
                bucket.clone(),
                key.clone(),
                size,
                None,
                None,
            );

            match events::process_s3_event_message(file_record).await {
                Ok(_) => {
                    info!(message_id = %message_id, bucket = %bucket, key = %key, "Successfully processed file record");
                }
                Err(e) => {
                    error!(message_id = %message_id, bucket = %bucket, key = %key, error = %e, "Failed to process file record");
                    failures.push(BatchItemFailure { item_identifier: message_id.clone() });
                }
            }
        }
    }

    Ok(SqsBatchResponse { batch_item_failures: failures })
}
