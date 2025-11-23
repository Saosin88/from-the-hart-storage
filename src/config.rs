use config::ConfigError;
use serde::Deserialize;
use std::env;
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
pub struct CloudFront {
    pub key_pair_id: String,
    #[serde(default = "default_private_key_ssm_path")]
    pub private_key_ssm_path: String,
    pub domain: String,
}

fn default_private_key_ssm_path() -> String {
    "/from-the-hart-tech-storage/dev/cloudfront-private-key".to_string()
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub environment: String,
    #[serde(default)]
    pub server: Option<ServerConfig>,
    #[serde(default)]
    pub timezone: Option<String>,

    pub dynamodb: Option<DynamoDB>,
    pub cloudfront: Option<CloudFront>,
}

impl AppConfig {
    pub fn load() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();

        let environment = env::var("APP_ENVIRONMENT")
            .map_err(|_| ConfigError::Message("APP_ENVIRONMENT is required".to_string()))?;

        let server = if let (Ok(host), Ok(port_str)) = (
            env::var("APP_SERVER_HOST"),
            env::var("APP_SERVER_PORT"),
        ) {
            let port = port_str.parse::<u16>().map_err(|e| {
                ConfigError::Message(format!("Invalid APP_SERVER_PORT: {}", e))
            })?;
            Some(ServerConfig { host, port })
        } else {
            None
        };

        let timezone = env::var("APP_TIMEZONE").ok();

        let dynamodb = env::var("APP_DYNAMODB_TABLE")
            .ok()
            .map(|table| DynamoDB { table });

        let cloudfront = match (
            env::var("APP_CLOUDFRONT_KEY_PAIR_ID").ok(),
            env::var("APP_CLOUDFRONT_DOMAIN").ok(),
        ) {
            (Some(key_pair_id), Some(domain)) => {
                let private_key_ssm_path = env::var("APP_CLOUDFRONT_PRIVATE_KEY_SSM_PATH")
                    .unwrap_or_else(|_| default_private_key_ssm_path());
                Some(CloudFront {
                    key_pair_id,
                    private_key_ssm_path,
                    domain,
                })
            }
            _ => None,
        };

        Ok(AppConfig {
            environment,
            server,
            timezone,
            dynamodb,
            cloudfront,
        })
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
