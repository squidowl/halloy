use std::fs::{self, File};
use std::io::BufReader;
use std::path::PathBuf;

use serde::Deserialize;
use thiserror::Error;

pub use self::buffer::Buffer;
pub use self::channel::Channel;
pub use self::dashboard::Dashboard;
pub use self::keys::Keys;
pub use self::notification::{Notification, Notifications};
pub use self::server::Server;
use crate::server::Map as ServerMap;
use crate::theme::Palette;
use crate::{environment, Theme};

pub mod buffer;
pub mod channel;
pub mod dashboard;
mod keys;
pub mod notification;
pub mod server;

const CONFIG_TEMPLATE: &[u8] = include_bytes!("../../config.yaml");
const DEFAULT_THEME_FILE_NAME: &str = "ferra.yaml";

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub themes: Themes,
    pub servers: ServerMap,
    pub font: Font,
    pub scale_factor: f64,
    pub buffer: Buffer,
    pub dashboard: Dashboard,
    pub keys: Keys,
    pub notifications: Notifications,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Font {
    pub family: Option<String>,
    pub size: Option<u8>,
}

#[derive(Debug, Clone)]
pub struct Themes {
    pub default: Theme,
    pub all: Vec<Theme>,
}

impl Default for Themes {
    fn default() -> Self {
        Self {
            default: Theme::default(),
            all: vec![Theme::default()],
        }
    }
}

impl Config {
    pub fn config_dir() -> PathBuf {
        let dir = environment::config_dir();

        if !dir.exists() {
            std::fs::create_dir_all(dir.as_path())
                .expect("expected permissions to create config folder");
        }

        dir
    }

    fn themes_dir() -> PathBuf {
        let dir = Self::config_dir().join("themes");

        if !dir.exists() {
            std::fs::create_dir_all(dir.as_path())
                .expect("expected permissions to create themes folder");
        }

        dir
    }

    fn path() -> PathBuf {
        Self::config_dir().join(environment::CONFIG_FILE_NAME)
    }

    pub fn load() -> Result<Self, Error> {
        #[derive(Deserialize)]
        pub struct Configuration {
            #[serde(default)]
            pub theme: String,
            pub servers: ServerMap,
            #[serde(default)]
            pub font: Font,
            #[serde(default = "default_scale_factor")]
            pub scale_factor: f64,
            #[serde(default, alias = "new_buffer")]
            pub buffer: Buffer,
            #[serde(default)]
            pub dashboard: Dashboard,
            #[serde(default)]
            pub keys: Keys,
            #[serde(default)]
            pub notifications: Notifications,
        }

        let path = Self::path();
        let file = File::open(path).map_err(|e| Error::Read(e.to_string()))?;

        let Configuration {
            theme,
            servers,
            font,
            scale_factor,
            buffer,
            dashboard,
            keys,
            notifications,
        } = serde_yaml::from_reader(BufReader::new(file))
            .map_err(|e| Error::Parse(e.to_string()))?;

        let themes = Self::load_themes(&theme).unwrap_or_default();

        Ok(Config {
            themes,
            servers,
            font,
            scale_factor,
            buffer,
            dashboard,
            keys,
            notifications,
        })
    }

    fn load_themes(default_key: &str) -> Result<Themes, Error> {
        #[derive(Deserialize)]
        pub struct Data {
            #[serde(default)]
            pub name: String,
            #[serde(default)]
            pub palette: Palette,
        }

        let read_entry = |entry: fs::DirEntry| {
            let content = fs::read(entry.path())?;

            let Data { name, palette } =
                serde_yaml::from_slice(&content).map_err(|e| Error::Parse(e.to_string()))?;

            Ok::<Theme, Error>(Theme::new(name, &palette))
        };

        let mut all = vec![];
        let mut default = Theme::default();
        let mut has_halloy_theme = false;

        for entry in fs::read_dir(Self::themes_dir())? {
            let Ok(entry) = entry else {
                continue;
            };

            let Some(file_name) = entry.file_name().to_str().map(String::from) else {
                continue;
            };

            if file_name.ends_with(".yaml") {
                if let Ok(theme) = read_entry(entry) {
                    if file_name.strip_suffix(".yaml").unwrap_or_default() == default_key {
                        default = theme.clone();
                    }
                    if file_name == DEFAULT_THEME_FILE_NAME {
                        has_halloy_theme = true;
                    }

                    all.push(theme);
                }
            }
        }

        if !has_halloy_theme {
            all.push(Theme::default());
        }

        Ok(Themes { default, all })
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
    const CONTENT: &[u8] = include_bytes!("../../assets/themes/ferra.yaml");

    // Create default theme file.
    let file = Config::themes_dir().join(DEFAULT_THEME_FILE_NAME);
    if !file.exists() {
        let _ = fs::write(file, CONTENT);
    }
}

fn default_scale_factor() -> f64 {
    1.0
}

#[derive(Debug, Error, Clone)]
pub enum Error {
    #[error("config could not be read: {0}")]
    Read(String),
    #[error("{0}")]
    Io(String),
    #[error("{0}")]
    Parse(String),
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}
