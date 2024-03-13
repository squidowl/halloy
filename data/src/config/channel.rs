use serde::Deserialize;

use crate::buffer::Color;
use crate::channel::Position;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Channel {
    #[serde(default)]
    pub users: Users,
    #[serde(default)]
    pub topic: Topic,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Users {
    pub(crate) visible: bool,
    #[serde(default)]
    pub position: Position,
    #[serde(default)]
    pub color: Color,
}

impl Default for Users {
    fn default() -> Self {
        Self {
            visible: true,
            position: Position::default(),
            color: Color::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Topic {
    #[serde(default)]
    pub visible: bool,
    #[serde(default = "default_topic_banner_max_lines")]
    pub max_lines: u16,
}

impl Default for Topic {
    fn default() -> Self {
        Self {
            visible: false,
            max_lines: default_topic_banner_max_lines(),
        }
    }
}

fn default_topic_banner_max_lines() -> u16 {
    2
}
