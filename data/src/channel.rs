use serde::{Deserialize, Serialize};

use crate::config;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Settings {
    pub nicklist: Nicklist,
    pub topic: Topic,
}

impl From<config::buffer::Channel> for Settings {
    fn from(config: config::buffer::Channel) -> Self {
        Self {
            nicklist: Nicklist::from(config.nicklist),
            topic: Topic::from(config.topic),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Position {
    Left,
    #[default]
    Right,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Nicklist {
    pub enabled: bool,
}

impl From<config::buffer::channel::Nicklist> for Nicklist {
    fn from(config: config::buffer::channel::Nicklist) -> Self {
        Nicklist {
            enabled: config.enabled,
        }
    }
}

impl Default for Nicklist {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl Nicklist {
    pub fn toggle_visibility(&mut self) {
        self.enabled = !self.enabled;
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
pub struct Topic {
    pub enabled: bool,
}

impl From<config::buffer::channel::Topic> for Topic {
    fn from(config: config::buffer::channel::Topic) -> Self {
        Topic {
            enabled: config.enabled,
        }
    }
}

impl Topic {
    pub fn toggle_visibility(&mut self) {
        self.enabled = !self.enabled;
    }
}
