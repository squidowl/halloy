use std::cmp::Ordering;
use std::path::PathBuf;
use std::{fs, io};

use jiff::tz::TimeZone;
use jiff::{Timestamp, Zoned};
use serde::{Deserialize, Serialize};

use crate::config::logs::LevelFilter;
use crate::{config, environment};

pub fn file(timestamp: config::logs::Timestamp) -> Result<fs::File, Error> {
    let zoned = match timestamp {
        config::logs::Timestamp::Local => Zoned::now(),
        config::logs::Timestamp::Utc => {
            Zoned::now().with_time_zone(TimeZone::UTC)
        }
    };
    let path = dir()?.join(
        Zoned::strftime(&zoned, "halloy.%Y-%m-%d-%H-%M-%S.log").to_string(),
    );

    Ok(fs::OpenOptions::new()
        .write(true)
        .create(true)
        .append(false)
        .truncate(true)
        .open(path)?)
}

fn dir() -> Result<PathBuf, Error> {
    let dir = environment::data_dir().join("logs");

    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }

    Ok(dir)
}

pub fn clear(number_of_logs_to_keep: usize) {
    if let Ok(dir) = dir() {
        for (index, dir_entry) in walkdir::WalkDir::new(dir)
            .max_depth(1)
            .sort_by(|a, b| b.file_name().cmp(a.file_name()))
            .into_iter()
            .filter_map(Result::ok)
            .filter(|dir_entry| {
                dir_entry.file_type().is_file()
                    && dir_entry.file_name().to_str().is_some_and(|file_name| {
                        file_name.starts_with("halloy.")
                            && file_name.ends_with(".log")
                    })
            })
            .enumerate()
        {
            if index >= number_of_logs_to_keep {
                let _ = fs::remove_file(dir_entry.path());
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Record {
    pub timestamp: Timestamp,
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
