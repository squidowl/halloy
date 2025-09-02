use serde::{Deserialize, Deserializer};

use crate::config::Scrollbar;

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(default)]
pub struct Sidebar {
    pub max_width: Option<u16>,
    pub unread_indicator: UnreadIndicator,
    pub position: Position,
    pub show_user_menu: bool,
    pub order_by: OrderBy,
    pub scrollbar: Scrollbar,
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
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct UnreadIndicator {
    pub title: bool,
    pub icon: Option<Icon>,
    pub icon_size: u32,
    pub highlight_icon: Option<Icon>,
    pub highlight_icon_size: u32,
}

impl Default for UnreadIndicator {
    fn default() -> Self {
        UnreadIndicator {
            title: false,
            icon: Some(Icon::default()),
            icon_size: 6,
            highlight_icon: Some(Icon::default()),
            highlight_icon_size: 6,
        }
    }
}

impl UnreadIndicator {
    pub fn has_unread_icon(&self) -> bool {
        self.icon.is_some()
    }

    pub fn has_unread_highlight_icon(&self) -> bool {
        self.highlight_icon.is_some()
    }
}

impl<'de> Deserialize<'de> for UnreadIndicator {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum IconRepr {
            Bool(bool),
            String(String),
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum UnreadIndicatorRepr {
            String(String),
            Struct {
                title: Option<bool>,
                icon: Option<IconRepr>,
                icon_size: Option<u32>,
                highlight_icon: Option<IconRepr>,
                highlight_icon_size: Option<u32>,
            },
        }

        let repr = UnreadIndicatorRepr::deserialize(deserializer)?;
        match repr {
            UnreadIndicatorRepr::String(s) => match s.as_str() {
                "title" => Ok(UnreadIndicator {
                    title: true,
                    icon: None,
                    highlight_icon: None,
                    ..UnreadIndicator::default()
                }),
                "none" => Ok(UnreadIndicator {
                    title: false,
                    icon: None,
                    highlight_icon: None,
                    ..UnreadIndicator::default()
                }),
                _ => Ok(UnreadIndicator::default()),
            },
            UnreadIndicatorRepr::Struct {
                title,
                icon,
                icon_size,
                highlight_icon,
                highlight_icon_size,
            } => {
                let icon = match icon {
                    Some(icon_repr) => match icon_repr {
                        IconRepr::Bool(enabled) => match enabled {
                            true => Some(Icon::default()),
                            false => None,
                        },
                        IconRepr::String(s) => Some(Icon::from(s.as_str())),
                    },
                    None => UnreadIndicator::default().icon,
                };

                if let Some(icon_size) = icon_size
                    && icon_size == 0
                {
                    return Err(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Unsigned(icon_size.into()),
                        &"any positive integer",
                    ));
                }

                let highlight_icon = match highlight_icon {
                    Some(icon_repr) => match icon_repr {
                        IconRepr::Bool(enabled) => match enabled {
                            true => Some(Icon::default()),
                            false => None,
                        },
                        IconRepr::String(s) => Some(Icon::from(s.as_str())),
                    },
                    None => UnreadIndicator::default().highlight_icon,
                };

                if let Some(highlight_icon_size) = highlight_icon_size
                    && highlight_icon_size == 0
                {
                    return Err(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Unsigned(
                            highlight_icon_size.into(),
                        ),
                        &"any positive integer",
                    ));
                }

                Ok(UnreadIndicator {
                    title: title.unwrap_or(UnreadIndicator::default().title),
                    icon,
                    icon_size: icon_size
                        .unwrap_or(UnreadIndicator::default().icon_size),
                    highlight_icon,
                    highlight_icon_size: highlight_icon_size.unwrap_or(
                        UnreadIndicator::default().highlight_icon_size,
                    ),
                })
            }
        }
    }
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
}

impl From<&str> for Icon {
    fn from(value: &str) -> Self {
        match value {
            "dot" => Icon::Dot,
            "circle-empty" => Icon::CircleEmpty,
            "dot-circled" => Icon::DotCircled,
            "certificate" => Icon::Certificate,
            "asterisk" => Icon::Asterisk,
            "speaker" => Icon::Speaker,
            "lightbulb" => Icon::Lightbulb,
            "star" => Icon::Star,
            _ => {
                log::warn!("[config.toml] Invalid icon: {value}");
                Icon::default()
            }
        }
    }
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
