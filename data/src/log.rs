use std::cmp::Ordering;
use std::path::PathBuf;
use std::{fs, io};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::config::logs::LevelFilter;
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
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Debug,
    Hash,
    Serialize,
    Deserialize,
    strum::Display,
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

impl std::cmp::PartialOrd<LevelFilter> for Level {
    fn partial_cmp(&self, other: &LevelFilter) -> Option<Ordering> {
        Some(match self {
            Level::Error => match other {
                LevelFilter::Off => Ordering::Greater,
                LevelFilter::Error => Ordering::Equal,
                LevelFilter::Warn
                | LevelFilter::Info
                | LevelFilter::Debug
                | LevelFilter::Trace => Ordering::Less,
            },
            Level::Warn => match other {
                LevelFilter::Off | LevelFilter::Error => Ordering::Greater,
                LevelFilter::Warn => Ordering::Equal,
                LevelFilter::Info | LevelFilter::Debug | LevelFilter::Trace => {
                    Ordering::Less
                }
            },
            Level::Info => match other {
                LevelFilter::Off | LevelFilter::Error | LevelFilter::Warn => {
                    Ordering::Greater
                }
                LevelFilter::Info => Ordering::Equal,
                LevelFilter::Debug | LevelFilter::Trace => Ordering::Less,
            },
            Level::Debug => match other {
                LevelFilter::Off
                | LevelFilter::Error
                | LevelFilter::Warn
                | LevelFilter::Info => Ordering::Greater,
                LevelFilter::Debug => Ordering::Equal,
                LevelFilter::Trace => Ordering::Less,
            },
            Level::Trace => match other {
                LevelFilter::Off
                | LevelFilter::Error
                | LevelFilter::Warn
                | LevelFilter::Info
                | LevelFilter::Debug => Ordering::Greater,
                LevelFilter::Trace => Ordering::Equal,
            },
        })
    }
}

impl std::cmp::PartialEq<LevelFilter> for Level {
    fn eq(&self, other: &LevelFilter) -> bool {
        match self {
            Level::Error => matches!(other, LevelFilter::Error),
            Level::Warn => matches!(other, LevelFilter::Warn),
            Level::Info => matches!(other, LevelFilter::Info),
            Level::Debug => matches!(other, LevelFilter::Debug),
            Level::Trace => matches!(other, LevelFilter::Trace),
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
