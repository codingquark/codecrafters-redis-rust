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
    let config: AppConfig = Config::builder()
        .set_default("dir", "data")?
        .set_default("dbfilename", "redis.db")?
        .add_source(File::with_name("config.toml"))
        .build()?
        .try_deserialize()?;

    Ok(config)
}
