use std::{
    fs,
    fs::File,
    io::Write,
    net::SocketAddr,
    path::Path,
    task::{Context, Poll},
};

use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{header::LOCATION, Request},
    response::Response,
};
use chrono::{DateTime, Local};
use color_string::{cs, fonts, Colored, Font::*};
use futures_util::future::BoxFuture;
use percent_encoding::percent_decode;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tower::{Layer, Service};

use crate::config;

#[derive(Clone)]
pub struct Logger {
    sender: UnboundedSender<LogMsg>,
}

impl Logger {
    pub fn new(config: config::Logger) -> Self {
        let mut time = Local::now();
        let mut file = config.file.then(|| config.create_log_file(&time));
        let (sender, mut rx) = unbounded_channel::<LogMsg>();

        // 单独线程 同步写入日志
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if config.stdout {
                    msg.stdout()
                }

                if let Some(file) = file.as_mut() {
                    // 切换日志文件
                    if time.date_naive() != msg.begin.date_naive() {
                        time = msg.begin;
                        *file = config.create_log_file(&time)
                    }
                    msg.file_out(file);
                    // 定期删除日志
                    if let Err(err) = config.delete_log() {
                        eprintln!("日志删除失败: {err}")
                    };
                }
            }
        });

        Self { sender }
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
    sender: UnboundedSender<LogMsg>,
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
                path = format!("{path} -> {}", percent_decode(p.as_bytes()).decode_utf8_lossy())
            }

            let msg = LogMsg {
                logo: "[AXUM]".into(),
                begin,
                end: Local::now(),
                status,
                ip,
                method,
                path,
                other: "".into(),
            };

            if let Err(err) = sender.send(msg) {
                eprintln!("Send 日志时出现错误 {err}")
            }
            Ok(response)
        })
    }
}

struct LogMsg {
    logo: String,
    begin: DateTime<Local>,
    end: DateTime<Local>,
    status: u16,
    ip: String,
    method: String,
    path: String,
    other: String,
}

impl LogMsg {
    fn stdout(&self) {
        let status = match self.status / 100 {
            2 => cs!(BgGreen; " {} ", self.status),
            3 => cs!(BgBlue; " {} ", self.status),
            4 | 5 => cs!(BgRed; " {} ", self.status),
            _ => cs!(BgYellow; " {} ", self.status),
        };

        let method = match self.method.as_str() {
            "GET" | "POST" => cs!(BgBlue; " {:<6} ", self.method),
            "DELETE" => cs!(BgRed; " {:<6} ", self.method),
            _ => cs!(BgYellow; " {:<6} ", self.method),
        };

        println!(
            "[{}] {} |{}| {:>6}ms | {} |{} {} {}",
            self.end.format("%Y-%m-%d %H:%M:%S").color(127, 132, 142),
            self.logo.fonts(fonts!(Bold, Yellow)),
            status,
            (self.end - self.begin).num_milliseconds(),
            cs!(Yellow; "{:<15}", self.ip),
            method,
            self.path,
            self.other
        );
    }

    fn file_out(&self, file: &mut File) {
        let msg = format!(
            "[{}] {} | {} | {:>6}ms | {:>15} | {:<6} {} {}\n",
            self.end.format("%Y-%m-%d %H:%M:%S"),
            self.logo,
            self.status,
            (self.end - self.begin).num_milliseconds(),
            self.ip,
            self.method,
            self.path,
            self.other
        );
        if let Err(err) = file.write_all(msg.as_bytes()) {
            println!("日志写入文件时出错 -> {err}")
        }
    }
}

pub fn create_log_file(path: String) -> File {
    let path = Path::new(&path);
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).expect("自动创建日志文件父级目录失败")
    }

    File::options()
        .create(true)
        .append(true)
        .write(true)
        .open(path)
        .expect("日志文件创建失败")
}
