use std::net::SocketAddr;

use axum::{async_trait, body::Body, extract::ConnectInfo, http::Request};

use crate::{
    compare::CompareStr,
    interceptor::{Intercept, Interceptor},
    reject, res, resp,
};

#[derive(Debug, Clone)]
pub struct BlackIp<T> {
    handler: T,
}

impl<T: CompareStr> BlackIp<T> {
    pub fn interceptor(handler: T) -> Interceptor<Self> {
        Interceptor::new(Self { handler })
    }
}

#[async_trait]
impl<T: CompareStr + Sync> Intercept for BlackIp<T> {
    type Context = ();

    async fn before(&self, req: &mut Request<Body>) -> resp::Result<Self::Context> {
        let addr = req
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .ok_or_else(|| res!(400, "获取连接 ip 失败"))?;

        let ip = addr.ip().to_string();
        if self.handler.compare(&ip) {
            return reject!(403, "黑名单 ip 禁止访问");
        }
        Ok(())
    }
}
