use serde::Deserialize;

use crate::channel::Position;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Channel {
    pub users: Users,
}
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Users {
    pub(crate) visible: bool,
    #[serde(default)]
    pub position: Position,
}

impl Default for Users {
    fn default() -> Self {
        Self {
            visible: true,
            position: Position::default(),
        }
    }
}
