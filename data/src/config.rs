use std::fs::{self, File};
use std::io::BufReader;
use std::path::PathBuf;

use serde::Deserialize;
use thiserror::Error;

use crate::palette::Palette;
use crate::{buffer, environment, server};

const CONFIG_TEMPLATE: &[u8] = include_bytes!("../../config.yaml");

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub palette: Palette,
    pub servers: server::Map,
    #[serde(default)]
    pub font: Font,
    /// Default settings when creating a new buffer
    #[serde(default)]
    pub new_buffer: buffer::Settings,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Font {
    pub family: Option<String>,
    pub size: Option<u8>,
    // TODO: Do we make size, etc configurable and pass to Theme?
}

impl Config {
    pub fn config_dir() -> PathBuf {
        let dir = environment::config_dir().join("halloy");

        if !dir.exists() {
            std::fs::create_dir(dir.as_path())
                .expect("expected permissions to create config folder");
        }

        dir
    }

    fn path() -> PathBuf {
        Self::config_dir().join("config.yaml")
    }

    pub fn load() -> Result<Self, Error> {
        let path = Self::path();
        let file = File::open(path).map_err(|e| Error::Read(e.to_string()))?;

        serde_yaml::from_reader(BufReader::new(file)).map_err(|e| Error::Parse(e.to_string()))
    }

    pub fn create_template_config() {
        // Checks if a config file is there
        let config_file = Self::path();
        if config_file.exists() {
            return;
        }

        // Create template configuration file.
        let config_template_file = Self::config_dir().join("config.template.yaml");
        let _ = fs::write(config_template_file, CONFIG_TEMPLATE);
    }
}

#[derive(Debug, Error, Clone)]
pub enum Error {
    #[error("config could not be read: {0}")]
    Read(String),
    #[error("{0}")]
    Parse(String),
}
