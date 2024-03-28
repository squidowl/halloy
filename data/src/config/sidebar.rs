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

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct Buttons {
    #[serde(default = "default_file_transfer")]
    pub file_transfer: bool,
    #[serde(default = "default_command_bar")]
    pub command_bar: bool,
}

impl Default for Buttons {
    fn default() -> Self {
        Buttons {
            file_transfer: default_file_transfer(),
            command_bar: default_command_bar(),
        }
    }
}

fn default_sidebar_width() -> u16 {
    120
}

fn default_file_transfer() -> bool {
    true
}

fn default_command_bar() -> bool {
    true
}
