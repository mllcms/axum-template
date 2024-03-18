use std::net::SocketAddr;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    pub addr: SocketAddr,
    #[serde(default = "default_protocol")]
    pub protocol: String,
}

library::gen_default!(default_protocol, "http");
