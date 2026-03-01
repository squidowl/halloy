use core::fmt;
use std::str::FromStr;

use chrono::Locale;
use serde::{Deserialize, Deserializer, Serialize};

use crate::serde::{
    deserialize_strftime_date, deserialize_strftime_date_maybe,
};
use crate::target::{self, Target};
use crate::{Server, channel, config, message};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Buffer {
    Upstream(Upstream),
    Internal(Internal),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Upstream {
    Server(Server),
    Channel(Server, target::Channel),
    Query(Server, target::Query),
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, strum::Display,
)]
pub enum Internal {
    #[strum(serialize = "File Transfers")]
    FileTransfers,
    Scripts,
    Logs,
    Highlights,
    #[strum(serialize = "Channel Discovery")]
    ChannelDiscovery(Option<Server>),
}

impl Buffer {
    pub fn key(&self) -> String {
        match self {
            Buffer::Upstream(upstream) => upstream.key(),
            Buffer::Internal(internal) => internal.key(),
        }
    }

    pub fn upstream(&self) -> Option<&Upstream> {
        if let Self::Upstream(upstream) = self {
            Some(upstream)
        } else {
            None
        }
    }

    pub fn internal(&self) -> Option<&Internal> {
        if let Self::Internal(internal) = self {
            Some(internal)
        } else {
            None
        }
    }
}

impl Upstream {
    pub fn key(&self) -> String {
        match self {
            Upstream::Server(server) => format!("server:{server}"),
            Upstream::Channel(server, channel) => {
                format!("server:{server}:{}", channel.as_str())
            }
            Upstream::Query(server, query) => {
                format!("server:{server}:{}", query.as_str())
            }
        }
    }

    pub fn server(&self) -> &Server {
        match self {
            Self::Server(server)
            | Self::Channel(server, _)
            | Self::Query(server, _) => server,
        }
    }

    pub fn channel(&self) -> Option<&target::Channel> {
        match self {
            Self::Channel(_, channel) => Some(channel),
            Self::Server(_) | Self::Query(_, _) => None,
        }
    }

    pub fn target(&self) -> Option<Target> {
        match self {
            Self::Channel(_, channel) => Some(Target::Channel(channel.clone())),
            Self::Query(_, query) => Some(Target::Query(query.clone())),
            Self::Server(_) => None,
        }
    }

    pub fn server_message_target(
        self,
        source: Option<message::source::Server>,
    ) -> message::Target {
        match self {
            Self::Server(_) => message::Target::Server {
                source: message::Source::Server(source),
            },
            Self::Channel(_, channel) => message::Target::Channel {
                channel,
                source: message::Source::Server(source),
            },
            Self::Query(_, query) => message::Target::Query {
                query,
                source: message::Source::Server(source),
            },
        }
    }
}

impl Internal {
    pub const ALL: &'static [Self] = &[
        Self::FileTransfers,
        Self::Scripts,
        Self::Logs,
        Self::Highlights,
        Self::ChannelDiscovery(None),
    ];

    pub fn key(&self) -> String {
        match self {
            Internal::FileTransfers => "file-transfers",
            Internal::Scripts => "scripts",
            Internal::Logs => "logs",
            Internal::Highlights => "highlights",
            Internal::ChannelDiscovery(_) => "channel-discovery",
        }
        .to_string()
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Settings {
    pub channel: channel::Settings,
}

impl From<config::Buffer> for Settings {
    fn from(config: config::Buffer) -> Self {
        Self {
            channel: channel::Settings::from(config.channel),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Timestamp {
    #[serde(deserialize_with = "deserialize_strftime_date")]
    pub format: String,
    pub brackets: Brackets,
    #[serde(deserialize_with = "deserialize_strftime_date")]
    pub context_menu_format: String,
    #[serde(deserialize_with = "deserialize_strftime_date_maybe")]
    pub copy_format: Option<String>,
    #[serde(deserialize_with = "deserialize_locale")]
    pub locale: Locale,
}

impl Default for Timestamp {
    fn default() -> Self {
        Self {
            format: "%R".to_string(),
            brackets: Brackets::default(),
            context_menu_format: "%x".to_string(),
            copy_format: None,
            locale: Locale::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum BacklogText {
    Text(String),
    Hidden,
}

impl Default for BacklogText {
    fn default() -> Self {
        Self::Text("backlog".to_string())
    }
}

impl<'de> Deserialize<'de> for BacklogText {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum T {
            String(String),
            Bool(bool),
        }

        match T::deserialize(deserializer)? {
            T::String(conf) => Ok(Self::Text(conf)),
            T::Bool(val) => {
                if !val {
                    Ok(Self::Hidden)
                } else {
                    Ok(Self::default())
                }
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct BacklogSeparator {
    pub hide_when_all_read: bool,
    pub text: BacklogText,
}

impl Default for BacklogSeparator {
    fn default() -> Self {
        Self {
            hide_when_all_read: true,
            text: BacklogText::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DateSeparators {
    #[serde(deserialize_with = "deserialize_strftime_date")]
    pub format: String,
    pub show: bool,
}

impl Default for DateSeparators {
    fn default() -> Self {
        Self {
            format: "%A, %B %-d".to_string(),
            show: true,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct StatusMessagePrefix {
    pub brackets: Brackets,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Brackets {
    pub left: String,
    pub right: String,
}

impl Brackets {
    pub fn format(&self, content: impl fmt::Display) -> String {
        format!("{}{}{}", self.left, content, self.right)
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Color {
    Solid,
    #[default]
    Unique,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Alignment {
    #[default]
    Left,
    Right,
    Top,
}

impl Alignment {
    pub fn is_right(&self) -> bool {
        matches!(self, Self::Right)
    }

    pub fn is_top(&self) -> bool {
        matches!(self, Self::Top)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Resize {
    None,
    Maximize,
    Restore,
}

impl Resize {
    pub fn action(can_resize: bool, maximized: bool) -> Self {
        if can_resize {
            if maximized {
                Self::Restore
            } else {
                Self::Maximize
            }
        } else {
            Self::None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SkinTone {
    #[default]
    Default,
    Light,
    MediumLight,
    Medium,
    MediumDark,
    Dark,
}

impl From<SkinTone> for emojis::SkinTone {
    fn from(skin_tone: SkinTone) -> Self {
        match skin_tone {
            SkinTone::Default => emojis::SkinTone::Default,
            SkinTone::Light => emojis::SkinTone::Light,
            SkinTone::MediumLight => emojis::SkinTone::MediumLight,
            SkinTone::Medium => emojis::SkinTone::Medium,
            SkinTone::MediumDark => emojis::SkinTone::MediumDark,
            SkinTone::Dark => emojis::SkinTone::Dark,
        }
    }
}

pub fn deserialize_locale<'de, D>(deserializer: D) -> Result<Locale, D::Error>
where
    D: Deserializer<'de>,
{
    let locale_string_maybe: Option<String> =
        Deserialize::deserialize(deserializer)?;

    if let Some(locale_string) = &locale_string_maybe {
        if let Ok(locale) = Locale::from_str(&locale_string.replace("-", "_")) {
            Ok(locale)
        } else {
            Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(locale_string),
                &"IETF BCP 47 language tag",
            ))
        }
    } else {
        Ok(sys_locale::get_locale()
            .and_then(|locale_string| {
                Locale::from_str(&locale_string.replace("-", "_")).ok()
            })
            .unwrap_or_default())
    }
}
