//! ```rust,ignore
//! async fn demo() -> Resp<String> {
//!     let data = /*handle*/.map_err(|err|res!(400,"{err}"))?;
//!
//!     if let Err(err) = /*handle*/ {
//!        reject(400, "{err}")
//!     }else {
//!        resolve!(200 => data, "ok")
//!     }
//! }
//! ```

use std::fmt::Display;

use axum::response::{IntoResponse, Response};
use serde::Serialize;

pub type Resp<T> = Result<Res<T>, Res<()>>;

#[derive(Debug, Clone, Serialize)]
pub struct Res<T: Serialize = ()> {
    pub code: u16,
    pub message: String,
    pub data: T,
}

impl<T: Serialize> Res<T> {
    pub fn new(code: u16, msg: impl Display, data: impl Into<T>) -> Self {
        Self { code, message: msg.to_string(), data: data.into() }
    }
}

impl<T: Serialize> IntoResponse for Res<T> {
    fn into_response(self) -> Response {
        Response::builder()
            .status(self.code)
            .header("Content-type", "application/json")
            .body(serde_json::to_vec(&self).unwrap().into())
            .unwrap()
    }
}

impl<F: Display> From<F> for Res {
    fn from(value: F) -> Self {
        Self::new(400, value, ())
    }
}

#[macro_export]
macro_rules! res {
    ($code:expr, $($msg:tt)+) => {
        Res::new($code, format!($($msg)+), ()) as Res
    };
    ($code:expr => $data:expr, $($msg:tt)+) => {
        Res::new($code, format!($($msg)+), $data)
    };
}

#[macro_export]
macro_rules! reject {
    ($($t:tt)*) => {
        Err($crate::res!($($t)*))
    };
}

#[macro_export]
macro_rules! resolve {
    ($($t:tt)*) => {
        Ok($crate::res!($($t)*))
    };
}
