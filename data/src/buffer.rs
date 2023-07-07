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

    pub fn server_message_source(self) -> message::Source {
        match self {
            Self::Server(_) => message::Source::Server,
            Self::Channel(_, channel) => message::Source::Channel(channel, message::Sender::Server),
            Self::Query(_, nick) => message::Source::Query(nick, message::Sender::Server),
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
pub enum InputVisibility {
    Focused,
    #[default]
    Always,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Timestamp {
    pub format: String,
    #[serde(default)]
    pub brackets: Brackets,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Nickname {
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
        format!("{}{}{} ", self.left, content, self.right)
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub enum Color {
    Solid,
    #[default]
    Unique,
}
