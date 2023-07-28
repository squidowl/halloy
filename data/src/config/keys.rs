use serde::Deserialize;

use crate::shortcut::{shortcut, KeyBind, Shortcut};

#[derive(Debug, Clone, Deserialize)]
pub struct Keys {
    #[serde(default = "KeyBind::move_up")]
    pub move_up: KeyBind,
    #[serde(default = "KeyBind::move_down")]
    pub move_down: KeyBind,
    #[serde(default = "KeyBind::move_left")]
    pub move_left: KeyBind,
    #[serde(default = "KeyBind::move_right")]
    pub move_right: KeyBind,
    #[serde(default = "KeyBind::maximize_buffer")]
    pub maximize_buffer: KeyBind,
    #[serde(default = "KeyBind::restore_buffer")]
    pub restore_buffer: KeyBind,
    #[serde(default = "KeyBind::cycle_next_buffer")]
    pub cycle_next_buffer: KeyBind,
    #[serde(default = "KeyBind::cycle_previous_buffer")]
    pub cycle_previous_buffer: KeyBind,
}

impl Default for Keys {
    fn default() -> Self {
        Self {
            move_up: KeyBind::move_up(),
            move_down: KeyBind::move_down(),
            move_left: KeyBind::move_left(),
            move_right: KeyBind::move_right(),
            maximize_buffer: KeyBind::maximize_buffer(),
            restore_buffer: KeyBind::restore_buffer(),
            cycle_next_buffer: KeyBind::cycle_next_buffer(),
            cycle_previous_buffer: KeyBind::cycle_previous_buffer(),
        }
    }
}

impl Keys {
    pub fn shortcuts(&self) -> Vec<Shortcut> {
        use crate::shortcut::Command::*;

        vec![
            shortcut(self.move_up, MoveUp),
            shortcut(self.move_down, MoveDown),
            shortcut(self.move_left, MoveLeft),
            shortcut(self.move_right, MoveRight),
            shortcut(self.maximize_buffer, MaximizeBuffer),
            shortcut(self.restore_buffer, RestoreBuffer),
            shortcut(self.cycle_next_buffer, CycleNextBuffer),
            shortcut(self.cycle_previous_buffer, CyclePreviousBuffer),
        ]
    }
}
