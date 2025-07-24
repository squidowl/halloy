use serde::Deserialize;

use crate::shortcut::{KeyBind, Shortcut, shortcut};

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Keyboard {
    pub move_up: KeyBind,
    pub move_down: KeyBind,
    pub move_left: KeyBind,
    pub move_right: KeyBind,
    pub close_buffer: KeyBind,
    pub maximize_buffer: KeyBind,
    pub restore_buffer: KeyBind,
    pub cycle_next_buffer: KeyBind,
    pub cycle_previous_buffer: KeyBind,
    pub leave_buffer: KeyBind,
    pub toggle_nick_list: KeyBind,
    pub toggle_topic: KeyBind,
    pub toggle_sidebar: KeyBind,
    pub toggle_fullscreen: KeyBind,
    pub command_bar: KeyBind,
    pub reload_configuration: KeyBind,
    pub file_transfers: KeyBind,
    pub logs: KeyBind,
    pub theme_editor: KeyBind,
    // Keep highlight as alias for backwards compatibility
    #[serde(alias = "highlight")]
    pub highlights: KeyBind,
    pub scroll_up_page: KeyBind,
    pub scroll_down_page: KeyBind,
    pub scroll_to_top: KeyBind,
    pub scroll_to_bottom: KeyBind,
    pub cycle_next_unread_buffer: KeyBind,
    pub cycle_previous_unread_buffer: KeyBind,
    pub mark_as_read: KeyBind,
    pub quit_application: Option<KeyBind>,
}

impl Default for Keyboard {
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
            leave_buffer: KeyBind::leave_buffer(),
            toggle_nick_list: KeyBind::toggle_nick_list(),
            toggle_sidebar: KeyBind::toggle_sidebar(),
            toggle_topic: KeyBind::toggle_topic(),
            toggle_fullscreen: KeyBind::toggle_fullscreen(),
            command_bar: KeyBind::command_bar(),
            reload_configuration: KeyBind::reload_configuration(),
            file_transfers: KeyBind::file_transfers(),
            logs: KeyBind::logs(),
            theme_editor: KeyBind::theme_editor(),
            highlights: KeyBind::highlights(),
            scroll_up_page: KeyBind::scroll_up_page(),
            scroll_down_page: KeyBind::scroll_down_page(),
            scroll_to_top: KeyBind::scroll_to_top(),
            scroll_to_bottom: KeyBind::scroll_to_bottom(),
            cycle_next_unread_buffer: KeyBind::cycle_next_unread_buffer(),
            cycle_previous_unread_buffer: KeyBind::cycle_previous_unread_buffer(
            ),
            mark_as_read: KeyBind::mark_as_read(),
            quit_application: None,
        }
    }
}

impl Keyboard {
    pub fn shortcuts(&self) -> Vec<Shortcut> {
        use crate::shortcut::Command::*;

        let mut shortcuts = vec![
            shortcut(self.move_up.clone(), MoveUp),
            shortcut(self.move_down.clone(), MoveDown),
            shortcut(self.move_left.clone(), MoveLeft),
            shortcut(self.move_right.clone(), MoveRight),
            shortcut(self.close_buffer.clone(), CloseBuffer),
            shortcut(self.maximize_buffer.clone(), MaximizeBuffer),
            shortcut(self.restore_buffer.clone(), RestoreBuffer),
            shortcut(self.cycle_next_buffer.clone(), CycleNextBuffer),
            shortcut(self.cycle_previous_buffer.clone(), CyclePreviousBuffer),
            shortcut(self.leave_buffer.clone(), LeaveBuffer),
            shortcut(self.toggle_nick_list.clone(), ToggleNicklist),
            shortcut(self.toggle_topic.clone(), ToggleTopic),
            shortcut(self.toggle_sidebar.clone(), ToggleSidebar),
            shortcut(self.toggle_fullscreen.clone(), ToggleFullscreen),
            shortcut(self.command_bar.clone(), CommandBar),
            shortcut(self.reload_configuration.clone(), ReloadConfiguration),
            shortcut(self.file_transfers.clone(), FileTransfers),
            shortcut(self.logs.clone(), Logs),
            shortcut(self.theme_editor.clone(), ThemeEditor),
            shortcut(self.scroll_up_page.clone(), ScrollUpPage),
            shortcut(self.scroll_down_page.clone(), ScrollDownPage),
            shortcut(self.scroll_to_top.clone(), ScrollToTop),
            shortcut(self.scroll_to_bottom.clone(), ScrollToBottom),
            shortcut(self.highlights.clone(), Highlights),
            shortcut(
                self.cycle_next_unread_buffer.clone(),
                CycleNextUnreadBuffer,
            ),
            shortcut(
                self.cycle_previous_unread_buffer.clone(),
                CyclePreviousUnreadBuffer,
            ),
            shortcut(self.mark_as_read.clone(), MarkAsRead),
        ];

        if let Some(quit_application) = self.quit_application.clone() {
            shortcuts.push(shortcut(quit_application, QuitApplication));
        }

        shortcuts
    }
}
