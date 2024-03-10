crate::re_export! {
   mod server;
   mod jwt;
}

use std::{fs, io, process};

use once_cell::sync::Lazy;
use serde::Deserialize;

use crate::middleware::logger::LoggerConfig;

pub static CONFIG: Lazy<Config> = Lazy::new(Config::new);

#[derive(Debug, Deserialize)]
pub struct Config {
    #[cfg(feature = "database")]
    pub database: crate::database::PgConfig,
    pub logger: LoggerConfig,
    pub server: ServerConfig,
    pub jwt: JwtConfig,
}

impl Config {
    fn new() -> Self {
        Self::parse().unwrap_or_else(|err| {
            eprintln!("加载配置失败: {err}");
            process::exit(0)
        })
    }

    fn parse() -> io::Result<Self> {
        let config = fs::read_to_string("config.toml")?;
        toml::from_str(&config).map_err(io::Error::other)
    }
}
