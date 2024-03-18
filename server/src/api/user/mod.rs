use library::{
    jsonwebtoken::{Jwt, JwtToken},
    reject, resolve,
    resp::Resp,
    validator::VJson,
};

use crate::auth::jwt;

pub async fn login(VJson(user): VJson<jwt::User>) -> Resp<String> {
    match user.encode() {
        Ok(token) => resolve!(201 => token, "登录成功"),
        Err(err) => reject!(400, "登录失败: {err}"),
    }
}

pub async fn get_info(Jwt(user): Jwt<jwt::User>) -> Resp<jwt::User> {
    resolve!(200 => user, "获取用户信息成功")
}

pub async fn put_info(Jwt(user): Jwt<jwt::User>) -> Resp<jwt::User> {
    resolve!(200 => user, "修改用户信息成功")
}
