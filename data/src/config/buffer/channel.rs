use serde::Deserialize;

use super::NicknameClickAction;
use crate::buffer::Color;
use crate::channel::Position;
use crate::serde::default_bool_true;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Channel {
    #[serde(default)]
    pub nicklist: Nicklist,
    #[serde(default)]
    pub topic: Topic,
    #[serde(default)]
    pub message: Message,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Message {
    #[serde(default)]
    pub nickname_color: Color,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Nicklist {
    #[serde(default = "default_bool_true")]
    pub enabled: bool,
    #[serde(default)]
    pub position: Position,
    #[serde(default)]
    pub color: Color,
    #[serde(default)]
    pub width: Option<f32>,
    #[serde(default)]
    pub alignment: Alignment,
    #[serde(default = "default_bool_true")]
    pub show_access_levels: bool,
    #[serde(default)]
    pub click: NicknameClickAction,
}

impl Default for Nicklist {
    fn default() -> Self {
        Self {
            enabled: default_bool_true(),
            position: Position::default(),
            color: Color::default(),
            width: Option::default(),
            alignment: Alignment::default(),
            show_access_levels: default_bool_true(),
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
pub struct Topic {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_topic_banner_max_lines")]
    pub max_lines: u16,
}

impl Default for Topic {
    fn default() -> Self {
        Self {
            enabled: false,
            max_lines: default_topic_banner_max_lines(),
        }
    }
}

fn default_topic_banner_max_lines() -> u16 {
    2
}
