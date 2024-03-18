use axum::{
    routing::{get, post},
    Router,
};
use library::jsonwebtoken::JwtAuth;

use crate::{api::user, auth::jwt};

pub async fn router() -> Router {
    Router::new()
        .route("/login", post(user::login))
        .route("/info", get(user::get_info).put(user::put_info))
        .layer(JwtAuth::<jwt::User, _>::new("/login"))
}
