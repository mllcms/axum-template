//! # Examples
//!
//! ```rust, ignore
//! let config = PgConfig{..};
//! Router::new()
//! .route("/user",post(get_user))
//! .layer(config.build().await);
//!
//! async fn get_user(PgConn(conn):PgConn) {}
//! ```

use std::{
    fmt::{Display, Formatter},
    process,
};

use axum::{async_trait, extract::FromRequestParts, http::request::Parts, Extension};
use bb8::{ManageConnection, PooledConnection};
use diesel_async::{pooled_connection::AsyncDieselConnectionManager, AsyncPgConnection};
use serde::{Deserialize, Serialize};

use crate::resp::Res;

pub type PgPool = bb8::Pool<AsyncDieselConnectionManager<AsyncPgConnection>>;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PgConfig {
    pub database: String,
    pub hostname: String,
    pub username: String,
    pub password: String,
}

impl PgConfig {
    pub async fn build(&self) -> Extension<PgPool> {
        let config = AsyncDieselConnectionManager::new(self.to_string());
        if config.connect().await.is_err() {
            eprintln!("Postgres 连接失败, 请检查配置");
            process::exit(0)
        }
        Extension(PgPool::builder().build(config).await.unwrap())
    }
}

impl Display for PgConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "postgres://{}:{}@{}/{}",
            self.username, self.password, self.hostname, self.database
        )
    }
}

/// 获取 pg 连接
pub struct PgConn(pub PooledConnection<'static, AsyncDieselConnectionManager<AsyncPgConnection>>);

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for PgConn {
    type Rejection = Res<()>;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        match parts.extensions.get::<PgPool>() {
            Some(pool) => match pool.get_owned().await.map(Self) {
                Ok(conn) => Ok(conn),
                Err(err) => panic!("获取 Postgres 连接失败: {}", err),
            },
            None => panic!("未设置 Postgres 连接池"),
        }
    }
}
