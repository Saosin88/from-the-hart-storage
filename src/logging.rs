use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init_logging() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .json()
                .with_ansi(false)
                .with_current_span(true)
                .with_span_list(false)
                .with_target(true)
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(true)
                .with_thread_names(true),
        )
        .with(filter)
        .with(tracing_error::ErrorLayer::default())
        .init();

    std::panic::set_hook(Box::new(|panic_info| {
        let payload = panic_info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| panic_info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "Unknown panic payload".to_string());

        let location = panic_info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown".to_string());

        eprintln!(
            "{{\"level\":\"ERROR\",\"panic\":\"{}\",\"location\":\"{}\"}}",
            payload.replace('"', "\\\""),
            location
        );

        tracing::error!(
            panic = %payload,
            location = %location,
            "PANIC: Application panicked"
        );
    }));
}
