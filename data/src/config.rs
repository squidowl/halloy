use std::path::PathBuf;
use std::{string, str};

use tokio_stream::wrappers::ReadDirStream;
use tokio_stream::StreamExt;

use rand::prelude::*;
use rand_chacha::ChaCha8Rng;

use serde::Deserialize;
use thiserror::Error;

pub use self::buffer::Buffer;
pub use self::channel::Channel;
pub use self::file_transfer::FileTransfer;
pub use self::keys::Keyboard;
pub use self::notification::Notifications;
pub use self::proxy::Proxy;
pub use self::server::Server;
pub use self::sidebar::Sidebar;

use crate::appearance::theme::Colors;
use crate::appearance::{self, Appearance};
use crate::audio::{self, Sound};
use crate::environment::config_dir;
use crate::server::Map as ServerMap;
use crate::{environment, Theme};

pub mod buffer;
pub mod channel;
pub mod file_transfer;
mod keys;
pub mod notification;
pub mod proxy;
pub mod server;
pub mod sidebar;

const CONFIG_TEMPLATE: &str = include_str!("../../config.toml");
const DEFAULT_THEME_NAME: &str = "ferra";

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub appearance: Appearance,
    pub servers: ServerMap,
    pub proxy: Option<Proxy>,
    pub font: Font,
    pub scale_factor: ScaleFactor,
    pub buffer: Buffer,
    pub sidebar: Sidebar,
    pub keyboard: Keyboard,
    pub notifications: Notifications<Sound>,
    pub file_transfer: FileTransfer,
    pub tooltips: bool,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct ScaleFactor(f64);

impl Default for ScaleFactor {
    fn default() -> Self {
        Self(1.0)
    }
}

impl From<f64> for ScaleFactor {
    fn from(value: f64) -> Self {
        ScaleFactor(value.clamp(0.1, 3.0))
    }
}

