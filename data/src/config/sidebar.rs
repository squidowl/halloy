use serde::{Deserialize, Deserializer};
use serde_untagged::UntaggedEnumVisitor;

use crate::config::Scrollbar;
use crate::serde::deserialize_positive_integer;

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(default)]
pub struct Sidebar {
    pub max_width: Option<u16>,
    #[serde(deserialize_with = "deserialize_unread_indicator")]
    pub unread_indicator: UnreadIndicator,
    pub position: Position,
    pub order_by: OrderBy,
    pub scrollbar: Scrollbar,
    #[serde(deserialize_with = "deserialize_positive_integer")]
    pub server_icon_size: u32,
    pub user_menu: UserMenu,
}
#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(default)]
pub struct UserMenu {
    pub enabled: bool,
    pub show_new_version_indicator: bool,
}

impl Default for UserMenu {
    fn default() -> Self {
        Self {
            enabled: true,
            show_new_version_indicator: true,
        }
    }
}

impl Default for Sidebar {
    fn default() -> Self {
        Sidebar {
            max_width: None,
            unread_indicator: UnreadIndicator::default(),
            position: Position::default(),
            order_by: OrderBy::default(),
            scrollbar: Scrollbar::default(),
            server_icon_size: 12,
            user_menu: UserMenu::default(),
        }
    }
}

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(default)]
pub struct UnreadIndicator {
    pub title: bool,
    pub icon: Icon,
    #[serde(deserialize_with = "deserialize_positive_integer")]
    pub icon_size: u32,
    pub highlight_icon: Icon,
    #[serde(deserialize_with = "deserialize_positive_integer")]
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
