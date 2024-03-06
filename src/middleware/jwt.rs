//! # 提取
//! 使用 [`Jwt`] 提取可以不依赖 [`JwtAuth`] 拿不到数据时会自己解析
//!
//! 使用 [`axum::extract::Extension`] 提取时必需使用 [`JwtAuth`]
//! ```rust,ignore
//! async fn info(Jwt(user): Jwt<User>) -> Resp<Arc<User>> {
//!     /* some handle */
//!     resolve!(200 => user, "获取用户信息成功")
//! }
//! ```
//!
//! # 中间件
//! ```rust,ignore
//! // JwtAuth::<User, _>::default() 和这个一样所有请求都要验证
//! JwtAuth::<User, _>::new(always_false);
//! fn always_false(_uri: &str) -> bool {
//!     false
//! }
//!
//! JwtAuth::<User, _>::new("/login");
//! JwtAuth::<User, _>::new(&["/login"]);
//!
//! JwtAuth::<User, _>::new(Arc::new(String::from("/login")));
//! JwtAuth::<User, _>::new(Arc::new(vec![String::from("/login")]));
//! ```

use std::{
    marker::PhantomData,
    sync::Arc,
    task::{Context, Poll},
};

use axum::{
    async_trait,
    body::Body,
    extract::FromRequestParts,
    http::{request::Parts, HeaderMap, Request},
    response::{IntoResponse, Response},
};
use axum_extra::headers::{authorization::Bearer, Authorization, HeaderMapExt};
use chrono::Local;
use futures_util::future::BoxFuture;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use tower::{Layer, Service};

use crate::res;

#[derive(Debug, Clone)]
pub struct Jwt<T: JwtToken>(pub T);

#[async_trait]
impl<T, S> FromRequestParts<S> for Jwt<T>
where
    T: JwtToken + Send + Sync + 'static,
    S: Send + Sync,
{
    type Rejection = Response;
    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        match parts.extensions.remove::<T>() {
            Some(data) => Ok(Self(data)),
            None => Ok(Self(auth_token(&parts.headers)?)),
        }
    }
}

fn auth_token<T: JwtToken>(header: &HeaderMap) -> Result<T, Response> {
    let auth = header
        .typed_get::<Authorization<Bearer>>()
        .ok_or(res!(401, "身份认证失败: 请求未携带有效token").into_response())?;
    T::decode(auth.token()).map_err(|err| res!(401, "身份认证失败: {err}").into_response())
}

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

impl<T: JwtToken, A: Allow> JwtAuth<T, A> {
    /// allow 返回 true 时免验证
    pub fn new(allow: A) -> Self {
        Self { allow, payload: PhantomData }
    }
}

impl<T: JwtToken> Default for JwtAuth<T, fn(&str) -> bool> {
    fn default() -> Self {
        fn always_false(_: &str) -> bool {
            true
        }
        Self::new(always_false)
    }
}

impl<S, T: JwtToken, A: Allow> Layer<S> for JwtAuth<T, A> {
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
    A: Allow,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        // 免验证直接放行
        if self.allow.allow(req.uri().path()) {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims<T> {
    exp: usize,
    data: T,
}

pub trait JwtToken: Serialize + for<'a> Deserialize<'a> + Clone {
    fn secret() -> &'static Secret;

    fn encode(self) -> anyhow::Result<String> {
        let exp = Local::now().timestamp() as usize + Self::duration();
        let claims = Claims { data: self, exp };
        let secret = Self::secret();
        Ok(jsonwebtoken::encode(&secret.header, &claims, &secret.encoding_key)?)
    }

    fn decode(token: &str) -> anyhow::Result<Self> {
        let secret = Self::secret();
        let data = jsonwebtoken::decode::<Claims<Self>>(token, &secret.decoding_key, &secret.validation)?;
        Ok(data.claims.data)
    }

    /// 持续时间默认半个月
    fn duration() -> usize {
        60 * 60 * 24 * 15
    }
}

/// 秘钥
#[derive(Clone)]
pub struct Secret {
    decoding_key: DecodingKey,
    encoding_key: EncodingKey,
    validation: Validation,
    header: Header,
}

impl Secret {
    pub fn new(secret: &str) -> Self {
        Self {
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            validation: Validation::default(),
            header: Header::default(),
        }
    }
}

pub trait Allow: Clone {
    fn allow(&self, uri: &str) -> bool;
}

impl<T: Fn(&str) -> bool + Clone> Allow for T {
    fn allow(&self, uri: &str) -> bool {
        self(uri)
    }
}

impl Allow for &str {
    fn allow(&self, uri: &str) -> bool {
        uri.contains(self)
    }
}

impl Allow for Arc<String> {
    fn allow(&self, uri: &str) -> bool {
        uri.contains(self.as_ref())
    }
}

impl<const N: usize> Allow for &'static [&str; N] {
    fn allow(&self, uri: &str) -> bool {
        self.iter().any(|a| uri.contains(a))
    }
}

impl Allow for Arc<Vec<String>> {
    fn allow(&self, uri: &str) -> bool {
        self.iter().any(|a| uri.contains(a))
    }
}
