use serde::Deserialize;

use super::NicknameClickAction;
use crate::buffer::Color;
use crate::channel::Position;
use crate::config::buffer::{AccessLevelFormat, Away};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Channel {
    pub nicklist: Nicklist,
    #[serde(alias = "topic")] // For backwards compatibility
    pub topic_banner: TopicBanner,
    pub message: Message,
    #[serde(default)]
    pub lowercase_channel: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Message {
    pub nickname_color: Color,
    pub show_emoji_reacts: bool,
}

impl Default for Message {
    fn default() -> Self {
        Self {
            nickname_color: Color::default(),
            show_emoji_reacts: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Nicklist {
    pub away: Away,
    pub enabled: bool,
    pub position: Position,
    pub color: Color,
    pub width: Option<f32>,
    pub alignment: Alignment,
    pub show_access_levels: AccessLevelFormat,
    pub click: NicknameClickAction,
}

impl Default for Nicklist {
    fn default() -> Self {
        Self {
            away: Away::default(),
            enabled: true,
            position: Position::default(),
            color: Color::default(),
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
