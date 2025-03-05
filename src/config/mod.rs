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
    pub dir: String,
    pub dbfilename: String,
}

pub fn load_config() -> Result<AppConfig, config::ConfigError> {
    let config: AppConfig = Config::builder()
        .set_default("dir", "data")?
        .set_default("dbfilename", "dump.db")?
        .add_source(File::with_name("config.toml"))
        .build()?
        .try_deserialize()?;

    Ok(config)
}
