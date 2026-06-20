use std::collections::HashMap;

use serde::Deserialize;

use crate::config::Error;
use crate::shortcut::{
    Command, Commands, KeyBind, KeyBinds, Shortcut, shortcut,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Keyboard {
    pub move_up: KeyBinds,
    pub move_down: KeyBinds,
    pub move_left: KeyBinds,
    pub move_right: KeyBinds,
    pub new_horizontal_buffer: KeyBinds,
    pub new_vertical_buffer: KeyBinds,
    pub close_buffer: KeyBinds,
    pub maximize_buffer: KeyBinds,
    pub restore_buffer: KeyBinds,
    pub cycle_next_buffer: KeyBinds,
    pub cycle_previous_buffer: KeyBinds,
    pub leave_buffer: KeyBinds,
    pub toggle_nick_list: KeyBinds,
    pub toggle_topic: KeyBinds,
    pub toggle_sidebar: KeyBinds,
    pub toggle_fullscreen: KeyBinds,
    pub command_bar: KeyBinds,
    pub reload_configuration: KeyBinds,
    pub file_transfers: KeyBinds,
    pub logs: KeyBinds,
    pub theme_editor: KeyBinds,
    // Keep highlight as alias for backwards compatibility
    #[serde(alias = "highlight")]
    pub highlights: KeyBinds,
    pub scroll_up_page: KeyBinds,
    pub scroll_down_page: KeyBinds,
    pub scroll_to_top: KeyBinds,
    pub scroll_to_bottom: KeyBinds,
    pub cycle_next_unread_buffer: KeyBinds,
    pub cycle_previous_unread_buffer: KeyBinds,
    pub mark_as_read: KeyBinds,
    pub config_editor_save: KeyBinds,
    pub quit_application: KeyBinds,
    pub open_config_editor: KeyBinds,
    pub open_config_file: KeyBinds,
}

impl Default for Keyboard {
    fn default() -> Self {
        Self {
            move_up: KeyBind::move_up().into(),
            move_down: KeyBind::move_down().into(),
            move_left: KeyBind::move_left().into(),
            move_right: KeyBind::move_right().into(),
            new_horizontal_buffer: KeyBind::new_horizontal_buffer().into(),
            new_vertical_buffer: KeyBind::new_vertical_buffer().into(),
            close_buffer: KeyBind::close_buffer().into(),
            maximize_buffer: KeyBind::maximize_buffer().into(),
            restore_buffer: KeyBind::restore_buffer().into(),
            cycle_next_buffer: KeyBind::cycle_next_buffer().into(),
            cycle_previous_buffer: KeyBind::cycle_previous_buffer().into(),
            leave_buffer: KeyBind::leave_buffer().into(),
            toggle_nick_list: KeyBind::toggle_nick_list().into(),
            toggle_sidebar: KeyBind::toggle_sidebar().into(),
            toggle_topic: KeyBind::toggle_topic().into(),
            toggle_fullscreen: KeyBind::toggle_fullscreen().into(),
            command_bar: KeyBind::command_bar().into(),
            reload_configuration: KeyBind::reload_configuration().into(),
            file_transfers: KeyBind::file_transfers().into(),
            logs: KeyBind::logs().into(),
            theme_editor: KeyBind::theme_editor().into(),
            highlights: KeyBind::highlights().into(),
            scroll_up_page: KeyBind::scroll_up_page().into(),
            scroll_down_page: KeyBind::scroll_down_page().into(),
            scroll_to_top: KeyBind::scroll_to_top().into(),
            scroll_to_bottom: KeyBind::scroll_to_bottom().into(),
            cycle_next_unread_buffer: KeyBind::cycle_next_unread_buffer()
                .into(),
            cycle_previous_unread_buffer:
                KeyBind::cycle_previous_unread_buffer().into(),
            mark_as_read: KeyBind::mark_as_read().into(),
            config_editor_save: KeyBind::config_editor_save().into(),
            quit_application: KeyBind::quit_application().into(),
            open_config_editor: KeyBind::open_config_editor().into(),
            open_config_file: KeyBind::open_config_file().into(),
        }
    }
}

impl Keyboard {
    fn keybind_pairs(&self) -> Vec<(&KeyBinds, Command)> {
        use Command::*;
        vec![
            (&self.move_up, MoveUp),
            (&self.move_down, MoveDown),
            (&self.move_left, MoveLeft),
            (&self.move_right, MoveRight),
            (&self.new_horizontal_buffer, NewHorizontalBuffer),
            (&self.new_vertical_buffer, NewVerticalBuffer),
            (&self.close_buffer, CloseBuffer),
            (&self.maximize_buffer, MaximizeBuffer),
            (&self.restore_buffer, RestoreBuffer),
            (&self.cycle_next_buffer, CycleNextBuffer),
            (&self.cycle_previous_buffer, CyclePreviousBuffer),
            (&self.leave_buffer, LeaveBuffer),
            (&self.toggle_nick_list, ToggleNicklist),
            (&self.toggle_topic, ToggleTopic),
            (&self.toggle_sidebar, ToggleSidebar),
            (&self.toggle_fullscreen, ToggleFullscreen),
            (&self.command_bar, CommandBar),
            (&self.reload_configuration, ReloadConfiguration),
            (&self.file_transfers, FileTransfers),
            (&self.logs, Logs),
            (&self.theme_editor, ThemeEditor),
            (&self.scroll_up_page, ScrollUpPage),
            (&self.scroll_down_page, ScrollDownPage),
            (&self.scroll_to_top, ScrollToTop),
            (&self.scroll_to_bottom, ScrollToBottom),
            (&self.highlights, Highlights),
            (&self.cycle_next_unread_buffer, CycleNextUnreadBuffer),
            (
                &self.cycle_previous_unread_buffer,
                CyclePreviousUnreadBuffer,
            ),
            (&self.mark_as_read, MarkAsRead),
            (&self.config_editor_save, ConfigEditorSave),
            (&self.quit_application, QuitApplication),
            (&self.open_config_editor, OpenConfigEditor),
            (&self.open_config_file, OpenConfigFile),
        ]
    }

    pub fn validate(&self) -> Result<(), Error> {
        let mut map: HashMap<KeyBind, Vec<Command>> = HashMap::new();

        for (keybinds, command) in self.keybind_pairs() {
            for key in keybinds.iter() {
                map.entry(key.clone()).or_default().push(command);
            }
        }

        for (key, commands) in map {
            if commands.len() > 1 {
                return Err(Error::KeyBindConflict {
                    keybind: key,
                    actions: Commands::from(commands),
                });
            }
        }

        Ok(())
    }

    pub fn shortcuts(&self) -> Vec<Shortcut> {
        self.keybind_pairs()
            .into_iter()
            .flat_map(|(keybinds, command)| {
                keybinds
                    .iter()
                    .cloned()
                    .map(move |key_bind| shortcut(key_bind, command))
            })
            .collect()
    }
}
