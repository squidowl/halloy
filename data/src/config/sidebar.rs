use serde::Deserialize;

use crate::config::Scrollbar;
use crate::serde::default_bool_true;

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct Sidebar {
    #[serde(default)]
    pub max_width: Option<u16>,
    #[serde(default)]
    pub unread_indicator: UnreadIndicator,
    #[serde(default)]
    pub position: Position,
    #[serde(default = "default_bool_true")]
    pub show_user_menu: bool,
    #[serde(default)]
    pub order_by: OrderBy,
    #[serde(default)]
    pub scrollbar: Scrollbar,
}

#[derive(Debug, Copy, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum UnreadIndicator {
    #[default]
    Dot,
    Title,
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

impl Default for Sidebar {
    fn default() -> Self {
        Sidebar {
            max_width: None,
            unread_indicator: UnreadIndicator::default(),
            position: Position::default(),
            show_user_menu: default_bool_true(),
            order_by: OrderBy::default(),
            scrollbar: Scrollbar::default(),
        }
    }
}
