use std::{fmt::Display, io};

use axum::{
    body::Body,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_extra::extract::multipart::MultipartError;
use serde::Serialize;

pub type Resp<T> = std::result::Result<Res<T>, Res<()>>;
pub type Result<T> = std::result::Result<T, Res<()>>;

#[derive(Debug, Clone, Serialize)]
pub struct Res<T: Serialize = ()> {
    pub code: u16,
    pub message: String,
    pub data: T,
}

impl Res<()> {
    pub fn msg(code: u16, msg: impl Display) -> Self {
        Self::new(code, msg, ())
    }
    pub fn err(msg: impl Display) -> Self {
        Self::new(400, msg, ())
    }
}

impl<T: Serialize> Res<T> {
    pub fn new(code: u16, msg: impl Display, data: impl Into<T>) -> Self {
        Self { code, message: msg.to_string(), data: data.into() }
    }

    pub fn data(code: StatusCode, data: impl Into<T>) -> Self {
        Self::new(code.as_u16(), code, data)
    }
}

impl<T: Serialize> IntoResponse for Res<T> {
    fn into_response(self) -> Response {
        let body = serde_json::to_string(&self).unwrap();
        Response::builder()
            .status(self.code)
            .header("Content-type", "application/json")
            .body(Body::new(body))
            .unwrap()
    }
}

macro_rules! res_from {
    ($($t:ty, $c:expr $(;)?)*) => {
        $(
            impl From<$t> for Res<()> {
                fn from(value: $t) -> Self {
                    Res::new($c, value, ())
                }
            }
        )*
    };

    ($($t:ty, $c:expr, $m:expr $(;)?)*) => {
    $(
        impl From<$t> for Res<()> {
            fn from(_: $t) -> Self {
                Res::new($c, $m, ())
            }
        }
    )*
    };
}

res_from!(
    &str, 400
    String, 400;
    io::Error, 400;
    MultipartError, 422
    serde::de::value::Error, 422;
);

// res_from!(MultipartError, 422, "提取数据失败");

impl From<StatusCode> for Res<()> {
    fn from(value: StatusCode) -> Self {
        Res::new(value.as_u16(), value.as_str(), ())
    }
}

impl<T: Serialize> From<(StatusCode, T)> for Res<T> {
    fn from((code, data): (StatusCode, T)) -> Self {
        Res::new(code.as_u16(), code.as_str(), data)
    }
}
