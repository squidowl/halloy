use std::path::PathBuf;
use std::{str, string};

use iced_core::font;
use indexmap::IndexMap;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Deserializer};
use thiserror::Error;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReadDirStream;

pub use self::actions::Actions;
pub use self::buffer::Buffer;
pub use self::context_menu::ContextMenu;
pub use self::ctcp::Ctcp;
pub use self::file_transfer::FileTransfer;
pub use self::highlights::Highlights;
pub use self::keys::Keyboard;
pub use self::logs::Logs;
pub use self::notification::Notifications;
pub use self::pane::Pane;
pub use self::platform_specific::PlatformSpecific;
pub use self::preview::Preview;
pub use self::proxy::Proxy;
pub use self::server::Server;
pub use self::sidebar::Sidebar;
pub use self::wgpu::Wgpu;
use crate::appearance::theme::Styles;
use crate::appearance::{self, Appearance};
use crate::audio::{self};
use crate::serde::{
    deserialize_positive_float_maybe, deserialize_positive_integer_maybe,
};
use crate::server::{ConfigMap as ServerMap, ServerName};
use crate::{Theme, environment};

pub mod actions;
pub mod buffer;
pub mod context_menu;
pub mod ctcp;
pub mod file_transfer;
pub mod highlights;
pub mod inclusivities;
pub mod keys;
pub mod logs;
pub mod notification;
pub mod pane;
pub mod platform_specific;
pub mod preview;
pub mod proxy;
pub mod server;
pub mod sidebar;
pub mod wgpu;

const CONFIG_TEMPLATE: &str = include_str!("../../config.toml");
const DEFAULT_THEME_NAME: &str = "ferra";

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub appearance: Appearance,
    pub context_menu: ContextMenu,
    pub servers: ServerMap,
    pub proxy: Option<Proxy>,
    pub font: Font,
    pub scale_factor: ScaleFactor,
    pub buffer: Buffer,
    pub pane: Pane,
    pub sidebar: Sidebar,
    pub keyboard: Keyboard,
    pub notifications: Notifications,
    pub file_transfer: FileTransfer,
    pub tooltips: bool,
    pub preview: Preview,
    pub highlights: Highlights,
    pub actions: Actions,
    pub ctcp: Ctcp,
    pub logs: Logs,
    pub platform_specific: PlatformSpecific,
    pub wgpu: Wgpu,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct ScaleFactor(f32);

impl Default for ScaleFactor {
    fn default() -> Self {
        Self(1.0)
    }
}

impl From<f32> for ScaleFactor {
    fn from(value: f32) -> Self {
        ScaleFactor(value.clamp(0.1, 3.0))
    }
}

impl From<ScaleFactor> for f32 {
    fn from(value: ScaleFactor) -> Self {
        value.0
    }
}

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(default)]
pub struct Scrollbar {
    /// Width of the scrollbar.
    pub width: u32,
    /// Width of the scrollbar scroller.
    pub scroller_width: u32,
}

