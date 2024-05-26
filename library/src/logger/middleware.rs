use std::{
    io,
    net::SocketAddr,
    sync::mpsc::{self, Sender},
    task::{Context, Poll},
    thread,
};

use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{header::LOCATION, Request},
    response::Response,
};
use chrono::Local;
use futures_util::future::BoxFuture;
use percent_encoding::percent_decode;
use tower::{Layer, Service};

use crate::logger::{LogMsg, LoggerConfig};

#[derive(Clone)]
pub struct Logger {
    sender: Sender<LogMsg>,
}

impl Logger {
    pub fn new(config: LoggerConfig) -> Self {
        let mut time = Local::now();
        let mut file = config.file.then(|| config.update_log_file(&time));
        let mut stdout = config.stdout.then(io::stdout);
        let (sender, rx) = mpsc::channel::<LogMsg>();

        thread::spawn(move || {
            // 单独线程 同步写入日志
            for msg in rx {
                if let Some(stdout) = stdout.as_mut() {
                    if let Err(err) = msg.write(&config, stdout, false) {
                        eprintln!("写入日志失败 -> {err}")
                    }
                }

                if let Some(file) = file.as_mut() {
                    // 更新日志文件
                    if time.date_naive() != msg.begin.date_naive() {
                        time = msg.begin;
                        *file = config.update_log_file(&time);
                    }
                    if let Err(err) = msg.write(&config, file, true) {
                        eprintln!("写入日志失败 -> {err}")
                    }
                }
            }
        });

        Self { sender }
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new(LoggerConfig::default())
    }
}

impl<S> Layer<S> for Logger {
    type Service = LoggerService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        LoggerService { inner, sender: self.sender.clone() }
    }
}

#[derive(Clone)]
pub struct LoggerService<S> {
    inner: S,
    sender: Sender<LogMsg>,
}

impl<S> Service<Request<Body>> for LoggerService<S>
where
    S: Service<Request<Body>, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        // 开始时间
        let begin = Local::now();
        // 请求方式
        let method = req.method().to_string();
        // 连接 ip
        let ip = match req.extensions().get::<ConnectInfo<SocketAddr>>() {
            Some(v) => v.0.ip().to_string(),
            None => panic!("Axum service 未配置 ConnectInfo<SocketAddr>"),
        };
        // 请求路径 解码为 utf-8
        let mut path = percent_decode(req.uri().path().as_bytes())
            .decode_utf8_lossy()
            .to_string();

        let sender = self.sender.clone();
        let future = self.inner.call(req);

        Box::pin(async move {
            let response: Self::Response = future.await?;
            // 状态码
            let status = response.status().as_u16();
            // 是否重定向
            if let Some(p) = response.headers().get(LOCATION) {
                path.push_str(" -> ");
                path.push_str(&percent_decode(p.as_bytes()).decode_utf8_lossy())
            }

            let msg = LogMsg { begin, end: Local::now(), status, ip, method, path };

            if let Err(err) = sender.send(msg) {
                eprintln!("Send 日志时出现错误 {err}")
            }
            Ok(response)
        })
    }
}
