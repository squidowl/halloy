use serde::Deserialize;

use super::NicknameClickAction;
use crate::buffer::Color;
use crate::channel::Position;
use crate::config::buffer::Away;
use crate::serde::default_bool_true;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Channel {
    pub nicklist: Nicklist,
    pub topic: Topic,
    pub message: Message,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Message {
    pub nickname_color: Color,
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
    pub show_access_levels: bool,
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
            show_access_levels: true,
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
pub struct Topic {
    pub enabled: bool,
    pub max_lines: u16,
}

impl Default for Topic {
    fn default() -> Self {
        Self {
            enabled: false,
            max_lines: 2,
        }
    }
}
