use std::{fs, fs::File, io, io::Write, path::PathBuf};

use chrono::{DateTime, Local};
use color_string::{cs, Colored, Font::*};
use serde::Deserialize;

pub struct LogMsg {
    pub begin: DateTime<Local>,
    pub end: DateTime<Local>,
    pub method: String,
    pub path: String,
    pub status: u16,
    pub ip: String,
}

impl LogMsg {
    pub fn write(&self, config: &LoggerConfig, mut writer: impl Write, is_file: bool) -> io::Result<()> {
        let duration = (self.end - self.begin).to_std().unwrap_or_default();

        if config.color && !is_file {
            let status = match self.status / 100 {
                2 => BgGreen,
                3 => BgBlue,
                4 | 5 => BgRed,
                _ => BgYellow,
            };

            let method = match self.method.as_str() {
                "GET" => BgGreen,
                "POST" => BgBlue,
                "PATCH" | "PUT" => BgYellow,
                "DELETE" => BgRed,
                _ => BgPurple,
            };

            writeln!(
                writer,
                "[{}] {} │ {} │ {:>8.2?} │ {} │ {} {}",
                self.end.format(&config.time).color(127, 132, 142),
                cs!(Bold, Yellow => config.logo),
                cs!(status; " {} ",self.status),
                duration,
                cs!(Yellow; "{:<15}", self.ip),
                cs!(method; " {:<6} ",self.method),
                self.path,
            )
        } else {
            writeln!(
                writer,
                "[{}] {} │ {} │ {:>8.2?} │ {:>15} │ {:<6} {}",
                self.begin.format(&config.time),
                config.logo,
                self.status,
                duration,
                self.ip,
                self.method,
                self.path,
            )
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggerConfig {
    pub path: PathBuf,
    pub name: String,
    pub time: String,
    pub logo: String,
    pub file: bool,
    pub color: bool,
    pub stdout: bool,
    pub delete: Option<i64>,
}

impl LoggerConfig {
    pub fn delete_log_file(&self) -> anyhow::Result<()> {
        if let Some(n) = self.delete {
            let now = Local::now();
            for entry in fs::read_dir(&self.path)?.flatten() {
                let meta = entry.metadata()?;
                let created_at: DateTime<Local> = meta.created()?.into();
                if (now - created_at).num_days() >= n {
                    fs::remove_file(entry.path())?
                }
            }
        };
        Ok(())
    }

    /// 更新日志文件 删除过期文件
    pub fn update_log_file(&self, time: &DateTime<Local>) -> File {
        if let Err(err) = self.delete_log_file() {
            eprintln!("日志删除失败: {err}")
        }

        fs::create_dir_all(&self.path).expect("自动创建日志文件父级目录失败");
        let name = time.format(&self.name).to_string();
        File::options()
            .create(true)
            .append(true)
            .open(self.path.join(name))
            .expect("日志文件创建失败")
    }
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            path: "logs".into(),
            name: "%Y%m%d.log".into(),
            time: "%y-%m-%d %H:%M:%S".into(),
            logo: "[AXUM]".into(),
            file: false,
            color: true,
            stdout: true,
            delete: None,
        }
    }
}
