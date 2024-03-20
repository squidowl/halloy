use serde::Deserialize;

use crate::dashboard::DefaultAction;

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct Sidebar {
    #[serde(default)]
    pub default_action: DefaultAction,
    #[serde(default = "default_sidebar_width")]
    pub width: u16,
}

impl Default for Sidebar {
    fn default() -> Self {
        Sidebar {
            default_action: Default::default(),
            width: default_sidebar_width(),
        }
    }
}

fn default_sidebar_width() -> u16 {
    120
}
