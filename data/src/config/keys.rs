use std::collections::HashMap;

use serde::Deserialize;
use strum::IntoEnumIterator;

use crate::config::Error;
use crate::shortcut::{
    Command, Commands, FocusCommand, FocusCommands, KeyBind, KeyBinds,
    Shortcut, shortcut,
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
    pub show_muted_buffers: KeyBinds,
    pub hide_muted_buffers: KeyBinds,
    pub focus_up: KeyBinds,
    pub focus_down: KeyBinds,
    pub focus_left: KeyBinds,
    pub focus_right: KeyBinds,
    pub focus_activate: KeyBinds,
    pub focus_activate_alt: KeyBinds,
    pub focus_reply: KeyBinds,
    pub focus_react: KeyBinds,
    pub focus_redact: KeyBinds,
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
            show_muted_buffers: KeyBind::show_muted_buffers().into(),
            hide_muted_buffers: KeyBind::hide_muted_buffers().into(),
            focus_up: KeyBind::focus_up().into(),
            focus_down: KeyBind::focus_down().into(),
            focus_left: KeyBind::focus_left().into(),
            focus_right: KeyBind::focus_right().into(),
            focus_activate: KeyBind::focus_activate().into(),
            focus_activate_alt: KeyBind::focus_activate_alt().into(),
            focus_reply: KeyBind::focus_reply().into(),
            focus_react: KeyBind::focus_react().into(),
            focus_redact: KeyBind::focus_redact().into(),
        }
    }
}

impl Keyboard {
    fn keybind_pairs(&self) -> Vec<(&KeyBinds, Command)> {
        use Command::*;

        Command::iter()
            .map(|command| {
                let field = match command {
                    MoveUp => &self.move_up,
                    MoveDown => &self.move_down,
                    MoveLeft => &self.move_left,
                    MoveRight => &self.move_right,
                    NewHorizontalBuffer => &self.new_horizontal_buffer,
                    NewVerticalBuffer => &self.new_vertical_buffer,
                    CloseBuffer => &self.close_buffer,
                    MaximizeBuffer => &self.maximize_buffer,
                    RestoreBuffer => &self.restore_buffer,
                    CycleNextBuffer => &self.cycle_next_buffer,
                    CyclePreviousBuffer => &self.cycle_previous_buffer,
                    LeaveBuffer => &self.leave_buffer,
                    ToggleNicklist => &self.toggle_nick_list,
                    ToggleTopic => &self.toggle_topic,
                    ToggleSidebar => &self.toggle_sidebar,
                    ToggleFullscreen => &self.toggle_fullscreen,
                    CommandBar => &self.command_bar,
                    ReloadConfiguration => &self.reload_configuration,
                    FileTransfers => &self.file_transfers,
                    Logs => &self.logs,
                    ThemeEditor => &self.theme_editor,
                    ScrollUpPage => &self.scroll_up_page,
                    ScrollDownPage => &self.scroll_down_page,
                    ScrollToTop => &self.scroll_to_top,
                    ScrollToBottom => &self.scroll_to_bottom,
                    Highlights => &self.highlights,
                    CycleNextUnreadBuffer => &self.cycle_next_unread_buffer,
                    CyclePreviousUnreadBuffer => {
                        &self.cycle_previous_unread_buffer
                    }
                    MarkAsRead => &self.mark_as_read,
                    ConfigEditorSave => &self.config_editor_save,
                    QuitApplication => &self.quit_application,
                    OpenConfigEditor => &self.open_config_editor,
                    OpenConfigFile => &self.open_config_file,
                    ShowMutedBuffers => &self.show_muted_buffers,
                    HideMutedBuffers => &self.hide_muted_buffers,
                    FocusUp => &self.focus_up,
                    FocusDown => &self.focus_down,
                    FocusLeft => &self.focus_left,
                    FocusRight => &self.focus_right,
                };

                (field, command)
            })
            .collect()
    }

    fn focus_keybind_pairs(&self) -> Vec<(&KeyBinds, FocusCommand)> {
        use FocusCommand::*;

        FocusCommand::iter()
            .map(|command| {
                let field = match command {
                    Up => &self.focus_up,
                    Down => &self.focus_down,
                    Left => &self.focus_left,
                    Right => &self.focus_right,
                    Activate => &self.focus_activate,
                    ActivateAlt => &self.focus_activate_alt,
                    Reply => &self.focus_reply,
                    React => &self.focus_react,
                    Redact => &self.focus_redact,
                };

                (field, command)
            })
            .collect()
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

        let mut map: HashMap<KeyBind, Vec<FocusCommand>> = HashMap::new();

        for (focus_keybinds, focus_command) in self.focus_keybind_pairs() {
            for key in focus_keybinds.iter() {
                map.entry(key.clone()).or_default().push(focus_command);
            }
        }

        for (key, focus_commands) in map {
            if focus_commands.len() > 1 {
                return Err(Error::FocusKeyBindConflict {
                    keybind: key,
                    actions: FocusCommands::from(focus_commands),
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

    pub fn focus_command(
        &self,
        key_bind: &KeyBind,
        in_focus: bool,
    ) -> Option<FocusCommand> {
        let matches =
            |binds: &KeyBinds| binds.iter().any(|bind| bind == key_bind);

        // Focus navigation keys need to exist in both focus and non-focus
        // modes. When not in focus mode modifiers are expected and in focus
        // mode modifiers are not expected, so allow them to match without
        // modifiers when in focus mode.
        let matches_ignore_modifiers = |binds: &KeyBinds| {
            binds.iter().any(|bind| {
                bind == key_bind
                    || (in_focus && bind.eq_ignore_modifiers(key_bind))
            })
        };

        if matches_ignore_modifiers(&self.focus_up) {
            Some(FocusCommand::Up)
        } else if matches_ignore_modifiers(&self.focus_down) {
            Some(FocusCommand::Down)
        } else if matches_ignore_modifiers(&self.focus_left) {
            Some(FocusCommand::Left)
        } else if matches_ignore_modifiers(&self.focus_right) {
            Some(FocusCommand::Right)
        } else if matches(&self.focus_activate) {
            Some(FocusCommand::Activate)
        } else if matches(&self.focus_activate_alt) {
            Some(FocusCommand::ActivateAlt)
        } else if matches(&self.focus_reply) {
            Some(FocusCommand::Reply)
        } else if matches(&self.focus_react) {
            Some(FocusCommand::React)
        } else if matches(&self.focus_redact) {
            Some(FocusCommand::Redact)
        } else if in_focus {
            key_bind.builtin_focus_command()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use iced_core::keyboard;

    use super::*;

    #[test]
    fn focus_command_requires_configured_modifiers() {
        let config: Keyboard = toml::from_str(
            r#"
            focus_redact = "ctrl+x"
            "#,
        )
        .unwrap();
        let ctrl_x = KeyBind::from((
            keyboard::Key::Character("x".into()),
            keyboard::Modifiers::CTRL,
        ));
        let x = KeyBind::from((
            keyboard::Key::Character("x".into()),
            keyboard::Modifiers::default(),
        ));

        assert_eq!(
            config.focus_command(&ctrl_x, true),
            Some(FocusCommand::Redact)
        );
        assert_eq!(config.focus_command(&x, true), None);
    }
}
