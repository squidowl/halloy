use std::collections::HashMap;

use serde::Deserialize;

use crate::config::Error;
use crate::shortcut::{
    Command, Commands, KeyBind, KeyBinds, MessageFocus, Shortcut, shortcut,
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
    pub quit_application: KeyBinds,
    pub open_config_file: KeyBinds,
    pub focus_message_up: KeyBinds,
    pub focus_message_down: KeyBinds,
    pub focus_message_actions: KeyBinds,
    pub focus_reply: KeyBinds,
    pub focus_react: KeyBinds,
    pub focus_redact_message: KeyBinds,
    pub focus_open_url: KeyBinds,
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
            quit_application: KeyBind::quit_application().into(),
            open_config_file: KeyBind::open_config_file().into(),
            focus_message_up: KeyBind::focus_message_up().into(),
            focus_message_down: KeyBind::focus_message_down().into(),
            focus_message_actions: vec![
                KeyBind::focus_message_actions(),
                KeyBind::focus_message_actions_tab(),
            ]
            .into(),
            focus_reply: KeyBind::focus_reply_message().into(),
            focus_react: KeyBind::focus_react_to_message().into(),
            focus_redact_message: KeyBind::focus_redact_message().into(),
            focus_open_url: KeyBind::focus_open_url_message().into(),
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
            (&self.quit_application, QuitApplication),
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

    /// Resolve pressed key bind to a message-focus action
    pub fn message_focus(&self, key_bind: &KeyBind) -> Option<MessageFocus> {
        let matches =
            |binds: &KeyBinds| binds.iter().any(|bind| bind == key_bind);

        if matches(&self.focus_message_up) {
            Some(MessageFocus::NavigateUp)
        } else if matches(&self.focus_message_down) {
            Some(MessageFocus::NavigateDown)
        } else if matches(&self.focus_message_actions) {
            Some(MessageFocus::OpenMenu)
        } else if matches(&self.focus_reply) {
            Some(MessageFocus::Reply)
        } else if matches(&self.focus_react) {
            Some(MessageFocus::React)
        } else if matches(&self.focus_redact_message) {
            Some(MessageFocus::Redact)
        } else if matches(&self.focus_open_url) {
            Some(MessageFocus::OpenUrl)
        } else {
            None
        }
    }
}
