use std::{fs, io, process};

use once_cell::sync::Lazy;
use serde::Deserialize;

crate::re_export! {
   mod logger;
   mod server;
}

pub static CONFIG: Lazy<Config> = Lazy::new(Config::new);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub logger: Logger,
    pub server: Server,
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
