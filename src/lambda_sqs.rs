use from_the_hart_storage::{config, handlers::sqs::worker, logging, utils::time};

use aws_lambda_events::event::sqs::SqsEvent;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Error> {
    logging::init_logging();
    time::init_start_time();
    config::init_config();

    info!(
        environment = %config::config().environment,
        default_timezone = ?config::config().default_timezone,
        "From The Hart Storage SQS Handler starting on Lambda"
    );

    run(service_fn(|event: LambdaEvent<SqsEvent>| async move {
        worker::handle_sqs_event(event.payload)
            .await
            .map_err(|e| Error::from(format!("Handler error: {}", e)))
    }))
    .await
}
