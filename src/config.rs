use serde::{Deserialize, Serialize};
use std::{fs::File, io::BufReader, path::PathBuf};
use thiserror::Error;

use crate::theme::Theme;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Config {
    pub theme: Theme,
}

impl Config {
    pub fn config_dir() -> Result<PathBuf, Error> {
        let mut dir = dirs_next::config_dir().ok_or(Error::DirectoryNotFound)?;
        dir.push("halloy");

        if !dir.exists() {
            let _ =
                std::fs::create_dir(dir.as_path()).map_err(|_| Error::DirectoryCreationError)?;
        }

        Ok(dir)
    }

    pub async fn save(self) -> Result<(), Error> {
        let mut config_dir = Self::config_dir()?;
        config_dir.push("config.yaml");

        let serialized = serde_yaml::to_string(&self).map_err(|_| Error::SerializationError)?;

        let _ = tokio::fs::write(config_dir, serialized)
            .await
            .map_err(|_| Error::WriteError)?;

        Ok(())
    }

    pub fn load() -> Option<Self> {
        let config_dir = Self::config_dir().ok()?;
        let file = File::open(config_dir).ok()?;
        let reader = BufReader::new(file);

        serde_yaml::from_reader(reader).ok()
    }
}

#[derive(Debug, Clone, Copy, Error)]
pub enum Error {
    #[error("config directory could not be found")]
    DirectoryNotFound,
    #[error("config directory could not be created")]
    DirectoryCreationError,
    #[error("config could not be serialized")]
    SerializationError,
    #[error("config file could not be written")]
    WriteError,
}
