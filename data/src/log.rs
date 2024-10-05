use std::path::PathBuf;
use std::{fs, io};

use chrono::{DateTime, Utc};

use serde::{Deserialize, Serialize};

use crate::environment;

pub fn file() -> Result<fs::File, Error> {
    let path = path()?;

    Ok(fs::OpenOptions::new()
        .write(true)
        .create(true)
        .append(false)
        .truncate(true)
        .open(path)?)
}

fn path() -> Result<PathBuf, Error> {
    let parent = environment::data_dir();

    if !parent.exists() {
        fs::create_dir_all(&parent)?;
    }

    Ok(parent.join("halloy.log"))
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Record {
    pub timestamp: DateTime<Utc>,
    pub level: Level,
    pub message: String,
}

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Serialize, Deserialize, strum::Display,
)]
#[strum(serialize_all = "UPPERCASE")]
pub enum Level {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl From<log::Level> for Level {
    fn from(level: log::Level) -> Self {
        match level {
            log::Level::Error => Level::Error,
            log::Level::Warn => Level::Warn,
            log::Level::Info => Level::Info,
            log::Level::Debug => Level::Debug,
            log::Level::Trace => Level::Trace,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    SetLog(#[from] log::SetLoggerError),
    #[error(transparent)]
    ParseLevel(#[from] log::ParseLevelError),
}
