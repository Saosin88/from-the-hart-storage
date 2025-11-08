use serde::Deserialize;
use std::sync::OnceLock;

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    #[serde(skip)]
    pub environment: String,
    pub server: ServerConfig,
}

impl AppConfig {
    pub fn load() -> Result<Self, String> {
        let environment = std::env::var("ENVIRONMENT")
            .map_err(|_| "ENVIRONMENT variable is required but not set".to_string())?;

        let config = config::Config::builder()
            .add_source(config::Environment::with_prefix("APP").separator("_"))
            .build()
            .map_err(|e| format!("Failed to build configuration: {}", e))?;

        let mut app_config: AppConfig = config
            .try_deserialize()
            .map_err(|e| format!("Failed to deserialize configuration: {}. Required variables: APP_SERVER_HOST, APP_SERVER_PORT", e))?;

        app_config.environment = environment;

        Ok(app_config)
    }
}

pub static CONFIG: OnceLock<AppConfig> = OnceLock::new();

pub fn init_config() {
    CONFIG.get_or_init(|| AppConfig::load().expect("Failed to load AppConfig"));
}

pub fn config() -> &'static AppConfig {
    CONFIG.get().expect("Config not initialized")
}
