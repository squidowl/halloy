use data::config::{self, sidebar, Config};
use data::dashboard::{BufferAction, BufferFocusedAction};
use data::{file_transfer, history, Buffer, Version};
use iced::widget::{
    self, button, center, column, container, horizontal_space, pane_grid, row, scrollable, text,
    vertical_rule, vertical_space, Column, Row, Scrollable,
};
use iced::{padding, Alignment, Length, Task};
use std::time::Duration;

use tokio::time;

use super::Panes;
use crate::widget::{context_menu, tooltip, Element};
use crate::{icon, theme, window};

const CONFIG_RELOAD_DELAY: Duration = Duration::from_secs(1);

#[derive(Debug, Clone)]
pub enum Message {
    Open(Buffer),
    Popout(Buffer),
    Focus(window::Id, pane_grid::Pane),
    Replace(window::Id, Buffer, pane_grid::Pane),
    Close(window::Id, pane_grid::Pane),
    Swap(window::Id, pane_grid::Pane, window::Id, pane_grid::Pane),
    Leave(Buffer),
    ToggleFileTransfers,
    ToggleLogs,
    ToggleCommandBar,
    ToggleThemeEditor,
    ReloadingConfigFile,
    ConfigReloaded(Result<Config, config::Error>),
    OpenReleaseWebsite,
    ReloadComplete,
}

#[derive(Debug, Clone)]
pub enum Event {
    Open(Buffer),
    Popout(Buffer),
    Focus(window::Id, pane_grid::Pane),
    Replace(window::Id, Buffer, pane_grid::Pane),
    Close(window::Id, pane_grid::Pane),
    Swap(window::Id, pane_grid::Pane, window::Id, pane_grid::Pane),
    Leave(Buffer),
    ToggleFileTransfers,
    ToggleLogs,
    ToggleCommandBar,
    ToggleThemeEditor,
    OpenReleaseWebsite,
    ConfigReloaded(Result<Config, config::Error>),
}

#[derive(Clone)]
pub struct Sidebar {
    pub hidden: bool,
    reloading_config: bool,
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new()
    }
}

impl Sidebar {
    pub fn new() -> Self {
        Self {
            hidden: false,
            reloading_config: false,
        }
    }

    pub fn toggle_visibility(&mut self) {
        self.hidden = !self.hidden
    }

    pub fn update(&mut self, message: Message) -> (Task<Message>, Option<Event>) {
        match message {
            Message::Open(source) => (Task::none(), Some(Event::Open(source))),
            Message::Popout(source) => (Task::none(), Some(Event::Popout(source))),
            Message::Focus(window, pane) => (Task::none(), Some(Event::Focus(window, pane))),
            Message::Replace(window, source, pane) => {
                (Task::none(), Some(Event::Replace(window, source, pane)))
            }
            Message::Close(window, pane) => (Task::none(), Some(Event::Close(window, pane))),
            Message::Swap(from_window, from_pane, to_window, to_pane) => (
                Task::none(),
                Some(Event::Swap(from_window, from_pane, to_window, to_pane)),
            ),
            Message::Leave(buffer) => (Task::none(), Some(Event::Leave(buffer))),
            Message::ToggleFileTransfers => (Task::none(), Some(Event::ToggleFileTransfers)),
            Message::ToggleLogs => (Task::none(), Some(Event::ToggleLogs)),
            Message::ToggleCommandBar => (Task::none(), Some(Event::ToggleCommandBar)),
            Message::ToggleThemeEditor => (Task::none(), Some(Event::ToggleThemeEditor)),
            Message::ReloadingConfigFile => {
                self.reloading_config = true;
                (Task::perform(Config::load(), Message::ConfigReloaded), None)
            }
            Message::ConfigReloaded(config) => (
                Task::perform(time::sleep(CONFIG_RELOAD_DELAY), |_| {
                    Message::ReloadComplete
                }),
                Some(Event::ConfigReloaded(config)),
            ),
            Message::OpenReleaseWebsite => (Task::none(), Some(Event::OpenReleaseWebsite)),
            Message::ReloadComplete => {
                self.reloading_config = false;
                (Task::none(), None)
            }
        }
    }

