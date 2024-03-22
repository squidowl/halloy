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
}

impl Default for Sidebar {
    fn default() -> Self {
        Sidebar {
            default_action: Default::default(),
            width: default_sidebar_width(),
            buttons: Default::default(),
        }
    }
}

#[derive(Debug, Copy, Clone, Deserialize, Default)]
pub struct Buttons {
    #[serde(default)]
    pub file_transfer: bool,
    #[serde(default)]
    pub command_bar: bool,
}

fn default_sidebar_width() -> u16 {
    120
}
