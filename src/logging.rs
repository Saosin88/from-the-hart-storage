use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_logging() {
    if std::env::var("RUST_LOG").is_err() {
        panic!("RUST_LOG environment variable must be set");
    }
    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .json()
                .with_ansi(false)
                .with_current_span(true)
                .with_span_list(false),
        )
        .with(tracing_subscriber::EnvFilter::from_default_env())
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

        let bt = std::backtrace::Backtrace::force_capture();

        tracing::error!(panic = %payload, location = %location, backtrace = ?bt, "panic occurred");

        eprintln!("PANIC: {} at {}\n{:?}", payload, location, bt);
    }));
}
