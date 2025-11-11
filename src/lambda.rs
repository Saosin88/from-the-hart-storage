use from_the_hart_storage::{config, handlers::sqs_worker, logging, routes, utils::time};

use aws_lambda_events::event::sqs::SqsEvent;
use lambda_http::Error;
use lambda_runtime::LambdaEvent;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Error> {
    time::init_start_time();
    config::init_config();
    logging::init_logging();

    let handler_type = &config::config().handler_type;

    match handler_type {
        config::HandlerType::HTTP => {
            info!(
                environment = %config::config().environment,
                "From The Hart Storage starting on Lambda as HTTP Handler"
            );

            let app = routes::configure_routes();
            let app = ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .service(app);

            lambda_http::run(app).await
        }
        config::HandlerType::SQS => {
            info!(
                environment = %config::config().environment,
                "From The Hart Storage starting on Lambda as SQS Handler"
            );

            lambda_runtime::run(lambda_runtime::service_fn(
                |event: LambdaEvent<SqsEvent>| async move {
                    sqs_worker::handle_sqs_event(event.payload)
                        .await
                        .map_err(|e| format!("Handler error: {}", e))
                },
            ))
            .await
        }
    }
}
