use std::task::{Context, Poll};

use axum::{
    async_trait,
    body::Body,
    http::Request,
    response::{IntoResponse, Response},
};
use futures_util::future::BoxFuture;
use tower::{Layer, Service};

use crate::resp;

#[async_trait]
pub trait Intercept: Clone {
    type Ctx: Send;
    /// 返回 Err 将不会往下执行
    async fn before(&self, req: &mut Request<Body>) -> resp::Result<Self::Ctx>;
    /// 如果 before 返回 Err 将不会执行这个
    async fn after(&self, _ctx: Self::Ctx, _res: &mut Response) {}
}

/// 拦截器
#[derive(Clone)]
pub struct Interceptor<T> {
    pub interceptor: T,
}

impl<T: Clone> Interceptor<T> {
    pub fn new(interceptor: T) -> Self {
        Self { interceptor }
    }
}

impl<S, T: Clone> Layer<S> for Interceptor<T> {
    type Service = InterceptorService<S, T>;

    fn layer(&self, inner: S) -> Self::Service {
        InterceptorService { inner, interceptor: self.interceptor.clone() }
    }
}

// 拦截器服务
#[derive(Clone)]
pub struct InterceptorService<S, T> {
    pub inner: S,
    pub interceptor: T,
}

impl<S, T> Service<Request<Body>> for InterceptorService<S, T>
where
    S: Service<Request<Body>, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    T: Intercept + Sync + Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        let interceptor = self.interceptor.clone();

        let not_ready_inner = self.inner.clone();
        let mut ready_inner = std::mem::replace(&mut self.inner, not_ready_inner);

        Box::pin(async move {
            match interceptor.before(&mut req).await {
                Ok(ctx) => {
                    let mut response = ready_inner.call(req).await?;
                    interceptor.after(ctx, &mut response).await;
                    Ok(response)
                }
                Err(err) => Ok(err.into_response()),
            }
        })
    }
}