    fn menu_buttons<'a>(
        &self,
        main_window: window::Id,
        panes: &Panes,
        config: data::config::Sidebar,
        show_tooltips: bool,
        file_transfers: &'a file_transfer::Manager,
        version: &'a Version,
        theme_editor_open: bool,
    ) -> Element<'a, Message> {
        let mut menu_buttons = row![]
            .spacing(1)
            .width(Length::Shrink)
            .align_y(Alignment::End);

        let tooltip_position = match config.position {
            sidebar::Position::Top => tooltip::Position::Bottom,
            sidebar::Position::Bottom | sidebar::Position::Left | sidebar::Position::Right => {
                tooltip::Position::Top
            }
        };

        let new_button = |icon: widget::Text<'a, theme::Theme>,
                          style: fn(&theme::Theme) -> text::Style,
                          on_press: Option<Message>,
                          enabled: bool,
                          tooltip_text: &'a str|
         -> Element<'a, Message> {
            let button = button(center(icon.style(style)))
                .on_press_maybe(on_press)
                .padding(5)
                .width(22)
                .height(22)
                .style(move |theme, status| theme::button::primary(theme, status, enabled));
            tooltip(
                button,
                show_tooltips.then_some(tooltip_text),
                tooltip_position,
            )
        };

        if version.is_old() {
            menu_buttons = menu_buttons.push(new_button(
                icon::megaphone(),
                theme::text::tertiary,
                Some(Message::OpenReleaseWebsite),
                false,
                "New Halloy version is available!",
            ));
        }

        if config.buttons.reload_config {
            menu_buttons = menu_buttons.push(if self.reloading_config {
                new_button(
                    icon::checkmark(),
                    theme::text::success,
                    None,
                    self.reloading_config,
                    "Reload config file",
                )
            } else {
                new_button(
                    icon::refresh(),
                    theme::text::primary,
                    Some(Message::ReloadingConfigFile),
                    self.reloading_config,
                    "Reload config file",
                )
            });
        }

        if config.buttons.command_bar {
            menu_buttons = menu_buttons.push(new_button(
                icon::search(),
                theme::text::primary,
                Some(Message::ToggleCommandBar),
                false,
                "Command Bar",
            ));
        }

        if config.buttons.file_transfer {
            let file_transfers_open = panes
                .iter(main_window)
                .any(|(_, _, pane)| matches!(pane.buffer, crate::buffer::Buffer::FileTransfers(_)));
            menu_buttons = menu_buttons.push(new_button(
                icon::file_transfer(),
                if file_transfers.is_empty() {
                    theme::text::primary
                } else {
                    theme::text::action
                },
                Some(Message::ToggleFileTransfers),
                file_transfers_open,
                "File Transfers",
            ));
        }

        if config.buttons.theme_editor {
            menu_buttons = menu_buttons.push(new_button(
                icon::theme_editor(),
                theme::text::primary,
                Some(Message::ToggleThemeEditor),
                theme_editor_open,
                "Theme Editor",
            ));
        }

        if config.buttons.logs {
            let logs_open = panes
                .iter(main_window)
                .any(|(_, _, pane)| matches!(pane.buffer, crate::buffer::Buffer::Logs(_)));
            menu_buttons = menu_buttons.push(new_button(
                icon::logs(),
                theme::text::primary,
                Some(Message::ToggleLogs),
                logs_open,
                "Logs",
            ));
        }

        let width = if config.position.is_horizontal() {
            Length::Shrink
        } else {
            Length::Fill
        };

        container(menu_buttons)
            .width(width)
            .align_x(Alignment::Center)
            .into()
    }

    pub fn view<'a>(
        &'a self,
        clients: &data::client::Map,
        history: &'a history::Manager,
        panes: &'a Panes,
        focus: Option<(window::Id, pane_grid::Pane)>,
        config: data::config::Sidebar,
        show_tooltips: bool,
        file_transfers: &'a file_transfer::Manager,
        version: &'a Version,
        theme_editor_open: bool,
        main_window: window::Id,
    ) -> Option<Element<'a, Message>> {
        if self.hidden {
            return None;
        }

        let menu_buttons = self.menu_buttons(
            main_window,
            panes,
            config,
            show_tooltips,
            file_transfers,
            version,
            theme_editor_open,
        );

        let mut buffers = vec![];

        for (i, (server, state)) in clients.iter().enumerate() {
            match state {
                data::client::State::Disconnected => {
                    buffers.push(buffer_button(
                        main_window,
                        panes,
                        focus,
                        Buffer::Server(server.clone()),
                        false,
                        config.buffer_action,
                        config.buffer_focused_action,
                        config.position,
                        config.unread_indicator,
                        history.has_unread(server, &history::Kind::Server),
                    ));
                }
                data::client::State::Ready(connection) => {
                    buffers.push(buffer_button(
                        main_window,
                        panes,
                        focus,
                        Buffer::Server(server.clone()),
                        true,
                        config.buffer_action,
                        config.buffer_focused_action,
                        config.position,
                        config.unread_indicator,
                        history.has_unread(server, &history::Kind::Server),
                    ));

                    for channel in connection.channels() {
                        buffers.push(buffer_button(
                            main_window,
                            panes,
                            focus,
                            Buffer::Channel(server.clone(), channel.clone()),
                            true,
                            config.buffer_action,
                            config.buffer_focused_action,
                            config.position,
                            config.unread_indicator,
                            history.has_unread(server, &history::Kind::Channel(channel.clone())),
                        ));
                    }

                    let queries = history.get_unique_queries(server);
                    for user in queries {
                        buffers.push(buffer_button(
                            main_window,
                            panes,
                            focus,
                            Buffer::Query(server.clone(), user.clone()),
                            true,
                            config.buffer_action,
                            config.buffer_focused_action,
                            config.position,
                            config.unread_indicator,
                            history.has_unread(server, &history::Kind::Query(user.clone())),
                        ));
                    }

                    // Separator between servers.
                    if config.position.is_horizontal() {
                        if i + 1 < clients.len() {
                            buffers.push(
                                container(vertical_rule(1))
                                    .padding(padding::top(6))
                                    .height(20)
                                    .width(12)
                                    .align_x(Alignment::Center)
                                    .into(),
                            )
                        }
                    } else {
                        buffers.push(vertical_space().height(12).into());
                    }
                }
            }
        }

        match config.position {
            sidebar::Position::Left | sidebar::Position::Right => {
                let content = column![Scrollable::new(Column::with_children(buffers).spacing(1))
                    .direction(scrollable::Direction::Vertical(
                        iced::widget::scrollable::Scrollbar::default()
                            .width(0)
                            .scroller_width(0),
                    )),];

                let body = column![container(content).height(Length::Fill), menu_buttons];
                let padding = match config.position {
                    sidebar::Position::Left => padding::top(8).bottom(6).left(6),
                    sidebar::Position::Right => padding::top(8).bottom(6).right(6),
                    _ => iced::Padding::default(),
                };

                Some(
                    container(body)
                        .height(Length::Fill)
                        .center_x(Length::Shrink)
                        .padding(padding)
                        .max_width(config.width)
                        .into(),
                )
            }
            sidebar::Position::Top | sidebar::Position::Bottom => {
                let content = row![Scrollable::new(Row::with_children(buffers).spacing(2))
                    .direction(scrollable::Direction::Horizontal(
                        iced::widget::scrollable::Scrollbar::default()
                            .width(0)
                            .scroller_width(0),
                    )),];

                let body: Row<Message, theme::Theme> =
                    row![container(content).width(Length::Fill), menu_buttons]
                        .align_y(Alignment::Center);
                let padding = match config.position {
                    sidebar::Position::Top => padding::top(8).left(8).right(8),
                    sidebar::Position::Bottom => padding::bottom(8).left(8).right(8),
                    _ => iced::Padding::default(),
                };

                Some(
                    container(body)
                        .center_x(Length::Shrink)
                        .padding(padding)
                        .into(),
                )
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Entry {
    NewPane,
    Popout,
    Replace(window::Id, pane_grid::Pane),
    Close(window::Id, pane_grid::Pane),
    Swap(window::Id, pane_grid::Pane, window::Id, pane_grid::Pane),
    Leave,
}

impl Entry {
    fn list(
        num_panes: usize,
        open: Option<(window::Id, pane_grid::Pane)>,
        focus: Option<(window::Id, pane_grid::Pane)>,
    ) -> Vec<Self> {
        match (open, focus) {
            (None, None) => vec![Entry::NewPane, Entry::Popout, Entry::Leave],
            (None, Some(close)) => {
                vec![
                    Entry::NewPane,
                    Entry::Popout,
                    Entry::Replace(close.0, close.1),
                    Entry::Leave,
                ]
            }
            (Some(open), None) => (num_panes > 1)
                .then_some(Entry::Close(open.0, open.1))
                .into_iter()
                .chain(Some(Entry::Leave))
                .collect(),
            (Some(open), Some(focus)) => (num_panes > 1)
                .then_some(Entry::Close(open.0, open.1))
                .into_iter()
                .chain((open != focus).then_some(Entry::Swap(open.0, open.1, focus.0, focus.1)))
                .chain(Some(Entry::Leave))
                .collect(),
        }
    }
}

fn buffer_button(
    main_window: window::Id,
    panes: &Panes,
    focus: Option<(window::Id, pane_grid::Pane)>,
    buffer: Buffer,
    connected: bool,
    buffer_action: BufferAction,
    focused_buffer_action: Option<BufferFocusedAction>,
    position: sidebar::Position,
    unread_indicator: sidebar::UnreadIndicator,
    has_unread: bool,
) -> Element<Message> {
    let open = panes
        .iter(main_window)
        .find_map(|(window_id, pane, state)| {
            (state.buffer.data() == Some(&buffer)).then_some((window_id, pane))
        });
    let is_focused = panes
        .iter(main_window)
        .find_map(|(window_id, pane, state)| {
            (Some((window_id, pane)) == focus && state.buffer.data() == Some(&buffer))
                .then_some((window_id, pane))
        });

    let show_unread_indicator =
        has_unread && matches!(unread_indicator, sidebar::UnreadIndicator::Dot);
    let show_title_indicator =
        has_unread && matches!(unread_indicator, sidebar::UnreadIndicator::Title);

    let unread_dot_indicator_spacing = horizontal_space().width(match position.is_horizontal() {
        true => {
            if show_unread_indicator {
                5
            } else {
                0
            }
        }
        false => {
            if show_unread_indicator {
                11
            } else {
                16
            }
        }
    });
    let buffer_title_style = if show_title_indicator {
        theme::text::unread_indicator
    } else {
        theme::text::primary
    };

    let row = match &buffer {
        Buffer::Server(server) => row![
            icon::connected().style(if connected {
                if show_unread_indicator {
                    theme::text::unread_indicator
                } else {
                    theme::text::primary
                }
            } else {
                theme::text::error
            }),
            text(server.to_string())
                .style(buffer_title_style)
                .shaping(text::Shaping::Advanced)
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
        Buffer::Channel(_, channel) => row![]
            .push(horizontal_space().width(3))
            .push_maybe(
                show_unread_indicator
                    .then_some(icon::dot().size(6).style(theme::text::unread_indicator)),
            )
            .push(unread_dot_indicator_spacing)
            .push(
                text(channel.clone())
                    .style(buffer_title_style)
                    .shaping(text::Shaping::Advanced),
            )
            .push(horizontal_space().width(3))
            .align_y(iced::Alignment::Center),
        Buffer::Query(_, nick) => row![]
            .push(horizontal_space().width(3))
            .push_maybe(
                show_unread_indicator
                    .then_some(icon::dot().size(6).style(theme::text::unread_indicator)),
            )
            .push(unread_dot_indicator_spacing)
            .push(
                text(nick.to_string())
                    .style(buffer_title_style)
                    .shaping(text::Shaping::Advanced),
            )
            .push(horizontal_space().width(3))
            .align_y(iced::Alignment::Center),
    };

    let width = if position.is_horizontal() {
        Length::Shrink
    } else {
        Length::Fill
    };

    let base = button(row)
        .padding(5)
        .width(width)
        .style(move |theme, status| {
            theme::button::sidebar_buffer(theme, status, is_focused.is_some(), open.is_some())
        })
        .on_press_maybe({
            match is_focused {
                Some((window, pane)) => {
                    if let Some(focus_action) = focused_buffer_action {
                        match focus_action {
                            BufferFocusedAction::ClosePane => Some(Message::Close(window, pane)),
                        }
                    } else {
                        None
                    }
                }
                None => {
                    if let Some((window, pane)) = open {
                        Some(Message::Focus(window, pane))
                    } else {
                        match buffer_action {
                            BufferAction::NewPane => Some(Message::Open(buffer.clone())),
                            BufferAction::ReplacePane => match focus {
                                Some((window, pane)) => {
                                    Some(Message::Replace(window, buffer.clone(), pane))
                                }
                                None => Some(Message::Open(buffer.clone())),
                            },
                            BufferAction::NewWindow => Some(Message::Popout(buffer.clone())),
                        }
                    }
                }
            }
        });

    let entries = Entry::list(panes.len(), open, focus);

    if entries.is_empty() || !connected {
        base.into()
    } else {
        context_menu(base, entries, move |entry, length| {
            let (content, message) = match entry {
                Entry::NewPane => ("Open in new pane", Message::Open(buffer.clone())),
                Entry::Popout => ("Open in new window", Message::Popout(buffer.clone())),
                Entry::Replace(window, pane) => (
                    "Replace current pane",
                    Message::Replace(window, buffer.clone(), pane),
                ),
                Entry::Close(window, pane) => ("Close pane", Message::Close(window, pane)),
                Entry::Swap(from_window, from_pane, to_window, to_pane) => (
                    "Swap with current pane",
                    Message::Swap(from_window, from_pane, to_window, to_pane),
                ),
                Entry::Leave => (
                    match &buffer {
                        Buffer::Server(_) => "Leave server",
                        Buffer::Channel(_, _) => "Leave channel",
                        Buffer::Query(_, _) => "Close query",
                    },
                    Message::Leave(buffer.clone()),
                ),
            };

            button(text(content).style(theme::text::primary))
                .width(length)
                .padding(5)
                .on_press(message)
                .into()
        })
        .into()
    }
}
