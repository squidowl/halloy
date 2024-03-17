use chrono::{DateTime, Local, Utc};
use serde::Deserialize;

use super::Channel;
use crate::{
    buffer::{Color, Nickname, TextInput, Timestamp},
    message::source,
};

#[derive(Debug, Clone, Deserialize)]
pub struct Buffer {
    #[serde(default)]
    pub timestamp: Timestamp,
    #[serde(default)]
    pub nickname: Nickname,
    #[serde(default)]
    pub text_input: TextInput,
    #[serde(default)]
    pub channel: Channel,
    #[serde(default)]
    pub server_messages: ServerMessages,
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
#[serde(rename_all = "kebab-case")]
pub struct ServerMessage {
    #[serde(default = "default_bool_true")]
    pub enabled: bool,
    #[serde(default)]
    pub smart: Option<i64>,
    #[serde(default)]
    pub username_format: UsernameFormat,
}

#[derive(Debug, Copy, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UsernameFormat {
    Short,
    #[default]
    Full,
}

impl Default for Buffer {
    fn default() -> Self {
        Buffer {
            timestamp: Timestamp::default(),
            nickname: Nickname {
                color: Color::Unique,
                brackets: Default::default(),
            },
            text_input: Default::default(),
            channel: Channel::default(),
            server_messages: Default::default(),
        }
    }
}

impl Buffer {
    pub fn format_timestamp(&self, date_time: &DateTime<Utc>) -> Option<String> {
        if self.timestamp.format.is_empty() {
            return None;
        }

        Some(format!(
            "{} ",
            self.timestamp.brackets.format(
                date_time
                    .with_timezone(&Local)
                    .format(&self.timestamp.format)
            )
        ))
    }
}

fn default_bool_true() -> bool {
    true
}
