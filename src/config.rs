use config::{Config, ConfigError, Environment};
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DynamoDB {
    pub table: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub environment: String,
    #[serde(default)]
    pub server: Option<ServerConfig>,
    #[serde(default)]
    pub timezone: Option<String>,

    pub dynamodb: Option<DynamoDB>,
}

impl AppConfig {
    pub fn load() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();

        let builder = Config::builder().add_source(Environment::with_prefix("APP").separator("_"));

        let settings = builder.build()?;
        settings.try_deserialize()
    }
}

static CONFIG: OnceLock<AppConfig> = OnceLock::new();

/// Initialize config - must be called after logging is set up
pub fn init_config() -> Result<(), ConfigError> {
    let cfg = AppConfig::load()?;
    
    // Safe to use tracing here since caller ensures logging is initialized
    tracing::info!("Loaded config: {:#?}", cfg);
    
    CONFIG.set(cfg).map_err(|_| {
        ConfigError::Message("Config already initialized".to_string())
    })?;
    
    Ok(())
}

pub fn config() -> &'static AppConfig {
    CONFIG.get().expect("Config not initialized - call init_config() first")
}
