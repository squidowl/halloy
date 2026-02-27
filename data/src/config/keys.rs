use serde::Deserialize;

use crate::shortcut::{KeyBind, KeyBinds, Shortcut, shortcut};

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
        }
    }
}

impl Keyboard {
    pub fn shortcuts(&self) -> Vec<Shortcut> {
        use crate::shortcut::Command::*;

        let mut shortcuts = vec![];

        let mut push = |key_binds: &KeyBinds, command| {
            shortcuts.extend(
                key_binds
                    .iter()
                    .cloned()
                    .map(|key_bind| shortcut(key_bind, command)),
            );
        };

        push(&self.move_up, MoveUp);
        push(&self.move_down, MoveDown);
        push(&self.move_left, MoveLeft);
        push(&self.move_right, MoveRight);
        push(&self.new_horizontal_buffer, NewHorizontalBuffer);
        push(&self.new_vertical_buffer, NewVerticalBuffer);
        push(&self.close_buffer, CloseBuffer);
        push(&self.maximize_buffer, MaximizeBuffer);
        push(&self.restore_buffer, RestoreBuffer);
        push(&self.cycle_next_buffer, CycleNextBuffer);
        push(&self.cycle_previous_buffer, CyclePreviousBuffer);
        push(&self.leave_buffer, LeaveBuffer);
        push(&self.toggle_nick_list, ToggleNicklist);
        push(&self.toggle_topic, ToggleTopic);
        push(&self.toggle_sidebar, ToggleSidebar);
        push(&self.toggle_fullscreen, ToggleFullscreen);
        push(&self.command_bar, CommandBar);
        push(&self.reload_configuration, ReloadConfiguration);
        push(&self.file_transfers, FileTransfers);
        push(&self.logs, Logs);
        push(&self.theme_editor, ThemeEditor);
        push(&self.scroll_up_page, ScrollUpPage);
        push(&self.scroll_down_page, ScrollDownPage);
        push(&self.scroll_to_top, ScrollToTop);
        push(&self.scroll_to_bottom, ScrollToBottom);
        push(&self.highlights, Highlights);
        push(&self.cycle_next_unread_buffer, CycleNextUnreadBuffer);
        push(
            &self.cycle_previous_unread_buffer,
            CyclePreviousUnreadBuffer,
        );
        push(&self.mark_as_read, MarkAsRead);
        push(&self.quit_application, QuitApplication);

        shortcuts
    }
}
