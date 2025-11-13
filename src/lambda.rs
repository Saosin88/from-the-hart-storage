use from_the_hart_storage::{
    config, handlers::http::routes, handlers::sqs::worker, logging, utils::time,
};

use aws_lambda_events::event::sqs::SqsEvent;
use lambda_http::Error;
use lambda_runtime::LambdaEvent;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize logging first so we can see errors
    logging::init_logging();
    
    info!("Lambda function starting - initializing components");
    
    // Initialize time tracking
    time::init_start_time();
    info!("Time tracking initialized");
    
    // Initialize configuration (this loads environment variables)
    config::init_config();
    info!("Configuration loaded");

    let handler_type = &config::config().handlertype;

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
                    worker::handle_sqs_event(event.payload)
                        .await
                        .map_err(|e| format!("Handler error: {}", e))
                },
            ))
            .await
        }
    }
}
