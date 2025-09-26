use serde::{Deserialize, Serialize};

use crate::config;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Settings {
    pub nicklist: Nicklist,
    pub topic_banner: TopicBanner,
}

impl From<config::buffer::Channel> for Settings {
    fn from(config: config::buffer::Channel) -> Self {
        Self {
            nicklist: Nicklist::from(config.nicklist),
            topic_banner: TopicBanner::from(config.topic_banner),
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
pub struct TopicBanner {
    pub enabled: bool,
}

impl From<config::buffer::channel::TopicBanner> for TopicBanner {
    fn from(config: config::buffer::channel::TopicBanner) -> Self {
        TopicBanner {
            enabled: config.enabled,
        }
    }
}

impl TopicBanner {
    pub fn toggle_visibility(&mut self) {
        self.enabled = !self.enabled;
    }
}
