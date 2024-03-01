use std::{
    collections::HashMap,
    fmt::{Debug, Formatter, Write},
    ops::{DerefMut, Range},
    str::FromStr,
};

use anyhow::anyhow;
use axum::async_trait;
use axum_extra::extract::{multipart::Field, Multipart};
use bytes::Bytes;
use derive_more::{Deref, DerefMut};

use crate::tools::{resp, resp::Res};

pub const KB: u64 = 1 << 10;
pub const MB: u64 = 1 << 20;
pub const GB: u64 = 1 << 30;
pub const TB: u64 = 1 << 40;

pub const UNIT: [&str; 4] = ["KB", "MB", "GB", "TB"];

pub fn unit(n: u64) -> String {
    let mut n = n as f64;
    for s in UNIT {
        n /= 1024.0;
        if 1024.0 > n {
            return format!("{n:.1}{s}");
        }
    }
    format!("{n:.1}{}", UNIT[3])
}

#[test]
fn unit_t() {
    assert_eq!("1.0KB", unit(KB));
    assert_eq!("1.0MB", unit(MB));
    assert_eq!("1.0GB", unit(GB));
    assert_eq!("1.0TB", unit(TB));
    assert_eq!("1025.0TB", unit(1025 * TB));
    assert_eq!("1.1MB", unit(MB + 100 * KB));
}

/// 默认 limit 0KB..5MB
///
/// 默认 count 1..1
#[derive(Debug, Deref, DerefMut)]
pub struct MultiValue<T: Default + MultiTake> {
    #[deref]
    #[deref_mut]
    pub value: T,
    /// 接收的数量
    pub index: u64,
    /// 大小范围
    pub limit: Range<u64>,
    /// 数量范围 更新或追加(Vec)
    pub count: Range<u64>,
}

impl<T: Default + MultiTake> MultiValue<T> {
    pub fn new(value: T, limit: Range<u64>, count: Range<u64>) -> Self {
        Self { value, limit, count, index: 0 }
    }
    pub fn value(value: T) -> Self {
        Self { value, ..Self::default() }
    }
    pub fn limit(limit: Range<u64>) -> Self {
        Self { limit, ..Self::default() }
    }
    pub fn count(count: Range<u64>) -> Self {
        Self { count, ..Self::default() }
    }

    pub fn range(limit: Range<u64>, count: Range<u64>) -> Self {
        Self { limit, count, ..Self::default() }
    }
}

impl<T: Default + MultiTake> Default for MultiValue<T> {
    fn default() -> Self {
        Self::new(T::default(), 0..5 * MB, 1..1)
    }
}

#[async_trait]
pub trait MultiExtract: Send {
    async fn extract(&mut self, field: Field) -> anyhow::Result<()>;
    fn check(&mut self, size: u64) -> anyhow::Result<()>;
    fn verify(&self) -> anyhow::Result<()>;
}

#[async_trait]
impl<T: Default + MultiTake> MultiExtract for MultiValue<T> {
    async fn extract(&mut self, field: Field) -> anyhow::Result<()> {
        let size = self.value.take(field).await?;
        self.check(size)?;
        self.index += 1;
        Ok(())
    }
    fn check(&mut self, size: u64) -> anyhow::Result<()> {
        if !self.limit.contains(&size) {
            let (start, end) = (unit(self.limit.start), unit(self.limit.end));
            return Err(anyhow!("大小{start}-{end}"));
        }
        let end = self.count.end;
        if self.index > end {
            return Err(anyhow!("数量不能大于{end}个"));
        }
        Ok(())
    }

    fn verify(&self) -> anyhow::Result<()> {
        let start = self.count.start;
        let end = self.count.end;
        if start == end {
            if self.index != start {
                return Err(anyhow!("数量必须有{start}个"));
            }
        } else if !self.count.contains(&self.index) {
            return Err(anyhow!("数量范围{start}-{end}"));
        }
        Ok(())
    }
}

#[async_trait]
pub trait MultiTake: Send {
    async fn take(&mut self, field: Field) -> anyhow::Result<u64>;
}

#[async_trait]
impl<T: FromStr + Sync + Send> MultiTake for T {
    async fn take(&mut self, field: Field) -> anyhow::Result<u64> {
        let data = field.text().await?;
        let size = data.len();
        *self = data.trim().parse().map_err(|_| anyhow!("类型不匹配"))?;
        Ok(size as u64)
    }
}

#[derive(Default, Deref, DerefMut)]
pub struct Array<T: MultiTake>(Vec<T>);

impl<T: MultiTake + Debug> Debug for Array<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
#[async_trait]
impl<T: Default + MultiTake> MultiTake for Array<T> {
    async fn take(&mut self, field: Field) -> anyhow::Result<u64> {
        let mut item = T::default();
        let size = item.take(field).await?;
        self.push(item);
        Ok(size)
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

#[derive(Default, Deref, DerefMut)]
pub struct MultiMap<'a>(pub HashMap<&'static str, &'a mut dyn MultiExtract>);

impl<'a> MultiMap<'a> {
    pub async fn load(&mut self, mut multi: Multipart) -> resp::Result<()> {
        let result = self.parse(&mut multi).await;
        while let Ok(Some(_f)) = multi.next_field().await {}
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
                .map_err(|err| anyhow!("数据验证失败: {name}<{err}>"))?;
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
