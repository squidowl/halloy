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
    #[serde(default = "KeyBind::close_buffer")]
    pub close_buffer: KeyBind,
    #[serde(default = "KeyBind::maximize_buffer")]
    pub maximize_buffer: KeyBind,
    #[serde(default = "KeyBind::restore_buffer")]
    pub restore_buffer: KeyBind,
    #[serde(default = "KeyBind::cycle_next_buffer")]
    pub cycle_next_buffer: KeyBind,
    #[serde(default = "KeyBind::cycle_previous_buffer")]
    pub cycle_previous_buffer: KeyBind,
    #[serde(default = "KeyBind::toggle_nick_list")]
    pub toggle_nick_list: KeyBind,
    #[serde(default = "KeyBind::command_bar")]
    pub command_bar: KeyBind,
}

impl Default for Keys {
    fn default() -> Self {
        Self {
            move_up: KeyBind::move_up(),
            move_down: KeyBind::move_down(),
            move_left: KeyBind::move_left(),
            move_right: KeyBind::move_right(),
            close_buffer: KeyBind::close_buffer(),
            maximize_buffer: KeyBind::maximize_buffer(),
            restore_buffer: KeyBind::restore_buffer(),
            cycle_next_buffer: KeyBind::cycle_next_buffer(),
            cycle_previous_buffer: KeyBind::cycle_previous_buffer(),
            toggle_nick_list: KeyBind::toggle_nick_list(),
            command_bar: KeyBind::command_bar(),
        }
    }
}

impl Keys {
    pub fn shortcuts(&self) -> Vec<Shortcut> {
        use crate::shortcut::Command::*;

        vec![
            shortcut(self.move_up.clone(), MoveUp),
            shortcut(self.move_down.clone(), MoveDown),
            shortcut(self.move_left.clone(), MoveLeft),
            shortcut(self.move_right.clone(), MoveRight),
            shortcut(self.close_buffer.clone(), CloseBuffer),
            shortcut(self.maximize_buffer.clone(), MaximizeBuffer),
            shortcut(self.restore_buffer.clone(), RestoreBuffer),
            shortcut(self.cycle_next_buffer.clone(), CycleNextBuffer),
            shortcut(self.cycle_previous_buffer.clone(), CyclePreviousBuffer),
            shortcut(self.toggle_nick_list.clone(), ToggleNickList),
            shortcut(self.command_bar.clone(), CommandBar),
        ]
    }
}
