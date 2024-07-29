use serde::Deserialize;

use crate::dashboard::DefaultAction;

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct Sidebar {
    #[serde(default)]
    pub default_action: DefaultAction,
    #[serde(default = "default_sidebar_width")]
    pub width: u16,
    #[serde(default)]
    pub buttons: Buttons,
    #[serde(default = "default_bool_true")]
    pub show_unread_indicators: bool,
}

impl Default for Sidebar {
    fn default() -> Self {
        Sidebar {
            default_action: Default::default(),
            width: default_sidebar_width(),
            buttons: Default::default(),
            show_unread_indicators: default_bool_true(),
        }
    }
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct Buttons {
    #[serde(default = "default_bool_true")]
    pub file_transfer: bool,
    #[serde(default = "default_bool_true")]
    pub command_bar: bool,
    #[serde(default = "default_bool_true")]
    pub reload_config: bool,
}

impl Default for Buttons {
    fn default() -> Self {
        Buttons {
            file_transfer: default_bool_true(),
            command_bar: default_bool_true(),
            reload_config: default_bool_true(),
        }
    }
}

fn default_sidebar_width() -> u16 {
    120
}

fn default_bool_true() -> bool {
    true
}
