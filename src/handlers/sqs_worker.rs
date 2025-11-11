use crate::service::events;
use aws_lambda_events::event::sqs::{BatchItemFailure, SqsBatchResponse, SqsEvent};
use tracing::{error, info};

pub async fn handle_sqs_event(event: SqsEvent) -> Result<SqsBatchResponse, anyhow::Error> {
    info!("Received SQS batch with {} messages", event.records.len());

    let mut failures = Vec::new();

    for record in event.records {
        let message_id = record.message_id.clone().unwrap_or_default();

        match events::process_s3_event_message(&record).await {
            Ok(_) => {
                info!(message_id = %message_id, "Successfully processed message");
            }
            Err(e) => {
                error!(message_id = %message_id, error = %e, "Failed to process message");
                failures.push(BatchItemFailure {
                    item_identifier: message_id,
                });
            }
        }
    }

    Ok(SqsBatchResponse {
        batch_item_failures: failures,
    })
}
