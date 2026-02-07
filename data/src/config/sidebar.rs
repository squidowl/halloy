use serde::{Deserialize, Deserializer};
use serde_untagged::UntaggedEnumVisitor;

use crate::config::Scrollbar;
use crate::config::inclusivities::{Inclusivities, is_target_channel_included};
use crate::serde::deserialize_positive_integer;
use crate::server::Server;
use crate::{isupport, target};

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Sidebar {
    pub max_width: Option<u16>,
    #[serde(deserialize_with = "deserialize_unread_indicator")]
    pub unread_indicator: UnreadIndicator,
    pub position: Position,
    pub show_nicklist: bool,
    pub split: bool,
    #[serde(deserialize_with = "deserialize_positive_integer")]
    pub buflist_space: u16,
    #[serde(deserialize_with = "deserialize_positive_integer")]
    pub nicklist_space: u16,
    pub order_by: OrderBy,
    pub scrollbar: Scrollbar,
    #[serde(
        deserialize_with = "deserialize_server_icon",
        alias = "server_icon_size"
    )]
    pub server_icon: ServerIcon,
    pub user_menu: UserMenu,
    pub padding: Padding,
    pub spacing: Spacing,
}

impl Default for Sidebar {
    fn default() -> Self {
        Self {
            max_width: None,
            unread_indicator: UnreadIndicator::default(),
            position: Position::default(),
            show_nicklist: false,
            split: true,
            buflist_space: 2,
            nicklist_space: 1,
            order_by: OrderBy::default(),
            scrollbar: Scrollbar::default(),
            server_icon: ServerIcon::default(),
            user_menu: UserMenu::default(),
            padding: Padding::default(),
            spacing: Spacing::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ServerIcon {
    Size(u32),
    Hidden,
}

impl Default for ServerIcon {
    fn default() -> Self {
        Self::Size(12)
    }
}

#[allow(clippy::redundant_closure_for_method_calls)]
pub fn deserialize_server_icon<'de, D>(
    deserializer: D,
) -> Result<ServerIcon, D::Error>
where
    D: Deserializer<'de>,
{
    UntaggedEnumVisitor::new()
        .u32(|value| {
            if value > 0 {
                Ok(ServerIcon::Size(value))
            } else {
                Err(serde::de::Error::invalid_value(
                    serde::de::Unexpected::Unsigned(value as u64),
                    &"a positive integer",
                ))
            }
        })
        .string(|string| match string {
            "hidden" | "none" => Ok(ServerIcon::Hidden),
            _ => Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(string),
                &"\"hidden\" or a size (positive integer)",
            )),
        })
        .bool(|value| match value {
            true => Ok(ServerIcon::Size(12)),
            false => Ok(ServerIcon::Hidden),
        })
        .map(|map| map.deserialize())
        .deserialize(deserializer)
}

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(default)]
pub struct Padding {
    pub buffer: [u16; 2],
}

impl Default for Padding {
    fn default() -> Self {
        Self { buffer: [5, 5] }
    }
}

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(default)]
pub struct Spacing {
    pub server: u32,
}

impl Default for Spacing {
    fn default() -> Self {
        Self { server: 12 }
    }
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

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct UnreadIndicator {
    pub title: bool,
    pub icon: Icon,
    #[serde(deserialize_with = "deserialize_positive_integer")]
    pub icon_size: u32,
    pub highlight_icon: Icon,
    #[serde(deserialize_with = "deserialize_positive_integer")]
    pub highlight_icon_size: u32,
    pub query_as_highlight: bool,
    pub exclude: Option<Inclusivities>,
    pub include: Option<Inclusivities>,
}

impl Default for UnreadIndicator {
    fn default() -> Self {
        UnreadIndicator {
            title: false,
            icon: Icon::Dot,
            icon_size: 6,
            highlight_icon: Icon::CircleEmpty,
            highlight_icon_size: 8,
            query_as_highlight: false,
            exclude: None,
            include: None,
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

    pub fn should_indicate_unread(
        &self,
        channel: &target::Channel,
        server: &Server,
        casemapping: isupport::CaseMap,
    ) -> bool {
        is_target_channel_included(
            self.include.as_ref(),
            self.exclude.as_ref(),
            None,
            channel,
            server,
            casemapping,
        )
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
