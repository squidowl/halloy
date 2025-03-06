use serde::Deserialize;

use crate::dashboard::{BufferAction, BufferFocusedAction};

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct Sidebar {
    #[serde(default, alias = "default_action")]
    pub buffer_action: BufferAction,
    #[serde(default)]
    pub buffer_focused_action: Option<BufferFocusedAction>,
    #[serde(default)]
    pub max_width: Option<u16>,
    #[serde(default)]
    pub unread_indicator: UnreadIndicator,
    #[serde(default)]
    pub position: Position,
    #[serde(default = "default_bool_true")]
    pub show_user_menu: bool,
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

impl Default for Sidebar {
    fn default() -> Self {
        Sidebar {
            buffer_action: Default::default(),
            buffer_focused_action: Default::default(),
            max_width: None,
            unread_indicator: UnreadIndicator::default(),
            position: Position::default(),
            show_user_menu: default_bool_true(),
        }
    }
}

fn default_bool_true() -> bool {
    true
}