impl From<ScaleFactor> for f64 {
    fn from(value: ScaleFactor) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Font {
    pub family: Option<String>,
    pub size: Option<u8>,
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

    pub fn sounds_dir() -> PathBuf {
        let dir = Self::config_dir().join("sounds");

        if !dir.exists() {
            std::fs::create_dir_all(dir.as_path())
                .expect("expected permissions to create sounds folder");
        }

        dir
    }

    pub fn themes_dir() -> PathBuf {
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

    pub async fn load() -> Result<Self, Error> {
        use tokio::fs;

        #[derive(Deserialize, Debug)]
        #[serde(untagged)]
        pub enum ThemeKeys {
            Static(String),
            Dynamic { light: String, dark: String },
        }

        impl Default for ThemeKeys {
            fn default() -> Self {
                Self::Static(Default::default())
            }
        }

        impl ThemeKeys {
            pub fn keys(&self) -> (&str, Option<&str>) {
                match self {
                    ThemeKeys::Static(manual) => (manual, None),
                    ThemeKeys::Dynamic { light, dark } => (light, Some(dark)),
                }
            }
        }

        #[derive(Deserialize)]
        pub struct Configuration {
            #[serde(default)]
            pub theme: ThemeKeys,
            pub servers: ServerMap,
            pub proxy: Option<Proxy>,
            #[serde(default)]
            pub font: Font,
            #[serde(default)]
            pub scale_factor: ScaleFactor,
            #[serde(default)]
            pub buffer: Buffer,
            #[serde(default)]
            pub sidebar: Sidebar,
            #[serde(default)]
            pub keyboard: Keyboard,
            #[serde(default)]
            pub notifications: Notifications,
            #[serde(default)]
            pub file_transfer: FileTransfer,
            #[serde(default = "default_tooltip")]
            pub tooltips: bool,
        }

        let path = Self::path();
        if !path.try_exists()? {
            return Err(Error::ConfigMissing { has_yaml_config: has_yaml_config()? });
        }
        let content = fs::read_to_string(path)
            .await
            .map_err(|e| Error::LoadConfigFile(e.to_string()))?;

        let Configuration {
            theme,
            mut servers,
            font,
            proxy,
            scale_factor,
            buffer,
            sidebar,
            keyboard,
            notifications,
            file_transfer,
            tooltips,
        } = toml::from_str(content.as_ref()).map_err(|e| Error::Parse(e.to_string()))?;

        servers.read_passwords().await?;

        let loaded_notifications = notifications.load_sounds()?;

        let appearance = Self::load_appearance(theme.keys())
            .await
            .unwrap_or_default();

        Ok(Config {
            appearance,
            servers,
            font,
            proxy,
            scale_factor,
            buffer,
            sidebar,
            keyboard,
            notifications: loaded_notifications,
            file_transfer,
            tooltips,
        })
    }

    async fn load_appearance(theme_keys: (&str, Option<&str>)) -> Result<Appearance, Error> {
        use tokio::fs;

        #[derive(Deserialize)]
        #[serde(untagged)]
        pub enum Data {
            V1 {
                #[serde(rename = "name")]
                _name: String,
            },
            V2(Colors),
        }

        let read_entry = |entry: fs::DirEntry| async move {
            let content = fs::read_to_string(entry.path()).await.ok()?;

            let data: Data = toml::from_str(content.as_ref()).ok()?;
            let name = entry.path().file_stem()?.to_string_lossy().to_string();

            match data {
                Data::V1 { .. } => None,
                Data::V2(colors) => Some(Theme::new(name, colors)),
            }
        };

        let mut all = vec![];
        let mut first_theme = Theme::default();
        let mut second_theme = theme_keys.1.map(|_| Theme::default());
        let mut has_halloy_theme = false;

        let mut stream = ReadDirStream::new(fs::read_dir(Self::themes_dir()).await?);
        while let Some(entry) = stream.next().await {
            let Ok(entry) = entry else {
                continue;
            };

            let Some(file_name) = entry.file_name().to_str().map(String::from) else {
                continue;
            };

            if let Some(file_name) = file_name.strip_suffix(".toml") {
                if let Some(theme) = read_entry(entry).await {
                    if file_name == theme_keys.0 {
                        first_theme = theme.clone();
                    }

                    if Some(file_name) == theme_keys.1 {
                        second_theme = Some(theme.clone());
                    }

                    if file_name.to_lowercase() == DEFAULT_THEME_NAME {
                        has_halloy_theme = true;
                    }

                    all.push(theme);
                }
            }
        }

        if !has_halloy_theme {
            all.push(Theme::default());
        }

        let selected = if let Some(second_theme) = second_theme {
            appearance::Selected::dynamic(first_theme, second_theme)
        } else {
            appearance::Selected::specific(first_theme)
        };

        Ok(Appearance { selected, all })
    }

    pub fn create_initial_config() {
        // Checks if a config file is there
        let config_file = Self::path();
        if config_file.exists() {
            return;
        }

        // Generate a unique nick
        let rand_nick = random_nickname();

        // Replace placeholder nick with unique nick
        let config_string = CONFIG_TEMPLATE.replace("__NICKNAME__", rand_nick.as_str());
        let config_bytes = config_string.as_bytes();

        // Create configuration path.
        let config_path = Self::config_dir().join("config.toml");

        let _ = std::fs::write(config_path, config_bytes);
    }
}

pub fn random_nickname() -> String {
    let mut rng = ChaCha8Rng::from_entropy();
    random_nickname_with_seed(&mut rng)
}

pub fn random_nickname_with_seed<R: Rng>(rng: &mut R) -> String {
    let rand_digit: u16 = rng.gen_range(1000..=9999);
    let rand_nick = format!("halloy{rand_digit}");

    rand_nick
}

/// Has YAML configuration file.
fn has_yaml_config() -> Result<bool, Error> {
    Ok(config_dir().join("config.yaml").try_exists()?)
}

fn default_tooltip() -> bool {
    true
}

#[derive(Debug, Error, Clone)]
pub enum Error {
    #[error("config could not be read: {0}")]
    LoadConfigFile(String),
    #[error("command could not be run: {0}")]
    ExecutePasswordCommand(String),
    #[error("{0}")]
    Io(String),
    #[error("{0}")]
    Parse(String),
    #[error("UTF8 parsing error: {0}")]
    StrUtf8Error(#[from] str::Utf8Error),
    #[error("UTF8 parsing error: {0}")]
    StringUtf8Error(#[from] string::FromUtf8Error),
    #[error(transparent)]
    LoadSounds(#[from] audio::LoadError),
    #[error("Only one of password, password_file and password_command can be set.")]
    DuplicatePassword,
    #[error("Only one of nick_password, nick_password_file and nick_password_command can be set.")]
    DuplicateNickPassword,
    #[error("Exactly one of sasl.plain.password, sasl.plain.password_file or sasl.plain.password_command must be set.")]
    DuplicateSaslPassword,
    #[error("Config does not exist")]
    ConfigMissing { has_yaml_config: bool },
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}
