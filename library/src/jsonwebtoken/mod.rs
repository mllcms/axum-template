//! # 使用
//! 使用 [`Jwt`] 提取可以不依赖 [`JwtAuth`] 拿不到数据时会自己解析
//!
//! 使用 [`axum::extract::Extension`] 提取时必需使用 [`JwtAuth`]
//!
//! ```rust,ignore
//! #[derive(Debug, Clone, Serialize, Deserialize, Validate)]
//! struct User {
//!     name: String,
//!     phone: String,
//! }
//!
//! impl JwtToken for User {
//!     fn secret() -> &'static Secret {
//!         &CONFIG.jwt.secret
//!     }
//! }
//!
//! async fn login(VJson(user): VJson<User>) -> Resp<String> {
//!     match user.encode() {
//!         Ok(token) => resolve!(201 => token, "登录成功"),
//!         Err(err) => reject!(400, "登录失败: {err}"),
//!     }
//! }
//!
//! async fn info(Jwt(user): Jwt<User>) -> Resp<User> {
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

crate::re_export! {
    mod extractor;
    mod middleware;
}

use axum::{
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use axum_extra::headers::{authorization::Bearer, Authorization, HeaderMapExt};
use chrono::Local;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::res;

fn auth_token<T: JwtToken>(header: &HeaderMap) -> Result<T, Response> {
    let auth = header
        .typed_get::<Authorization<Bearer>>()
        .ok_or(res!(401, "身份认证失败: 请求未携带有效token").into_response())?;
    T::decode(auth.token()).map_err(|err| res!(401, "身份认证失败: {err}").into_response())
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
