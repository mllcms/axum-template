use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use axum::{
    async_trait,
    body::Body,
    http::{
        header::{CONTENT_LENGTH, CONTENT_TYPE},
        Request, StatusCode,
    },
    response::Response,
};
use axum_extra::headers::HeaderValue;
use tokio::{fs::File, io::AsyncReadExt};

use crate::{
    interceptor::{Intercept, Interceptor},
    resp,
};

#[derive(Debug, Clone)]
pub struct Html404 {
    path: Arc<PathBuf>,
}

impl Html404 {
    pub fn new<P: AsRef<Path>>(path: P) -> Interceptor<Self> {
        assert!(
            path.as_ref()
                .extension()
                .map(|m| m.eq_ignore_ascii_case("html"))
                .unwrap_or_default(),
            "不是 html 文件"
        );

        Interceptor::new(Self { path: path.as_ref().to_path_buf().into() })
    }
}

#[async_trait]
impl Intercept for Html404 {
    type Context = ();

    async fn before(&self, _: &mut Request<Body>) -> resp::Result<Self::Context> {
        Ok(())
    }

    async fn after(&self, _: Self::Context, res: &mut Response) {
        if res.status() == StatusCode::NOT_FOUND {
            if let Ok(mut file) = File::open(self.path.as_ref()).await {
                let mut buf = Vec::with_capacity(file.metadata().await.map(|m| m.len() as usize).unwrap_or(1024));
                if file.read_to_end(&mut buf).await.is_ok() {
                    let header = res.headers_mut();
                    header.insert(CONTENT_LENGTH, buf.len().into());
                    header.insert(CONTENT_TYPE, HeaderValue::from_str("text/html").unwrap());
                    *res.body_mut() = buf.into();
                }
            };
        }
    }
}
