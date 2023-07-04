use std::fs::{self, File};
use std::io::BufReader;
use std::path::PathBuf;

use serde::Deserialize;
use thiserror::Error;

use crate::palette::Palette;
use crate::{buffer, dashboard, environment, server};

const CONFIG_TEMPLATE: &[u8] = include_bytes!("../../config.yaml");
const DEFAULT_THEME: (&str, &[u8]) = ("halloy", include_bytes!("../../assets/themes/halloy.yaml"));

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub palette: Palette,
    pub servers: server::Map,
    pub font: Font,
    /// Default settings when creating a new buffer
    pub new_buffer: buffer::Settings,
    pub dashboard: dashboard::Config,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Font {
    pub family: Option<String>,
    pub size: Option<u8>,
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

    fn themes_dir() -> PathBuf {
        let dir = Self::config_dir().join("themes");

        if !dir.exists() {
            std::fs::create_dir(dir.as_path())
                .expect("expected permissions to create themes folder");
        }

        dir
    }

    fn path() -> PathBuf {
        Self::config_dir().join("config.yaml")
    }

    pub fn load() -> Result<Self, Error> {
        #[derive(Deserialize)]
        pub struct Configuration {
            #[serde(default)]
            pub theme: String,
            pub servers: server::Map,
            #[serde(default)]
            pub font: Font,
            /// Default settings when creating a new buffer
            #[serde(default)]
            pub new_buffer: buffer::Settings,
            #[serde(default)]
            pub dashboard: dashboard::Config,
        }

        let path = Self::path();
        let file = File::open(path).map_err(|e| Error::Read(e.to_string()))?;

        let Configuration {
            theme,
            servers,
            font,
            new_buffer,
            dashboard,
        } = serde_yaml::from_reader(BufReader::new(file))
            .map_err(|e| Error::Parse(e.to_string()))?;

        // If theme fails to load, use default Palette (Halloy theme)
        let palette = Self::load_theme(&theme).unwrap_or_default();

        Ok(Config {
            palette,
            servers,
            font,
            new_buffer,
            dashboard,
        })
    }

    fn load_theme(theme: &str) -> Result<Palette, Error> {
        #[derive(Deserialize)]
        pub struct Theme {
            #[serde(default)]
            pub name: String,
            #[serde(default)]
            pub palette: Palette,
        }

        let path = Self::themes_dir().join(format!("{theme}.yaml"));
        let file = File::open(path).map_err(|e| Error::Read(e.to_string()))?;
        let Theme { palette, .. } = serde_yaml::from_reader(BufReader::new(file))
            .map_err(|e| Error::Parse(e.to_string()))?;

        Ok(palette)
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

pub fn create_themes_dir() {
    // Create default theme file.
    let (theme, content) = DEFAULT_THEME;
    let file = Config::themes_dir().join(format!("{theme}.yaml"));
    if !file.exists() {
        let _ = fs::write(file, content);
    }
}

#[derive(Debug, Error, Clone)]
pub enum Error {
    #[error("config could not be read: {0}")]
    Read(String),
    #[error("{0}")]
    Parse(String),
}
