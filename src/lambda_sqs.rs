use from_the_hart_storage::{config, handlers::sqs::worker, logging, utils::time};

use aws_lambda_events::event::sqs::SqsEvent;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use std::panic;
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Ensure panics are printed to stderr immediately (visible in Lambda init logs)
    panic::set_hook(Box::new(|info| {
        eprintln!("panic hook: {}", info);
    }));

    eprintln!("lambda_sqs: start main");

    logging::init_logging();
    eprintln!("lambda_sqs: logging initialized");

    time::init_start_time();
    eprintln!("lambda_sqs: time initialized");

    config::init_config();
    eprintln!("lambda_sqs: config initialized");

    info!(
        environment = %config::config().environment,
        timezone = ?config::config().timezone,
        "From The Hart Storage SQS Handler starting on Lambda"
    );

    let result = run(service_fn(|event: LambdaEvent<SqsEvent>| async move {
        worker::handle_sqs_event(event.payload)
            .await
            .map_err(|e| Error::from(format!("Handler error: {}", e)))
    }))
    .await;

    if let Err(e) = &result {
        // Print to stderr so the Lambda platform surfaces the failure during init/invoke
        eprintln!("lambda_sqs: runtime error: {:?}", e);
        error!(error = ?e, "lambda runtime error");
    } else {
        eprintln!("lambda_sqs: run returned Ok");
    }

    result
}
