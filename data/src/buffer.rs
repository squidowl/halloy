use core::fmt;

use serde::{Deserialize, Serialize};

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
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    strum::Display,
)]
pub enum Internal {
    #[strum(serialize = "File Transfers")]
    FileTransfers,
    Logs,
    Highlights,
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
    pub const ALL: &'static [Self] =
        &[Self::FileTransfers, Self::Logs, Self::Highlights];

    pub fn key(&self) -> String {
        match self {
            Internal::FileTransfers => "file-transfers",
            Internal::Logs => "logs",
            Internal::Highlights => "highlights",
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
pub struct TextInputNickname {
    pub enabled: bool,
    pub show_access_level: bool,
}

impl Default for TextInputNickname {
    fn default() -> Self {
        Self {
            enabled: true,
            show_access_level: true,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct TextInput {
    pub visibility: TextInputVisibility,
    pub auto_format: AutoFormat,
    pub autocomplete: Autocomplete,
    pub nickname: TextInputNickname,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OrderBy {
    Alpha,
    #[default]
    Recent,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SortDirection {
    #[default]
    Asc,
    Desc,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Autocomplete {
    pub order_by: OrderBy,
    pub sort_direction: SortDirection,
    pub completion_suffixes: [String; 2],
}

impl Default for Autocomplete {
    fn default() -> Self {
        Self {
            order_by: OrderBy::default(),
            sort_direction: SortDirection::default(),
            completion_suffixes: [": ".to_string(), " ".to_string()],
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TextInputVisibility {
    Focused,
    #[default]
    Always,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AutoFormat {
    #[default]
    Disabled,
    Markdown,
    All,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Timestamp {
    #[serde(deserialize_with = "deserialize_strftime_date")]
    pub format: String,
    pub brackets: Brackets,
    #[serde(deserialize_with = "deserialize_strftime_date")]
    pub context_menu_format: String,
}

impl Default for Timestamp {
    fn default() -> Self {
        Self {
            format: "%R".to_string(),
            brackets: Brackets::default(),
            context_menu_format: "%x".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct BacklogSeparator {
    pub hide_when_all_read: bool,
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
