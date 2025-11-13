use config::{Config, ConfigError, Environment};
use serde::Deserialize;
use std::sync::LazyLock;

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub enum HandlerType {
    HTTP,
    SQS,
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub environment: String,
    #[serde(default)]
    pub server: Option<ServerConfig>,
    pub handlertype: HandlerType,
}

impl AppConfig {
    fn load() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();

        tracing::debug!("Loading application configuration");

        let builder = Config::builder().add_source(Environment::with_prefix("APP").separator("_"));

        let settings = builder.build()?;
        let app_config: AppConfig = settings.try_deserialize()?;

        tracing::info!(
            environment = %app_config.environment,
            handler_type = ?app_config.handlertype,
            "Application configuration loaded successfully"
        );

        Ok(app_config)
    }
}

static CONFIG: LazyLock<AppConfig> = LazyLock::new(|| {
    AppConfig::load().unwrap_or_else(|e| {
        eprintln!("Failed to load configuration: {}", e);
        eprintln!("Required environment variables:");
        eprintln!("  APP_ENVIRONMENT");
        eprintln!("  APP_HANDLERTYPE (HTTP or SQS)");
        std::process::exit(1);
    })
});

pub fn init_config() {
    LazyLock::force(&CONFIG);
}

pub fn config() -> &'static AppConfig {
    &CONFIG
}
