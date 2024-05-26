library::re_export! {
   mod server;
   mod jwt;
}

use library::{config::ConfigLoad, logger::LoggerConfig};
use once_cell::sync::Lazy;
use serde::Deserialize;

pub static CONFIG: Lazy<Config> = Lazy::new(|| Config::from_toml("config.toml", "加载配置失败: "));

#[derive(Debug, Deserialize)]
pub struct Config {
    #[cfg(feature = "database")]
    pub database: crate::database::PgConfig,
    pub logger: LoggerConfig,
    pub server: ServerConfig,
    pub jwt: JwtConfig,
}

impl ConfigLoad for Config {}
