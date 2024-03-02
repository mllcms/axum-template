use serde::Deserialize;

use crate::middleware::logger;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Logger {
    pub path: String,
    pub file: bool,
    pub stdout: bool,
}

impl Logger {
    pub fn build(&self) -> logger::Logger {
        logger::Logger::new(&self.path, self.stdout, self.file)
    }
}
