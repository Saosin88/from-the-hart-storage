use config::{Config, ConfigError, Environment};

use once_cell::sync::Lazy;
use serde::Deserialize;

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
    pub handler_type: HandlerType,
}

impl AppConfig {
    fn load() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();

        let builder = Config::builder().add_source(Environment::with_prefix("APP").separator("_"));

        let settings = builder.build()?;
        settings.try_deserialize()
    }
}

static CONFIG: Lazy<AppConfig> = Lazy::new(|| {
    AppConfig::load().expect("Failed to load configuration. Check environment variables.")
});

pub fn init_config() {
    Lazy::force(&CONFIG);
}

pub fn config() -> &'static AppConfig {
    &CONFIG
}
