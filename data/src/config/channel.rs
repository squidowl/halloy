use serde::Deserialize;

use crate::buffer::Color;
use crate::channel::Position;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Channel {
    pub users: Users,
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

#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct Topic {
    #[serde(default)]
    pub(crate) visible: bool,
    #[serde(default = "default_topic_banner_max_lines")]
    pub max_lines: u16,
}

fn default_topic_banner_max_lines() -> u16 {
    2
}
