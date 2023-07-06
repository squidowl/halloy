use core::fmt;

use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};

use crate::user::Nick;
use crate::{channel, message, Server};

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

    pub fn message_source(self) -> message::Source {
        match self {
            Self::Server(_) => message::Source::Server,
            Self::Channel(_, channel) => message::Source::Channel(channel, message::Sender::Server),
            Self::Query(_, nick) => message::Source::Query(nick, message::Sender::Server),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Settings {
    #[serde(default)]
    pub timestamp: Option<Timestamp>,
    #[serde(default)]
    pub nickname: Nickname,
    #[serde(default)]
    pub channel: channel::Settings,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            timestamp: Some(Timestamp {
                format: "%T".into(),
                brackets: Default::default(),
            }),
            nickname: Nickname {
                color: Color::Unique,
                brackets: Default::default(),
            },
            channel: channel::Settings::default(),
        }
    }
}

impl Settings {
    pub fn format_timestamp(&self, date_time: &DateTime<Utc>) -> Option<String> {
        self.timestamp.as_ref().map(|timestamp| {
            timestamp
                .brackets
                .format(date_time.with_timezone(&Local).format(&timestamp.format))
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Timestamp {
    pub format: String,
    #[serde(default)]
    pub brackets: Brackets,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Nickname {
    pub color: Color,
    #[serde(default)]
    pub brackets: Brackets,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Brackets {
    pub left: String,
    pub right: String,
}

impl Brackets {
    pub fn format(&self, content: impl fmt::Display) -> String {
        format!("{}{}{} ", self.left, content, self.right)
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub enum Color {
    Solid,
    #[default]
    Unique,
}
