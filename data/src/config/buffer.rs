use std::collections::HashSet;

use chrono::{DateTime, Local, Utc};
use serde::Deserialize;

use super::Channel;
use crate::buffer::{Color, InputVisibility, Nickname, Timestamp};
use crate::message;

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
    pub hidden_server_messages: HashSet<message::source::Server>,
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
            hidden_server_messages: HashSet::default(),
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
