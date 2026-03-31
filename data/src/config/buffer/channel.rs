use serde::Deserialize;

use super::NicknameClickAction;
use crate::channel::Position;
use crate::config::buffer::AccessLevelFormat;
use crate::isupport;
use crate::serde::deserialize_u32_positive_integer;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Channel {
    pub nicklist: Nicklist,
    #[serde(alias = "topic")] // For backwards compatibility
    pub topic_banner: TopicBanner,
    pub message: Message,
    pub channel_name_casing: Option<ChannelNameCasing>,
}

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChannelNameCasing {
    Lowercase,
}

impl ChannelNameCasing {
    pub fn apply(&self, name: &str, casemapping: isupport::CaseMap) -> String {
        match self {
            Self::Lowercase => casemapping.normalize(name),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Message {
    pub show_emoji_reacts: bool,
    #[serde(deserialize_with = "deserialize_u32_positive_integer")]
    pub max_reaction_display: u32,
    #[serde(deserialize_with = "deserialize_u32_positive_integer")]
    pub max_reaction_chars: u32,
}

impl Default for Message {
    fn default() -> Self {
        Self {
            show_emoji_reacts: true,
            max_reaction_display: 5,
            max_reaction_chars: 64,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Nicklist {
    pub enabled: bool,
    pub position: Position,
    pub width: Option<f32>,
    pub alignment: Alignment,
    pub show_access_levels: AccessLevelFormat,
    pub click: NicknameClickAction,
}

impl Default for Nicklist {
    fn default() -> Self {
        Self {
            enabled: true,
            position: Position::default(),
            width: None,
            alignment: Alignment::default(),
            show_access_levels: AccessLevelFormat::default(),
            click: NicknameClickAction::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Alignment {
    #[default]
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(default)]
pub struct TopicBanner {
    pub enabled: bool,
    pub max_lines: u16,
}

impl Default for TopicBanner {
    fn default() -> Self {
        Self {
            enabled: false,
            max_lines: 2,
        }
    }
}
