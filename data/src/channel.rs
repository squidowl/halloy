use serde::{Deserialize, Serialize};

use crate::config;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Settings {
    pub nicklist: Nicklist,
    pub topic_banner: TopicBanner,
    pub typing: Typing,
}

impl From<config::buffer::Channel> for Settings {
    fn from(config: config::buffer::Channel) -> Self {
        Self {
            nicklist: Nicklist::from(config.nicklist),
            topic_banner: TopicBanner::from(config.topic_banner),
            typing: Typing::from(config.typing),
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

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(default)]
pub struct Typing {
    pub share: bool,
    pub show: bool,
}

impl Default for Typing {
    fn default() -> Self {
        Self {
            share: false,
            show: true,
        }
    }
}

impl From<config::buffer::channel::Typing> for Typing {
    fn from(config: config::buffer::channel::Typing) -> Self {
        Typing {
            share: config.share,
            show: config.show,
        }
    }
}

impl Typing {
    pub fn toggle_share(&mut self) {
        self.share = !self.share;
    }

    pub fn toggle_show(&mut self) {
        self.show = !self.show;
    }
}
