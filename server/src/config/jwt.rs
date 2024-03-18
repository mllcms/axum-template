use std::fmt::{Debug, Formatter};

use library::jsonwebtoken::Secret;
use serde::{Deserialize, Deserializer};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JwtConfig {
    #[serde(deserialize_with = "parse_secret")]
    pub secret: Secret,
    pub duration: u64,
}

fn parse_secret<'de, D: Deserializer<'de>>(de: D) -> Result<Secret, D::Error> {
    Ok(Secret::new(&String::deserialize(de)?))
}

impl Debug for JwtConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Jwt").field("duration", &self.duration).finish()
    }
}
