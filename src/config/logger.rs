use std::{fs, fs::File, path::PathBuf};

use chrono::{DateTime, Local};
use serde::Deserialize;

use crate::middleware::logger;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Logger {
    pub path: PathBuf,
    pub name: String,
    pub file: bool,
    pub stdout: bool,
    pub delete: Option<usize>,
}

impl Logger {
    pub fn build(&self) -> logger::Logger {
        logger::Logger::new(self.clone())
    }

    pub fn delete_log(&self) -> anyhow::Result<()> {
        if let Some(n) = self.delete {
            let now = Local::now();
            for entry in fs::read_dir(&self.path)?.flatten() {
                let meta = entry.metadata()?;
                let date: DateTime<Local> = meta.created()?.into();
                if (now - date).num_days() > n as i64 {
                    fs::remove_file(entry.path())?
                }
            }
        };
        Ok(())
    }

    pub fn create_log_file(&self, time: &DateTime<Local>) -> File {
        fs::create_dir_all(&self.path).expect("自动创建日志文件父级目录失败");
        let name = time.format(&self.name).to_string();
        File::options()
            .create(true)
            .append(true)
            .write(true)
            .open(self.path.join(name))
            .expect("日志文件创建失败")
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self {
            path: "logs".into(),
            name: "%Y%m%d.log".into(),
            file: false,
            stdout: true,
            delete: None,
        }
    }
}
