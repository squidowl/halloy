use core::fmt;

use serde::{Deserialize, Serialize};

use crate::user::Nick;
use crate::{channel, config, message, Server};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Buffer {
    Server(Server),
    Channel(Server, String),
    Query(Server, Nick),
}

impl Buffer {
    pub fn server(&self) -> &Server {
        match self {
            Buffer::Server(server) | Buffer::Channel(server, _) | Buffer::Query(server, _) => {
                server
            }
        }
    }

    pub fn target(&self) -> Option<String> {
        match self {
            Buffer::Server(_) => None,
            Buffer::Channel(_, channel) => Some(channel.clone()),
            Buffer::Query(_, nick) => Some(nick.to_string()),
        }
    }

    pub fn server_message_target(self, source: Option<message::source::Server>) -> message::Target {
        match self {
            Self::Server(_) => message::Target::Server {
                source: message::Source::Server(source),
            },
            Self::Channel(_, channel) => message::Target::Channel {
                channel,
                source: message::Source::Server(source),
            },
            Self::Query(_, nick) => message::Target::Query {
                nick,
                source: message::Source::Server(source),
            },
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Settings {
    pub channel: channel::Settings,
}

impl From<config::Buffer> for Settings {
    fn from(config: config::Buffer) -> Self {
        Self {
            channel: channel::Settings::from(config.channel),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct TextInput {
    pub visibility: TextInputVisibility,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TextInputVisibility {
    Focused,
    #[default]
    Always,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Timestamp {
    #[serde(default = "default_timestamp")]
    pub format: String,
    #[serde(default)]
    pub brackets: Brackets,
}

impl Default for Timestamp {
    fn default() -> Self {
        Self {
            format: default_timestamp(),
            brackets: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Nickname {
    #[serde(default)]
    pub color: Color,
    #[serde(default)]
    pub brackets: Brackets,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Brackets {
    pub left: String,
    pub right: String,
}

impl Brackets {
    pub fn format(&self, content: impl fmt::Display) -> String {
        format!("{}{}{}", self.left, content, self.right)
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Color {
    Solid,
    #[default]
    Unique,
}

#[derive(Debug, Clone, Copy)]
pub enum Resize {
    None,
    Maximize,
    Restore,
}

impl Resize {
    pub fn action(can_resize: bool, maximized: bool) -> Self {
        if can_resize {
            if maximized {
                Self::Restore
            } else {
                Self::Maximize
            }
        } else {
            Self::None
        }
    }
}

fn default_timestamp() -> String {
    "%R".to_string()
}
