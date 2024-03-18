use library::jsonwebtoken::{JwtToken, Secret};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::config::CONFIG;

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct User {
    pub name: String,
    pub phone: String,
}

impl JwtToken for User {
    fn secret() -> &'static Secret {
        &CONFIG.jwt.secret
    }
}
