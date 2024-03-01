use std::{collections::HashMap, ops::DerefMut, str::FromStr};

use axum::async_trait;
use axum_extra::extract::{multipart::Field, Multipart};
use bytes::Bytes;
use derive_more::{Deref, DerefMut, Display};

use crate::tools::{resp, resp::Res};

#[async_trait]
pub trait MultiTake: Send {
    async fn take(&mut self, field: Field) -> resp::Result<()>;
    fn size(&self) -> usize {
        1
    }
}

#[derive(Default, Deref, DerefMut)]
pub struct MultiMap<'a>(pub HashMap<&'static str, (&'a mut dyn MultiTake, u8)>);

impl<'a> MultiMap<'a> {
    pub async fn load(&mut self, mut multi: Multipart) -> resp::Result<()> {
        let result = self.parse(&mut multi).await;
        while let Ok(Some(_f)) = multi.next_field().await {}
        result
    }

    pub async fn parse(&mut self, multi: &mut Multipart) -> resp::Result<()> {
        while let Some(field) = multi.next_field().await? {
            let item = field
                .name()
                .and_then(|a| self.get_mut(a))
                .ok_or(Res::msg(422, "字段有误"))?;
            item.0.take(field).await?;
            item.1 += 1;
        }

        for (k, (_, v)) in self.deref_mut() {
            if *v == 0 {
                return Err(Res::msg(422, format!("缺少 {k}")));
            }
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! multi_take {
    ($multi:expr => $($field:expr $(,)?)+) => {{
        let mut mp = $crate::tools::multipart::MultiMap::default();
        $(
            mp.insert(stringify!($field), (&mut $field as &mut dyn MultiTake, 0));
        )+
        mp.load($multi).await
    }};
}

#[derive(Debug, Default)]
pub struct MultiFile {
    pub name: String,
    pub bytes: Bytes,
    pub type_: String,
}

impl MultiFile {
    pub async fn update(&mut self, field: Field) -> resp::Result<()> {
        self.name = field.file_name().map(ToString::to_string).ok_or("获取文件名字失败")?;
        self.type_ = field
            .content_type()
            .map(ToString::to_string)
            .ok_or("获取文件类型失败")?;
        self.bytes = field.bytes().await?;
        Ok(())
    }
}

#[async_trait]
impl MultiTake for MultiFile {
    async fn take(&mut self, field: Field) -> resp::Result<()> {
        self.update(field).await
    }
}

#[async_trait]
impl MultiTake for Vec<MultiFile> {
    async fn take(&mut self, field: Field) -> resp::Result<()> {
        let mut mf = MultiFile::default();
        mf.update(field).await?;
        self.push(mf);
        Ok(())
    }
    fn size(&self) -> usize {
        usize::MAX
    }
}

#[derive(Debug, Default, Deref, DerefMut, Display)]
pub struct MultiInfo<T>(pub T);

impl<T: FromStr + Send> MultiInfo<T> {
    pub async fn update(&mut self, field: Field) -> resp::Result<()> {
        let data = field.text().await?;
        self.0 = data.trim().parse().map_err(|_| Res::err("类型解析失败"))?;
        Ok(())
    }
}

#[async_trait]
impl<T: FromStr + Send> MultiTake for MultiInfo<T> {
    async fn take(&mut self, field: Field) -> resp::Result<()> {
        self.update(field).await
    }
}

#[async_trait]
impl<T: FromStr + Send + Default> MultiTake for Vec<MultiInfo<T>> {
    async fn take(&mut self, field: Field) -> resp::Result<()> {
        let mut mi: MultiInfo<T> = MultiInfo::default();
        mi.update(field).await?;
        self.push(mi);
        Ok(())
    }
    fn size(&self) -> usize {
        usize::MAX
    }
}
