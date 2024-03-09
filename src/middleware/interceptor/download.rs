use axum::{
    async_trait,
    body::Body,
    http::{
        header::{CONTENT_DISPOSITION, CONTENT_TYPE},
        HeaderValue, Request,
    },
    response::Response,
};
use serde::{Deserialize, Serialize};

use crate::{
    middleware::interceptor::cors::{Intercept, Interceptor},
    tools::{parse_query, resp},
};

/// # Examples
/// ```rust,ignore
/// fn static_server() -> Router {
///     let static_server = ServeDir::new("static");
///     Router::new()
///         .nest_service("/static", static_server)
///         .layer(Download::interceptor())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Download {}

impl Download {
    pub fn interceptor() -> Interceptor<Self> {
        Interceptor::new(Self {})
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", tag = "type")]
enum Type {
    Download,
}

#[async_trait]
impl Intercept for Download {
    type Ctx = bool;

    async fn before(&self, req: &mut Request<Body>) -> resp::Result<Self::Ctx> {
        Ok(matches!(parse_query::<Type>(&req), Ok(Type::Download)))
    }

    async fn after(&self, ctx: Self::Ctx, res: &mut Response) {
        if ctx {
            let headers = res.headers_mut();
            let stream = HeaderValue::from_str("application/octet-stream").unwrap();
            let attachment = HeaderValue::from_str("attachment").unwrap();
            headers.insert(CONTENT_TYPE, stream);
            headers.insert(CONTENT_DISPOSITION, attachment);
        }
    }
}
