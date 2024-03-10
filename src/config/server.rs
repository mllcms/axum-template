use std::net::SocketAddr;

use serde::Deserialize;

use crate::gen_default;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    pub addr: SocketAddr,
    #[serde(default = "default_protocol")]
    pub protocol: String,
}

gen_default!(default_protocol, "http");
