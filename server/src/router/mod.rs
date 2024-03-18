mod user;

use axum::{routing::get, Router};
use library::{
    interceptor::{Download, Html404},
    logger::Logger,
};
use tower_http::services::ServeDir;

use crate::config::CONFIG;

pub async fn router() -> Router {
    Router::new()
        .merge(static_server())
        .route("/", get(|| async { "hello world" }))
        .nest("/user", user::router().await)
        .layer(Html404::new("static/404.html"))
        .layer(Logger::new(CONFIG.logger.clone()))
}

fn static_server() -> Router {
    let static_server = ServeDir::new("static");

    Router::new()
        .nest_service("/static", static_server)
        .layer(Download::interceptor())
}
