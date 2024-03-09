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
    All,
    #[default]
    None,
    Smart(i64),
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ServerMessages {
    #[serde(default)]
    pub topic: ServerMessage,
    #[serde(default)]
    pub join: ServerMessage,
    #[serde(default)]
    pub part: ServerMessage,
    #[serde(default)]
    pub quit: ServerMessage,
}

impl ServerMessages {
    pub fn get(&self, server: &source::Server) -> Option<ServerMessage> {
        match server.kind() {
            source::server::Kind::ReplyTopic => Some(self.topic),
            source::server::Kind::Part => Some(self.part),
            source::server::Kind::Quit => Some(self.quit),
            source::server::Kind::Join => Some(self.join),
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
            format!(
                "{} ",
                timestamp
                    .brackets
                    .format(date_time.with_timezone(&Local).format(&timestamp.format))
            )
        })
    }
}
