use std::{
    marker::PhantomData,
    task::{Context, Poll},
};

use axum::{body::Body, extract::Request, response::Response};
use futures_util::future::BoxFuture;
use tower::{Layer, Service};

use crate::{
    compare::{always_false, CompareStr},
    jsonwebtoken::{auth_token, JwtToken},
};

/// # Examples
///
/// ```rust,ignore
/// JwtAuth::<User, _>::new(always_false);
/// fn always_false(_uri: &str) -> bool {
///     true
/// }
///
/// JwtAuth::<User, _>::new("/login");
/// JwtAuth::<User, _>::new(&["/login"]);
///
/// JwtAuth::<User, _>::new(Arc::new(String::from("/login")));
/// JwtAuth::<User, _>::new(Arc::new(vec![String::from("/login")]));
/// ```
#[derive(Clone)]
pub struct JwtAuth<T, A> {
    allow: A,
    // 幻象数据存储类型不会占内存
    payload: PhantomData<T>,
}

impl<T: JwtToken, A: CompareStr> JwtAuth<T, A> {
    /// allow 返回 true 时免验证
    pub fn new(allow: A) -> Self {
        Self { allow, payload: PhantomData }
    }
}

impl<T: JwtToken> Default for JwtAuth<T, fn(&str) -> bool> {
    fn default() -> Self {
        Self::new(always_false)
    }
}

impl<S, T: JwtToken, A: CompareStr> Layer<S> for JwtAuth<T, A> {
    type Service = JwtAuthService<S, T, A>;

    fn layer(&self, inner: S) -> Self::Service {
        JwtAuthService { inner, allow: self.allow.clone(), payload: self.payload }
    }
}

#[derive(Clone)]
pub struct JwtAuthService<S, T, A> {
    inner: S,
    allow: A,
    payload: PhantomData<T>,
}

impl<S, T, A> Service<Request<Body>> for JwtAuthService<S, T, A>
where
    S: Service<Request<Body>, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
    T: JwtToken + Sync + Send + 'static,
    A: CompareStr,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        // 免验证直接放行
        if self.allow.compare(req.uri().path()) {
            return Box::pin(self.inner.call(req));
        }

        let result = auth_token::<T>(req.headers()).map(|claims| {
            req.extensions_mut().insert(claims);
            self.inner.call(req)
        });

        Box::pin(async move {
            match result {
                Ok(future) => future.await,
                Err(response) => Ok(response),
            }
        })
    }
}
