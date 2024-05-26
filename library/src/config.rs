use std::{fmt::Display, fs, io, path::Path, process};

use serde::Deserialize;

pub trait ConfigLoad: for<'de> Deserialize<'de> {
    fn from_toml<P: AsRef<Path>, D: Display>(path: P, err_prefix: D) -> Self {
        from_toml(path).unwrap_or_else(|_| {
            eprintln!("{err_prefix}");
            process::exit(0)
        })
    }
    fn try_from_toml<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        from_toml(path)
    }
    fn from_json<P: AsRef<Path>, D: Display>(path: P, err_prefix: D) -> Self {
        from_json(path).unwrap_or_else(|_| {
            eprintln!("{err_prefix}");
            process::exit(0)
        })
    }
    fn try_from_json<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        from_json(path)
    }
}

fn from_toml<P: AsRef<Path>, T: for<'de> Deserialize<'de>>(path: P) -> io::Result<T> {
    let data = fs::read_to_string(path)?;
    toml::from_str(&data).map_err(io::Error::other)
}

fn from_json<P: AsRef<Path>, T: for<'de> Deserialize<'de>>(path: P) -> io::Result<T> {
    let data = fs::read_to_string(path)?;
    serde_json::from_str(&data).map_err(io::Error::other)
}
