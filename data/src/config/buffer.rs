use serde::{Deserialize, Serialize};

use crate::Message;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Buffer {
    #[serde(default)]
    pub timestamp: Option<Timestamp>,
    #[serde(default)]
    pub nickname: Nickname,
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
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Timestamp {
    pub format: String,
    #[serde(default)]
    pub brackets: Brackets,
}

impl Timestamp {
    pub fn format_message_with_timestamp(&self, message: &Message) -> String {
        format!(
            "{}{}{} ",
            self.brackets.left,
            &message.formatted_datetime(self.format.as_str()),
            self.brackets.right,
        )
    }
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

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub enum Color {
    Solid,
    #[default]
    Unique,
}
