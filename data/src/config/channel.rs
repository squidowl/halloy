use serde::Deserialize;

use crate::buffer::Color;
use crate::channel::Position;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Channel {
    #[serde(default)]
    pub nicklist: Nicklist,
    #[serde(default)]
    pub topic: Topic,
}

#[derive(Debug, Clone, Default, Deserialize)]
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

fn default_bool_true() -> bool {
    true
}
