use std::fmt::Write;

use axum::{
    async_trait,
    extract::{
        rejection::{BytesRejection, RawFormRejection},
        FromRequest, RawForm, Request,
    },
    http::HeaderMap,
    RequestExt,
};
use axum_extra::headers::{ContentType, HeaderMapExt};
use bytes::Bytes;
use derive_more::{Deref, DerefMut};
use once_cell::sync::Lazy;
use serde::Deserialize;
use validator::Validate;

use crate::{reject, res, tools::resp::Res};

/// 提取 Json 类型数据 并验证数据
#[must_use]
#[derive(Debug, Clone, Default, Deref, DerefMut)]
pub struct VJson<T: Validate>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for VJson<T>
where
    T: for<'de> Deserialize<'de> + Validate,
    S: Send + Sync,
{
    type Rejection = Res<()>;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        if !json_content_type(req.headers()) {
            return reject!(401, "请求头必须为 application/json");
        }

        let data = des_json(Bytes::from_request(req, state).await)?;
        Ok(VJson(data))
    }
}

/// 提取 Form 类型数据 并验证数据
#[must_use]
#[derive(Debug, Clone, Default, Deref, DerefMut)]
pub struct VForm<T: Validate>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for VForm<T>
where
    T: for<'de> Deserialize<'de> + Validate,
    S: Send + Sync,
{
    type Rejection = Res<()>;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let data = des_form(req.extract::<RawForm, _>().await)?;
        Ok(VForm(data))
    }
}

/// 提取 Json 或者 Form 类型数据 并验证数据
#[must_use]
#[derive(Debug, Clone, Default, Deref, DerefMut)]
pub struct VJsonOrForm<T: Validate>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for VJsonOrForm<T>
where
    T: for<'de> Deserialize<'de> + Validate,
    S: Send + Sync,
{
    type Rejection = Res<()>;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let data = if json_content_type(req.headers()) {
            des_json(Bytes::from_request(req, state).await)?
        } else {
            des_form(req.extract::<RawForm, _>().await)?
        };

        Ok(VJsonOrForm(data))
    }
}

/// 提取 Query 类型数据 并验证数据
#[must_use]
#[derive(Debug, Clone, Default, Deref, DerefMut)]
pub struct VQuery<T: Validate>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for VQuery<T>
where
    T: for<'de> Deserialize<'de> + Validate,
    S: Send + Sync,
{
    type Rejection = Res<()>;

    async fn from_request(req: Request, _: &S) -> Result<Self, Self::Rejection> {
        let uri = req.uri().query().unwrap_or_default();
        let data = serde_urlencoded::from_str(uri).map_err(|err| res!(422, "{err}"))?;

        validate(&data)?;
        Ok(VQuery(data))
    }
}

static JSON: Lazy<ContentType> = Lazy::new(ContentType::json);

/// 判断 json 请求头
pub fn json_content_type(headers: &HeaderMap) -> bool {
    headers.typed_get::<ContentType>().map(|t| t == *JSON).unwrap_or(false)
}

/// 返序列化 json
fn des_json<T>(data: Result<Bytes, BytesRejection>) -> Result<T, Res<()>>
where
    T: for<'de> Deserialize<'de> + Validate,
{
    let bytes = data.map_err(|err| res!(422, "{err}"))?;
    let data = serde_json::from_slice(&bytes).map_err(|err| res!(422, "{err}"))?;

    validate(&data)?;
    Ok(data)
}

/// 返序列化 form
fn des_form<T>(data: Result<RawForm, RawFormRejection>) -> Result<T, Res<()>>
where
    T: for<'de> Deserialize<'de> + Validate,
{
    let data = match data {
        Ok(RawForm(bytes)) => serde_urlencoded::from_bytes::<T>(&bytes)?,
        Err(_) => return Err(res!(422, "无法获取到表单数据")),
    };

    validate(&data)?;
    Ok(data)
}

/// 数据验证
pub fn validate(data: impl Validate) -> Result<(), Res<()>> {
    if let Err(err) = data.validate() {
        let mut msg = String::new();
        write!(msg, "数据验证失败: ").unwrap();
        for (key, value) in err.field_errors() {
            write!(msg, "{key}<").unwrap();
            value
                .iter()
                .map(|m| &m.code)
                .for_each(|field| write!(msg, "{field}, ").unwrap());
            msg.replace_range(msg.len() - 2.., ">; ")
        }
        msg.pop();
        return reject!(422, "{msg}");
    }
    Ok(())
}

#[test]
fn validate_t() {
    #[derive(Validate)]
    struct User {
        #[validate(email(code = "邮箱格式不正确"))]
        #[validate(length(min = 6, max = 30, code = "长度6-30"))]
        pub email: &'static str,
        #[validate(range(min = 0, max = 130, code = "年龄0-130"))]
        pub age: u16,
    }

    let user = User { email: "asd", age: 150 };
    println!("{}", serde_json::to_string(&validate(user).err().unwrap()).unwrap())
}
