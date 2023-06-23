use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use serde::Deserialize;
use thiserror::Error;

use crate::palette::Palette;
use crate::{environment, pane, server};

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub palette: Palette,
    pub servers: server::Map,
    /// Default settings when creating a new pane
    #[serde(default)]
    pub new_pane: pane::Settings,
}

impl Config {
    pub fn config_dir() -> Result<PathBuf, Error> {
        let dir = environment::config_dir()
            .ok_or(Error::DirectoryNotFound)?
            .join("halloy");

        if !dir.exists() {
            std::fs::create_dir(dir.as_path()).map_err(|_| Error::DirectoryCreation)?;
        }

        Ok(dir)
    }

    fn path() -> Result<PathBuf, Error> {
        Ok(Self::config_dir()?.join("config.yaml"))
    }

    pub fn load() -> Result<Self, Error> {
        let path = Self::path()?;
        let file = File::open(path).map_err(|e| Error::Read(e.to_string()))?;

        serde_yaml::from_reader(BufReader::new(file)).map_err(|e| Error::Parse(e.to_string()))
    }
}

#[derive(Debug, Error, Clone)]
pub enum Error {
    #[error("config directory could not be found")]
    DirectoryNotFound,
    #[error("config directory could not be created")]
    DirectoryCreation,
    #[error("config could not be read: {0}")]
    Read(String),
    #[error("config could not be parsed: {0}")]
    Parse(String),
}
