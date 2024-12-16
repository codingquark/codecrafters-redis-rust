use config::{Config, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub address: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
}

pub fn load_config() -> Result<AppConfig, config::ConfigError> {
    Config::builder()
        .add_source(File::with_name("config.toml"))
        .build()?
        .try_deserialize()
}