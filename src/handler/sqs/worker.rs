use crate::{
    error::StorageError,
    service::{file::create::handle_file_created, File},
    state::AppState,
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
    state: &AppState,
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
            let mut failure = BatchItemFailure::default();
            failure.item_identifier = message_id.to_string();
            failures.push(failure);
            continue;
        };

        let s3_event: S3Event = match serde_json::from_str(body) {
            Ok(ev) => ev,
            Err(e) => {
                error!(message_id = %message_id, error = %e, "Failed to parse S3 event from SQS body");
                let mut failure = BatchItemFailure::default();
                failure.item_identifier = message_id.to_string();
                failures.push(failure);
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

            span.record("key", key.as_str());

            if key.as_str().ends_with("/") {
                info!(
                    message_id = %message_id,
                    key = %key,
                    "Skipping S3 zero-byte folder placeholder object"
                );
                continue;
            }

            let file = File::new(key, bucket.to_string());

            match event {
                name if name.starts_with("ObjectCreated:") => {
                    if let Err(e) = handle_file_created(
                        file,
                        state
                            .s3_repository
                            .as_ref()
                            .expect("s3_repository is required for SQS handler")
                            .as_ref(),
                        state.dynamo_db_repository.as_ref(),
                        state
                            .metadata_service
                            .as_ref()
                            .expect("metadata_service is required for SQS handler")
                            .as_ref(),
                    )
                    .await
                    {
                        error!(
                            message_id = %message_id,
                            error = %e,
                            error_source = ?e.source(),
                            "Failed to handle file creation"
                        );
                        let mut failure = BatchItemFailure::default();
                        failure.item_identifier = message_id.to_string();
                        failures.push(failure);
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

    let mut response = SqsBatchResponse::default();
    response.batch_item_failures = failures;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::mock::{MockDynamoDbRepository, MockMetadataService, MockS3Repository};
    use aws_lambda_events::event::sqs::SqsMessage;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_handle_sqs_event_success() {
        let s3_mock = MockS3Repository::new();
        let dynamodb_mock = MockDynamoDbRepository::new();
        let metadata_mock = MockMetadataService::new();

        // Construct a valid S3 event inside SQS message
        let s3_event_json = r#"{
            "Records": [
                {
                    "eventVersion": "2.0",
                    "eventSource": "aws:s3",
                    "awsRegion": "us-east-1",
                    "eventTime": "1970-01-01T00:00:00.000Z",
                    "eventName": "ObjectCreated:Put",
                    "userIdentity": {
                        "principalId": "EXAMPLE"
                    },
                    "requestParameters": {
                        "sourceIPAddress": "127.0.0.1"
                    },
                    "responseElements": {
                        "x-amz-request-id": "EXAMPLE123456789",
                        "x-amz-id-2": "EXAMPLE123/5678abcdefghijklambdaisawesome/mnopqrstuvwxyzABCDEFGH"
                    },
                    "s3": {
                        "s3SchemaVersion": "1.0",
                        "configurationId": "testConfigRule",
                        "bucket": {
                            "name": "example-bucket",
                            "ownerIdentity": {
                                "principalId": "EXAMPLE"
                            },
                            "arn": "arn:aws:s3:::example-bucket"
                        },
                        "object": {
                            "key": "test/key",
                            "size": 1024,
                            "eTag": "0123456789abcdef0123456789abcdef",
                            "sequencer": "0A1B2C3D4E5F678901"
                        }
                    }
                }
            ]
        }"#;

        let mut message = SqsMessage::default();
        message.message_id = Some("msg-id".into());
        message.body = Some(s3_event_json.into());

        let mut event = SqsEvent::default();
        event.records = vec![message];

        // Configure mocks for success
        // We need to mock S3 metadata call which happens inside handle_file_created
        use aws_sdk_s3::operation::head_object::HeadObjectOutput;
        let s3_mock = s3_mock
            .with_head_object_response(Ok(HeadObjectOutput::builder().build()))
            .with_fetch_head_bytes_response(Ok(vec![]));

        let state = AppState::new(
            Some(Arc::new(s3_mock)),
            Arc::new(dynamodb_mock.clone()),
            Some(Arc::new(metadata_mock)),
            None,
        );

        let result = handle_sqs_event(event, &state).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.batch_item_failures.is_empty());

        let calls = dynamodb_mock.put_file_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
    }
}
