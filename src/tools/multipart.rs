//! # Examples
//!
//! ```rust,ignore
//! async fn demo(multi: Multipart) {
//!     // 通过类型构建提取器
//!     let mut title = take!(String);
//!     // 通过值构建提取器
//!     let mut status = take!(false);
//!     // 限制大小
//!     let mut video = take!(MultiFile, limit = 0..GB);
//!     // 限制数量和类型
//!     let mut images = take!(MultiFiles, count = 1..10, ct = is_image);
//!     // 开始提取 multipart/form-data 表单字段和变量同名
//!     multi_take!(multi => title, status, video, images)?;
//!     println!("title:{} status:{}", title.value, status.value);
//!     for item in images.iter() {
//!         println!("{item:?}")
//!     }
//! }
//! ```

use std::{
    collections::HashMap,
    fmt::{Debug, Formatter, Write},
    ops::{DerefMut, Range},
    str::FromStr,
};

use anyhow::anyhow;
use axum::{async_trait, extract::DefaultBodyLimit};
use axum_extra::extract::{multipart::Field, Multipart};
use bytes::Bytes;
use derive_more::{Deref, DerefMut};
use tower::{
    layer::util::{Identity, Stack},
    ServiceBuilder,
};
use tower_http::limit::RequestBodyLimitLayer;

use crate::tools::{resp, resp::Res, unit::*};

/// 默认 limit 0KB..5MB
///
/// 默认 count 1..1
#[derive(Debug, Deref, DerefMut)]
pub struct Take<T: Default + MultiTake> {
    #[deref]
    #[deref_mut]
    pub value: T,
    /// 大小范围
    pub limit: Range<u64>,
    /// 数量范围 更新或追加(Vec)
    pub count: Range<u64>,
    /// 校验 content-type
    pub ct: fn(Option<&str>) -> bool,
    /// 接收的数量
    index: u64,
}

impl<T: Default + MultiTake> Take<T> {
    pub fn value(value: T) -> Self {
        Self { value, ..Self::default() }
    }
}

#[macro_export]
macro_rules! take {
    ($t:ty) => {
        $crate::tools::multipart::Take::<$t>::default()
    };
    ($t:ty, $($k:ident = $v:expr $(,)?)*) => {{
        let mut mv = $crate::tools::multipart::Take::<$t>::default();
        $(mv.$k = $v;)*
        mv
    }};
    ($value:expr) => {
        $crate::tools::multipart::Take::value($value)
    };
    ($value:expr, $($k:ident = $v:expr $(,)?)*) => {{
        let mut mv = $crate::tools::multipart::Take::value($value);
        $(mv.$k = $v;)*
        mv
    }};
}

impl<T: Default + MultiTake> Default for Take<T> {
    fn default() -> Self {
        fn always_true(_: Option<&str>) -> bool {
            true
        }
        Self {
            ct: always_true,
            value: T::default(),
            limit: 0..5 * MB,
            count: 1..1,
            index: 0,
        }
    }
}

#[async_trait]
pub trait MultiExtract: Send {
    async fn extract(&mut self, field: Field) -> anyhow::Result<()>;
    fn verify(&self) -> anyhow::Result<()>;
}

#[async_trait]
impl<T: Default + MultiTake> MultiExtract for Take<T> {
    async fn extract(&mut self, field: Field) -> anyhow::Result<()> {
        if !(self.ct)(field.content_type()) {
            return Err(anyhow!("类型不匹配"));
        }

        let size = self.value.take(field).await?;
        if !self.limit.contains(&size) {
            let (start, end) = (unit(self.limit.start), unit(self.limit.end));
            return Err(anyhow!("大小{start}-{end}"));
        }

        self.index += 1;
        let end = self.count.end;
        if self.index > end {
            return Err(anyhow!("不大于{end}个"));
        }
        Ok(())
    }

    fn verify(&self) -> anyhow::Result<()> {
        let start = self.count.start;
        let end = self.count.end;
        if start == end {
            if self.index != start {
                return Err(anyhow!("必须有{start}个"));
            }
        } else if !self.count.contains(&self.index) {
            return Err(anyhow!("至少有{start}个"));
        }
        Ok(())
    }
}

#[async_trait]
pub trait MultiTake: Send {
    async fn take(&mut self, field: Field) -> anyhow::Result<u64>;
}

#[async_trait]
impl<T: FromStr + Send> MultiTake for T {
    async fn take(&mut self, field: Field) -> anyhow::Result<u64> {
        let data = field.text().await?;
        let size = data.len();
        *self = data.trim().parse().map_err(|_| anyhow!("类型不匹配"))?;
        Ok(size as u64)
    }
}

#[derive(Default)]
pub struct MultiFile {
    pub name: String,
    pub bytes: Bytes,
    pub type_: String,
}

impl Debug for MultiFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultiFile")
            .field("name", &self.name)
            .field("type", &self.type_)
            .finish()
    }
}

#[async_trait]
impl MultiTake for MultiFile {
    async fn take(&mut self, field: Field) -> anyhow::Result<u64> {
        self.name = field
            .file_name()
            .map(ToString::to_string)
            .ok_or(anyhow!("获取文件名字失败"))?;
        self.type_ = field
            .content_type()
            .map(ToString::to_string)
            .ok_or(anyhow!("获取文件类型失败"))?;
        self.bytes = field.bytes().await?;
        Ok(self.bytes.len() as u64)
    }
}

#[derive(Debug, Default, Deref, DerefMut)]
pub struct MultiFiles(pub Vec<MultiFile>);

#[async_trait]
impl MultiTake for MultiFiles {
    async fn take(&mut self, field: Field) -> anyhow::Result<u64> {
        let mut mf = MultiFile::default();
        let size = mf.take(field).await?;
        self.push(mf);
        Ok(size)
    }
}

#[derive(Default, Deref, DerefMut)]
pub struct MultiMap<'a>(pub HashMap<&'static str, &'a mut dyn MultiExtract>);

impl<'a> MultiMap<'a> {
    pub async fn load(&mut self, mut multi: Multipart) -> resp::Result<()> {
        let result = self.parse(&mut multi).await;
        while let Ok(Some(_)) = multi.next_field().await {}
        result
    }

    pub async fn parse(&mut self, multi: &mut Multipart) -> resp::Result<()> {
        while let Some(field) = multi.next_field().await? {
            let key = field.name().ok_or(Res::msg(422, "获取字段名失败"))?;
            let name = key.to_string();
            let value = self.get_mut(key).ok_or(Res::msg(422, format!("未知字段 {key}")))?;
            value
                .extract(field)
                .await
                .map_err(|err| Res::msg(422, format!("数据验证失败: {name}<{err}>")))?;
        }

        let mut msg = String::from("数据验证失败: ");
        for (k, v) in self.deref_mut() {
            if let Err(err) = v.verify() {
                write!(msg, "{k}<{err}>; ").unwrap();
            }
        }
        msg.pop();
        if msg.len() > 20 {
            return Err(Res::msg(422, msg));
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! multi_take {
    ($multi:expr => $($field:expr $(,)?)+) => {{
        use $crate::tools::multipart::MultiExtract;
        let mut mp = $crate::tools::multipart::MultiMap::default();
        $(
            mp.insert(stringify!($field), &mut $field as &mut dyn MultiExtract);
        )+
        mp.load($multi).await
    }};
}

pub fn limit_layer(max: u64) -> ServiceBuilder<Stack<RequestBodyLimitLayer, Stack<DefaultBodyLimit, Identity>>> {
    ServiceBuilder::new()
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(max as usize))
}
