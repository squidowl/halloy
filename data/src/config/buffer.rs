use chrono::{DateTime, Local, Utc};
use serde::Deserialize;

use super::Channel;
use crate::{
    buffer::{Color, InputVisibility, Nickname, Timestamp},
    message::source,
};

#[derive(Debug, Clone, Deserialize)]
pub struct Buffer {
    #[serde(default)]
    pub timestamp: Option<Timestamp>,
    #[serde(default)]
    pub nickname: Nickname,
    #[serde(default)]
    pub input_visibility: InputVisibility,
    #[serde(default)]
    pub channel: Channel,
    #[serde(default)]
    pub server_messages: ServerMessages,
}

#[derive(Debug, Copy, Clone, Default, Deserialize)]
pub enum Exclude {
    #[default]
    All,
    None,
    Smart(i64),
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ServerMessages {
    #[serde(default)]
    pub join: ServerMessage,
    #[serde(default)]
    pub part: ServerMessage,
    #[serde(default)]
    pub quit: ServerMessage,
}

impl ServerMessages {
    pub fn get(&self, server: &source::Server) -> ServerMessage {
        match server.kind() {
            source::server::Kind::Join => self.join,
            source::server::Kind::Part => self.part,
            source::server::Kind::Quit => self.quit,
        }
    }
}

#[derive(Debug, Copy, Clone, Default, Deserialize)]
pub struct ServerMessage {
    #[serde(default)]
    pub exclude: Exclude,
    #[serde(default)]
    pub username_format: UsernameFormat,
}

#[derive(Debug, Copy, Clone, Default, Deserialize)]
pub enum UsernameFormat {
    Short,
    #[default]
    Full,
}

impl Default for Buffer {
    fn default() -> Self {
        Buffer {
            timestamp: Some(Timestamp {
                format: "%T".into(),
                brackets: Default::default(),
            }),
            nickname: Nickname {
                color: Color::Unique,
                brackets: Default::default(),
            },
            input_visibility: InputVisibility::default(),
            channel: Channel::default(),
            server_messages: Default::default(),
        }
    }
}

impl Buffer {
    pub fn format_timestamp(&self, date_time: &DateTime<Utc>) -> Option<String> {
        self.timestamp.as_ref().map(|timestamp| {
            timestamp
                .brackets
                .format(date_time.with_timezone(&Local).format(&timestamp.format))
        })
    }
}
