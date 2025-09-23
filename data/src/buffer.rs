use core::fmt;
use std::str::FromStr;

use chrono::Locale;
use iced_core::Color as IcedColor;
use serde::{Deserialize, Deserializer, Serialize};

pub mod timestamp;

pub use self::timestamp::Timestamp;
use crate::appearance::theme::hex_to_color;
use crate::serde::deserialize_strftime_date;
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
    Logs,
    Highlights,
    #[strum(serialize = "Channel Discovery")]
    ChannelDiscovery(Option<Server>),
    #[strum(serialize = "Search Results")]
    SearchResults(Server),
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
    ) -> Option<message::Target> {
        match self {
            Self::Server(_) => Some(message::Target::Server {
                source: message::Source::Server(source),
            }),
            Self::Channel(_, channel) => Some(message::Target::Channel {
                channel,
                source: message::Source::Server(source),
            }),
            Self::Query(_, query) => Some(message::Target::Query {
                query,
                source: message::Source::Server(source),
            }),
        }
    }
}

impl Internal {
    pub const ALL: &'static [Self] = &[
        Self::FileTransfers,
        Self::Logs,
        Self::Highlights,
        Self::ChannelDiscovery(None),
    ];

    pub fn key(&self) -> String {
        match self {
            Internal::FileTransfers => "file-transfers".to_string(),
            Internal::Logs => "logs".to_string(),
            Internal::Highlights => "highlights".to_string(),
            Internal::ChannelDiscovery(_) => "channel-discovery".to_string(),
            Internal::SearchResults(server) => {
                format!("server:{server}:search-results")
            }
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
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

#[derive(Debug, Clone, Default)]
pub enum Color {
    Solid,
    #[default]
    Unique,
    Palette(Vec<IcedColor>),
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            String(String),
            Palette { palette: Vec<String> },
        }

        match Repr::deserialize(deserializer)? {
            Repr::String(value) => match value.as_str() {
                "solid" => Ok(Self::Solid),
                "unique" => Ok(Self::Unique),
                _ => Err(serde::de::Error::custom(format!(
                    "unknown color: {value}",
                ))),
            },
            Repr::Palette { palette } => {
                if palette.is_empty() {
                    return Err(serde::de::Error::custom(
                        "palette must contain at least one hex color",
                    ));
                }

                let colors = palette
                    .into_iter()
                    .map(|hex| {
                        hex_to_color(&hex).ok_or_else(|| {
                            serde::de::Error::custom(format!(
                                "invalid hex color in palette: {hex}",
                            ))
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(Self::Palette(colors))
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use super::Color;

    #[derive(Debug, serde::Deserialize)]
    struct Root {
        color: Color,
    }

    #[test]
    fn color_deserializes_palette() {
        let root: Root = toml::from_str(
            r##"color = { palette = ["#112233", "#445566", "#778899"] }"##,
        )
        .expect("valid palette color");

        match root.color {
            Color::Palette(colors) => assert_eq!(colors.len(), 3),
            _ => panic!("expected palette color"),
        }
    }

    #[test]
    fn color_rejects_empty_palette() {
        let err = toml::from_str::<Root>(r#"color = { palette = [] }"#)
            .expect_err("empty palette should be rejected");

        assert!(
            err.to_string()
                .contains("palette must contain at least one")
        );
    }

    #[test]
    fn color_rejects_invalid_palette_hex() {
        let err = toml::from_str::<Root>(
            r##"color = { palette = ["#112233", "not-a-color"] }"##,
        )
        .expect_err("invalid palette hex should be rejected");

        assert!(err.to_string().contains("invalid hex color in palette"));
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
