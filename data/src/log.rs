use std::path::PathBuf;
use std::{fs, io};

pub use log::*;

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
    let data_dir = dirs_next::data_dir().ok_or(Error::ResolvableDataDir)?;

    let parent = data_dir.join("halloy");

    if !parent.exists() {
        fs::create_dir_all(&parent)?;
    }

    Ok(parent.join("halloy.log"))
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("can't resolve data directory")]
    ResolvableDataDir,
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    SetLog(#[from] log::SetLoggerError),
    #[error(transparent)]
    ParseLevel(#[from] log::ParseLevelError),
}
