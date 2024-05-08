use core::fmt;

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

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
pub enum ColorKind {
    Solid,
    #[default]
    Unique,
}

#[derive(Debug, Clone)]
pub struct Color {
    pub kind: ColorKind,
    pub hex: Option<String>,
}

impl Default for Color {
    fn default() -> Self {
        Self { kind: ColorKind::default(), hex: None }
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ColorVisitor)
    }
}

/// A visitor to handle both string and map cases for Color.
struct ColorVisitor;

impl<'de> Visitor<'de> for ColorVisitor {
    type Value = Color;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string or a map with 'kind' and optionally 'hex'")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match v {
            "solid" => Ok(Color { kind: ColorKind::Solid, hex: None }),
            "unique" => Ok(Color { kind: ColorKind::Unique, hex: None }),
            _ => Err(de::Error::unknown_variant(v, &["solid", "unique"])),
        }
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut kind: Option<ColorKind> = None;
        let mut hex: Option<String> = None;

        while let Some((key, value)) = map.next_entry::<String, String>()? {
            match key.as_str() {
                "kind" => {
                    kind = match value.as_str() {
                        "solid" => Some(ColorKind::Solid),
                        "unique" => Some(ColorKind::Unique),
                        _ => return Err(de::Error::unknown_variant(&value, &["solid", "unique"])),
                    };
                }
                "hex" => {
                    hex = Some(value);
                }
                _ => return Err(de::Error::unknown_field(&key, &["kind", "hex"])),
            }
        }

        let kind = kind.ok_or_else(|| de::Error::missing_field("kind"))?;
        Ok(Color { kind, hex })
    }
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