impl Default for Scrollbar {
    fn default() -> Self {
        Self {
            width: 5,
            scroller_width: 5,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Font {
    pub family: Option<String>,
    #[serde(deserialize_with = "deserialize_positive_integer_maybe")]
    pub size: Option<u8>,
    #[serde(deserialize_with = "deserialize_positive_float_maybe")]
    pub line_height: Option<f32>,
    #[serde(deserialize_with = "deserialize_font_weight_from_string")]
    pub weight: font::Weight,
    #[serde(deserialize_with = "deserialize_optional_font_weight_from_string")]
    #[serde(alias = "bold-weight")] // For backwards compatibility
    pub bold_weight: Option<font::Weight>,
    #[serde(deserialize_with = "deserialize_positive_integer_maybe")]
    pub only_emojis_size: Option<u8>,
}

impl Default for Font {
    fn default() -> Self {
        Self {
            family: None,
            size: None,
            line_height: None,
            weight: font::Weight::Normal,
            bold_weight: None,
            only_emojis_size: None,
        }
    }
}

fn deserialize_font_weight_from_string<'de, D>(
    deserializer: D,
) -> Result<font::Weight, D::Error>
where
    D: Deserializer<'de>,
{
    let string = String::deserialize(deserializer)?;

    match string.as_ref() {
        "thin" => Ok(font::Weight::Thin),
        "extra-light" => Ok(font::Weight::ExtraLight),
        "light" => Ok(font::Weight::Light),
        "normal" => Ok(font::Weight::Normal),
        "medium" => Ok(font::Weight::Medium),
        "semibold" => Ok(font::Weight::Semibold),
        "bold" => Ok(font::Weight::Bold),
        "extra-bold" => Ok(font::Weight::ExtraBold),
        "black" => Ok(font::Weight::Black),
        _ => Err(serde::de::Error::invalid_value(
            serde::de::Unexpected::Str(&string),
            &"one of \
              \"thin\", \
              \"extra-light\", \
              \"light\", \
              \"normal\", \
              \"medium\", \
              \"semibold\", \
              \"bold\", \
              \"extra-bold\", or \
              \"black\"",
        )),
    }
}

fn deserialize_optional_font_weight_from_string<'de, D>(
    deserializer: D,
) -> Result<Option<font::Weight>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Some(deserialize_font_weight_from_string(deserializer)?))
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

    pub fn path() -> PathBuf {
        Self::config_dir().join(environment::CONFIG_FILE_NAME)
    }

    pub async fn load() -> Result<Self, Error> {
        use tokio::fs;

        #[derive(Deserialize, Debug)]
        #[serde(untagged)]
        pub enum ThemeKeys {
            Static(ThemeValue),
            Dynamic { light: ThemeValue, dark: ThemeValue },
        }

        #[derive(Deserialize, Debug, Clone)]
        #[serde(untagged)]
        enum ThemeValue {
            Single(String),
            Multiple(Vec<String>),
        }

        impl ThemeValue {
            fn to_vec(&self) -> Vec<String> {
                match self {
                    ThemeValue::Single(s) => vec![s.clone()],
                    ThemeValue::Multiple(v) => v.clone(),
                }
            }
        }

        impl Default for ThemeKeys {
            fn default() -> Self {
                Self::Static(ThemeValue::Single(String::default()))
            }
        }

        impl ThemeKeys {
            pub fn keys(&self) -> (String, Option<String>) {
                use rand::prelude::*;
                use rand_chacha::ChaCha8Rng;

                let mut rng = ChaCha8Rng::from_rng(&mut rand::rng());

                match self {
                    ThemeKeys::Static(theme_value) => {
                        let themes = theme_value.to_vec();

                        if themes.is_empty() {
                            (String::default(), None)
                        } else {
                            let selected =
                                themes.choose(&mut rng).unwrap_or(&themes[0]);
                            (selected.clone(), None)
                        }
                    }
                    ThemeKeys::Dynamic { light, dark } => {
                        let light_themes = light.to_vec();
                        let dark_themes = dark.to_vec();

                        let light_selected = if light_themes.is_empty() {
                            String::default()
                        } else {
                            light_themes
                                .choose(&mut rng)
                                .unwrap_or(&light_themes[0])
                                .clone()
                        };

                        let dark_selected = if dark_themes.is_empty() {
                            None
                        } else {
                            Some(
                                dark_themes
                                    .choose(&mut rng)
                                    .unwrap_or(&dark_themes[0])
                                    .clone(),
                            )
                        };

                        (light_selected, dark_selected)
                    }
                }
            }
        }

        #[derive(Deserialize)]
        #[serde(default)]
        pub struct Configuration {
            pub theme: ThemeKeys,
            pub servers: IndexMap<ServerName, Server>,
            pub context_menu: ContextMenu,
            pub proxy: Option<Proxy>,
            pub font: Font,
            pub scale_factor: ScaleFactor,
            pub buffer: Buffer,
            pub pane: Pane,
            pub sidebar: Sidebar,
            pub keyboard: Keyboard,
            pub notifications: Notifications,
            pub file_transfer: FileTransfer,
            pub tooltips: bool,
            pub preview: Preview,
            pub highlights: Highlights,
            pub actions: Actions,
            pub ctcp: Ctcp,
            pub logs: Logs,
            pub platform_specific: PlatformSpecific,
            pub wgpu: Wgpu,
        }

        impl Default for Configuration {
            fn default() -> Self {
                Self {
                    theme: ThemeKeys::default(),
                    servers: IndexMap::<ServerName, Server>::default(),
                    context_menu: ContextMenu::default(),
                    proxy: None,
                    font: Font::default(),
                    scale_factor: ScaleFactor::default(),
                    buffer: Buffer::default(),
                    pane: Pane::default(),
                    sidebar: Sidebar::default(),
                    keyboard: Keyboard::default(),
                    notifications: Notifications::default(),
                    file_transfer: FileTransfer::default(),
                    tooltips: true,
                    preview: Preview::default(),
                    highlights: Highlights::default(),
                    actions: Actions::default(),
                    ctcp: Ctcp::default(),
                    logs: Logs::default(),
                    platform_specific: PlatformSpecific::default(),
                    wgpu: Wgpu::default(),
                }
            }
        }

        let path = Self::path();
        if !path.try_exists()? {
            return Err(Error::ConfigMissing);
        }
        let content = fs::read_to_string(path)
            .await
            .map_err(|e| Error::LoadConfigFile(e.to_string()))?;

        let config = toml::Deserializer::new(content.as_ref());

        let Configuration {
            theme,
            servers,
            context_menu,
            font,
            proxy,
            scale_factor,
            buffer,
            sidebar,
            keyboard,
            notifications,
            file_transfer,
            tooltips,
            preview,
            pane,
            highlights,
            actions,
            ctcp,
            logs,
            platform_specific,
            wgpu,
        } = serde_ignored::deserialize(config, |ignored| {
            log::warn!("[config.toml] Ignoring unknown setting: {ignored}");
        })
        .map_err(|e| Error::Parse(e.to_string()))?;

        let servers = ServerMap::new(servers).await?;

        let appearance = Self::load_appearance(theme.keys())
            .await
            .unwrap_or_default();

        Ok(Config {
            appearance,
            context_menu,
            servers,
            font,
            proxy,
            scale_factor,
            buffer,
            sidebar,
            keyboard,
            notifications,
            file_transfer,
            tooltips,
            preview,
            pane,
            highlights,
            actions,
            ctcp,
            logs,
            platform_specific,
            wgpu,
        })
    }

    async fn load_appearance(
        theme_keys: (String, Option<String>),
    ) -> Result<Appearance, Error> {
        use tokio::fs;

        #[derive(Deserialize)]
        #[serde(untagged)]
        pub enum Data {
            V1 {
                #[serde(rename = "name")]
                _name: String,
            },
            V2(Styles),
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
        let mut second_theme = theme_keys.1.clone().map(|_| Theme::default());
        let mut has_halloy_theme = false;

        let mut stream =
            ReadDirStream::new(fs::read_dir(Self::themes_dir()).await?);
        while let Some(entry) = stream.next().await {
            let Ok(entry) = entry else {
                continue;
            };

            let Some(file_name) = entry.file_name().to_str().map(String::from)
            else {
                continue;
            };

            if let Some(file_name) = file_name.strip_suffix(".toml")
                && let Some(theme) = read_entry(entry).await
            {
                if file_name == theme_keys.0 {
                    first_theme = theme.clone();
                }

                if Some(file_name) == theme_keys.1.as_deref() {
                    second_theme = Some(theme.clone());
                }

                if file_name.to_lowercase() == DEFAULT_THEME_NAME {
                    has_halloy_theme = true;
                }

                all.push(theme);
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

    pub fn load_logs() -> Option<Logs> {
        #[derive(Default, Deserialize)]
        #[serde(default)]
        pub struct Configuration {
            pub logs: Logs,
        }

        let path = Self::path();
        let content = std::fs::read_to_string(path).ok()?;

        let Configuration { logs } = toml::from_str(content.as_ref()).ok()?;

        Some(logs)
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
        let config_string =
            CONFIG_TEMPLATE.replace("__NICKNAME__", rand_nick.as_str());
        let config_bytes = config_string.as_bytes();

        // Create configuration path.
        let config_path = Self::config_dir().join("config.toml");

        let _ = std::fs::write(config_path, config_bytes);
    }
}

pub fn random_nickname() -> String {
    let mut rng = ChaCha8Rng::from_rng(&mut rand::rng());
    random_nickname_with_seed(&mut rng)
}

pub fn random_nickname_with_seed<R: Rng>(rng: &mut R) -> String {
    let rand_digit: u16 = rng.random_range(1000..=9999);
    let rand_nick = format!("halloy{rand_digit}");

    rand_nick
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
    #[error(
        "Only one of password, password_file and password_command can be set."
    )]
    DuplicatePassword,
    #[error(
        "Only one of nick_password, nick_password_file and nick_password_command can be set."
    )]
    DuplicateNickPassword,
    #[error(
        "Exactly one of sasl.plain.password, sasl.plain.password_file or sasl.plain.password_command must be set."
    )]
    DuplicateSaslPassword,
    #[error("Config does not exist")]
    ConfigMissing,
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}
