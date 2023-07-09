use serde::{Deserialize, Serialize};

use crate::config;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Settings {
    pub users: Users,
}

impl From<config::Channel> for Settings {
    fn from(config: config::Channel) -> Self {
        Self {
            users: Users::from(config.users),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub enum Position {
    Left,
    #[default]
    Right,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Users {
    pub visible: bool,
}

impl From<config::channel::Users> for Users {
    fn from(config: config::channel::Users) -> Self {
        Users {
            visible: config.visible,
        }
    }
}

impl Default for Users {
    fn default() -> Self {
        Self { visible: true }
    }
}

impl Users {
    pub fn toggle_visibility(&mut self) {
        self.visible = !self.visible
    }
}
