use config::{Config, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub address: String,
    pub port: u16
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub dir: Option<String>,
    pub dbfilename: Option<String>,
}

pub fn load_config() -> Result<AppConfig, config::ConfigError> {
    let mut config: AppConfig = Config::builder()
        .add_source(File::with_name("config.toml"))
        .build()?
        .try_deserialize()?;

    set_defaults(&mut config);

    Ok(config)
}

pub fn set_defaults(config: &mut AppConfig) {
    if config.dir.is_none() {
        config.dir = Some("data".to_string());
    }
    if config.dbfilename.is_none() {
        config.dbfilename = Some("redis.db".to_string());
    }
}