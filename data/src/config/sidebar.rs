use serde::{Deserialize, Deserializer};
use serde_untagged::UntaggedEnumVisitor;

use crate::config::Scrollbar;

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(default)]
pub struct Sidebar {
    pub max_width: Option<u16>,
    #[serde(deserialize_with = "deserialize_unread_indicator")]
    pub unread_indicator: UnreadIndicator,
    pub position: Position,
    pub show_user_menu: bool,
    pub order_by: OrderBy,
    pub scrollbar: Scrollbar,
    #[serde(deserialize_with = "deserialize_icon_size")]
    pub server_icon_size: u32,
}

impl Default for Sidebar {
    fn default() -> Self {
        Sidebar {
            max_width: None,
            unread_indicator: UnreadIndicator::default(),
            position: Position::default(),
            show_user_menu: true,
            order_by: OrderBy::default(),
            scrollbar: Scrollbar::default(),
            server_icon_size: 12,
        }
    }
}

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(default)]
pub struct UnreadIndicator {
    pub title: bool,
    pub icon: Icon,
    #[serde(deserialize_with = "deserialize_icon_size")]
    pub icon_size: u32,
    pub highlight_icon: Icon,
    #[serde(deserialize_with = "deserialize_icon_size")]
    pub highlight_icon_size: u32,
}

impl Default for UnreadIndicator {
    fn default() -> Self {
        UnreadIndicator {
            title: false,
            icon: Icon::Dot,
            icon_size: 6,
            highlight_icon: Icon::CircleEmpty,
            highlight_icon_size: 8,
        }
    }
}

impl UnreadIndicator {
    pub fn has_unread_icon(&self) -> bool {
        !matches!(self.icon, Icon::None)
    }

    pub fn has_unread_highlight_icon(&self) -> bool {
        !matches!(self.highlight_icon, Icon::None)
    }
}

pub fn deserialize_unread_indicator<'de, D>(
    deserializer: D,
) -> Result<UnreadIndicator, D::Error>
where
    D: Deserializer<'de>,
{
    #[allow(clippy::redundant_closure_for_method_calls)]
    UntaggedEnumVisitor::new()
        .string(|string| match string {
            "title" => Ok(UnreadIndicator {
                title: true,
                icon: Icon::None,
                highlight_icon: Icon::None,
                ..UnreadIndicator::default()
            }),
            "none" => Ok(UnreadIndicator {
                title: false,
                icon: Icon::None,
                highlight_icon: Icon::None,
                ..UnreadIndicator::default()
            }),
            "dot" => Ok(UnreadIndicator {
                title: false,
                icon: Icon::Dot,
                highlight_icon: Icon::Dot,
                ..UnreadIndicator::default()
            }),
            _ => Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(string),
                &"one of: \"dot\", \"title\", or \"none\"",
            )),
        })
        .map(|map| map.deserialize())
        .deserialize(deserializer)
}

#[derive(Debug, Copy, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Icon {
    #[default]
    Dot,
    CircleEmpty,
    DotCircled,
    Certificate,
    Asterisk,
    Speaker,
    Lightbulb,
    Star,
    None,
}

#[derive(Debug, Copy, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Position {
    #[default]
    Left,
    Right,
    Top,
    Bottom,
}

impl Position {
    pub fn is_horizontal(&self) -> bool {
        match self {
            Position::Left | Position::Right => false,
            Position::Top | Position::Bottom => true,
        }
    }
}

#[derive(Debug, Copy, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum OrderBy {
    #[default]
    Alpha,
    Config,
}

pub fn deserialize_icon_size<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    let integer: u32 = Deserialize::deserialize(deserializer)?;

    if integer == 0 || integer > 17 {
        Err(serde::de::Error::invalid_value(
            serde::de::Unexpected::Unsigned(integer.into()),
            &"any positive integer less than or equal to 17",
        ))
    } else {
        Ok(integer)
    }
}
