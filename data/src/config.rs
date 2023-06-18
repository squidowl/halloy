use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::palette::Palette;
use crate::{channel, server};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub palette: Palette,
    pub servers: BTreeMap<String, server::Config>,
    #[serde(default)]
    pub channels: BTreeMap<String, BTreeMap<String, channel::Config>>,
    #[serde(default)]
    pub user_colors: UserColor,
    #[serde(skip)]
    pub error: Option<Error>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub enum UserColor {
    Solid,
    #[default]
    Unique,
}

impl UserColor {
    pub fn unique_colors(&self) -> bool {
        match self {
            UserColor::Solid => false,
            UserColor::Unique => true,
        }
    }
}

impl Config {
    pub fn config_dir() -> Result<PathBuf, Error> {
        let mut dir = dirs_next::config_dir().ok_or(Error::DirectoryNotFound)?;
        dir.push("halloy");

        if !dir.exists() {
            std::fs::create_dir(dir.as_path()).map_err(|_| Error::DirectoryCreation)?;
        }

        Ok(dir)
    }

    pub async fn save(self) -> Result<(), Error> {
        let mut config_dir = Self::config_dir()?;
        config_dir.push("config.yaml");

        let serialized =
            serde_yaml::to_string(&self).map_err(|error| Error::Serialize(error.to_string()))?;
        tokio::fs::write(config_dir, serialized)
            .await
            .map_err(|error| Error::Write(error.to_string()))?;

        Ok(())
    }

    pub fn load() -> Self {
        let config_dir = match Self::config_dir() {
            Ok(dir) => dir,
            Err(error) => {
                return Self {
                    error: Some(error),
                    ..Self::default()
                }
            }
        };

        let file = match File::open(config_dir.join("config.yaml")) {
            Ok(file) => file,
            Err(error) => {
                return Self {
                    error: Some(Error::Read(error.to_string())),
                    ..Self::default()
                }
            }
        };

        match serde_yaml::from_reader(BufReader::new(file)) {
            Ok::<Self, _>(config) => config,
            Err(error) => Self {
                error: Some(Error::Parse(error.to_string())),
                ..Self::default()
            },
        }
    }

    pub fn channel_config(&self, server: impl AsRef<str>, channel: &str) -> channel::Config {
        self.channels
            .get(server.as_ref())
            .and_then(|channels| channels.get(channel))
            .cloned()
            .unwrap_or_default()
    }

    pub fn channel_config_mut(
        &mut self,
        server: impl AsRef<str>,
        channel: &str,
    ) -> &mut channel::Config {
        let servers = self
            .channels
            .entry(server.as_ref().to_string())
            .or_insert(BTreeMap::new());

        let config = servers
            .entry(channel.to_string())
            .or_insert_with_key(|_| Default::default());

        config
    }
}

#[derive(Debug, Error, Clone)]
pub enum Error {
    #[error("config directory could not be found")]
    DirectoryNotFound,
    #[error("config directory could not be created")]
    DirectoryCreation,
    #[error("config could not be serialized: {0}")]
    Serialize(String),
    #[error("config could not be written: {0}")]
    Write(String),
    #[error("config could not be read: {0}")]
    Read(String),
    #[error("config could not be parsed: {0}")]
    Parse(String),
}
