use axum::{async_trait, extract::FromRequestParts, http::request::Parts, response::Response};

use crate::jsonwebtoken::{auth_token, JwtToken};

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
