use data::config::{self, sidebar, Config};
use data::dashboard::{BufferAction, BufferFocusedAction};
use data::{buffer, file_transfer, history, Version};
use iced::widget::{
    button, column, container, horizontal_rule, horizontal_space, pane_grid, row, scrollable, text,
    vertical_rule, vertical_space, Column, Row, Scrollable, Space,
};
use iced::{padding, Alignment, Length, Task};
use std::time::Duration;

use tokio::time;

use super::Panes;
use crate::widget::{context_menu, Element, Text};
use crate::{icon, theme, window};

const CONFIG_RELOAD_DELAY: Duration = Duration::from_secs(1);

#[derive(Debug, Clone)]
pub enum Message {
    Open(buffer::Upstream),
    Popout(buffer::Upstream),
    Focus(window::Id, pane_grid::Pane),
    Replace(window::Id, buffer::Upstream, pane_grid::Pane),
    Close(window::Id, pane_grid::Pane),
    Swap(window::Id, pane_grid::Pane, window::Id, pane_grid::Pane),
    Leave(buffer::Upstream),
    ToggleInternalBuffer(buffer::Internal),
    ToggleCommandBar,
    ToggleThemeEditor,
    ReloadingConfigFile,
    ConfigReloaded(Result<Config, config::Error>),
    OpenReleaseWebsite,
    OpenDocumentation,
    ReloadComplete,
    Noop,
}

#[derive(Debug, Clone)]
pub enum Event {
    Open(buffer::Upstream),
    Popout(buffer::Upstream),
    Focus(window::Id, pane_grid::Pane),
    Replace(window::Id, buffer::Upstream, pane_grid::Pane),
    Close(window::Id, pane_grid::Pane),
    Swap(window::Id, pane_grid::Pane, window::Id, pane_grid::Pane),
    Leave(buffer::Upstream),
    ToggleInternalBuffer(buffer::Internal),
    ToggleCommandBar,
    ToggleThemeEditor,
    OpenReleaseWebsite,
    OpenDocumentation,
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
            Message::ToggleInternalBuffer(buffer) => {
                (Task::none(), Some(Event::ToggleInternalBuffer(buffer)))
            }
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
            Message::Noop => (Task::none(), None),
            Message::OpenDocumentation => (Task::none(), Some(Event::OpenDocumentation)),
        }
    }

    fn user_menu_button<'a>(
        &self,
        keyboard: &'a data::config::Keyboard,
        file_transfers: &'a file_transfer::Manager,
        version: &'a Version,
    ) -> Element<'a, Message> {
        let base = button(icon::menu())
            .padding(5)
            .width(Length::Shrink)
            .on_press(Message::Noop);

        let menu = Menu::list();

        if menu.is_empty() {
            base.into()
        } else {
            context_menu(
                context_menu::MouseButton::Left,
                base,
                menu,
                move |menu, length| {
                    let context_button =
                        |title: Text<'a>,
                         keybind: Option<&data::shortcut::KeyBind>,
                         icon: Text<'a>,
                         message: Message| {
                            button(
                                row![icon.width(Length::Fixed(12.0)), title]
                                    .push_maybe(keybind.map(|kb| {
                                        text(format!("({kb})"))
                                            .shaping(text::Shaping::Advanced)
                                            .size(theme::TEXT_SIZE - 2.0)
                                            .style(theme::text::secondary)
                                    }))
                                    .spacing(8)
                                    .align_y(iced::Alignment::Center),
                            )
                            .width(length)
                            .padding(5)
                            .on_press(message)
                            .into()
                        };

                    match menu {
                        Menu::RefreshConfig => context_button(
                            text("Reload configuration"),
                            Some(&keyboard.reload_configuration),
                            icon::refresh(),
                            Message::ReloadingConfigFile,
                        ),
                        Menu::CommandBar => context_button(
                            text("Command Bar"),
                            Some(&keyboard.command_bar),
                            icon::search(),
                            Message::ToggleCommandBar,
                        ),
                        Menu::FileTransfers => context_button(
                            text("File Transfers").style(if file_transfers.is_empty() {
                                theme::text::primary
                            } else {
                                theme::text::tertiary
                            }),
                            Some(&keyboard.file_transfers),
                            icon::file_transfer().style(if file_transfers.is_empty() {
                                theme::text::primary
                            } else {
                                theme::text::tertiary
                            }),
                            Message::ToggleInternalBuffer(buffer::Internal::FileTransfers),
                        ),
                        Menu::Highlights => context_button(
                            text("Highlights"),
                            Some(&keyboard.highlight),
                            icon::highlights(),
                            Message::ToggleInternalBuffer(buffer::Internal::Highlights),
                        ),
                        Menu::Logs => context_button(
                            text("Logs"),
                            Some(&keyboard.logs),
                            icon::logs(),
                            Message::ToggleInternalBuffer(buffer::Internal::Logs),
                        ),
                        Menu::ThemeEditor => context_button(
                            text("Theme Editor"),
                            Some(&keyboard.theme_editor),
                            icon::theme_editor(),
                            Message::ToggleThemeEditor,
                        ),
                        Menu::HorizontalRule => match length {
                            Length::Fill => container(horizontal_rule(1)).padding([0, 6]).into(),
                            _ => Space::new(length, 1).into(),
                        },
                        Menu::Version => match version.is_old() {
                            true => context_button(
                                text("New version available").style(theme::text::tertiary),
                                None,
                                icon::megaphone().style(theme::text::tertiary),
                                Message::OpenReleaseWebsite,
                            ),
                            false => container(
                                text(format!("Halloy ({})", version.current))
                                    .style(theme::text::secondary),
                            )
                            .padding(5)
                            .into(),
                        },
                        Menu::Documentation => context_button(
                            text("Documentation"),
                            None,
                            icon::documentation(),
                            Message::OpenDocumentation,
                        ),
                    }
                },
            )
            .into()
        }
    }

    pub fn view<'a>(
        &'a self,
        clients: &data::client::Map,
        history: &'a history::Manager,
        panes: &'a Panes,
        focus: Option<(window::Id, pane_grid::Pane)>,
        config: data::config::Sidebar,
        keyboard: &'a data::config::Keyboard,
        file_transfers: &'a file_transfer::Manager,
        version: &'a Version,
        main_window: window::Id,
    ) -> Option<Element<'a, Message>> {
        if self.hidden {
            return None;
        }

        let user_menu_button = config
            .show_user_menu
            .then(|| self.user_menu_button(keyboard, file_transfers, version));

        let mut buffers = vec![];

        for (i, (server, state)) in clients.iter().enumerate() {
            match state {
                data::client::State::Disconnected => {
                    buffers.push(upstream_buffer_button(
                        main_window,
                        panes,
                        focus,
                        buffer::Upstream::Server(server.clone()),
                        false,
                        config.buffer_action,
                        config.buffer_focused_action,
                        config.position,
                        config.unread_indicator,
                        history.has_unread(&history::Kind::Server(server.clone())),
                    ));
                }
                data::client::State::Ready(connection) => {
                    buffers.push(upstream_buffer_button(
                        main_window,
                        panes,
                        focus,
                        buffer::Upstream::Server(server.clone()),
                        true,
                        config.buffer_action,
                        config.buffer_focused_action,
                        config.position,
                        config.unread_indicator,
                        history.has_unread(&history::Kind::Server(server.clone())),
                    ));

                    for channel in connection.channels() {
                        buffers.push(upstream_buffer_button(
                            main_window,
                            panes,
                            focus,
                            buffer::Upstream::Channel(server.clone(), channel.clone()),
                            true,
                            config.buffer_action,
                            config.buffer_focused_action,
                            config.position,
                            config.unread_indicator,
                            history.has_unread(&history::Kind::Channel(
                                server.clone(),
                                channel.clone(),
                            )),
                        ));
                    }

                    let queries = history.get_unique_queries(server);
                    for query in queries {
                        let query = clients.resolve_query(server, query).unwrap_or(query);

                        buffers.push(upstream_buffer_button(
                            main_window,
                            panes,
                            focus,
                            buffer::Upstream::Query(server.clone(), query.clone()),
                            true,
                            config.buffer_action,
                            config.buffer_focused_action,
                            config.position,
                            config.unread_indicator,
                            history
                                .has_unread(&history::Kind::Query(server.clone(), query.clone())),
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

                let body =
                    column![container(content).height(Length::Fill)].push_maybe(user_menu_button);
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

                let body: Row<Message, theme::Theme> = row![container(content).width(Length::Fill)]
                    .push_maybe(user_menu_button)
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
enum Menu {
    RefreshConfig,
    CommandBar,
    ThemeEditor,
    Highlights,
    Logs,
    FileTransfers,
    Version,
    HorizontalRule,
    Documentation,
}

impl Menu {
    fn list() -> Vec<Self> {
        vec![
            Menu::Version,
            Menu::HorizontalRule,
            Menu::CommandBar,
            Menu::FileTransfers,
            Menu::Highlights,
            Menu::Logs,
            Menu::RefreshConfig,
            Menu::ThemeEditor,
            Menu::Documentation,
        ]
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

fn upstream_buffer_button(
    main_window: window::Id,
    panes: &Panes,
    focus: Option<(window::Id, pane_grid::Pane)>,
    buffer: buffer::Upstream,
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
            (state.buffer.upstream() == Some(&buffer)).then_some((window_id, pane))
        });
    let is_focused = panes
        .iter(main_window)
        .find_map(|(window_id, pane, state)| {
            (Some((window_id, pane)) == focus && state.buffer.upstream() == Some(&buffer))
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
        buffer::Upstream::Server(server) => row![
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
        buffer::Upstream::Channel(_, channel) => row![]
            .push(horizontal_space().width(3))
            .push_maybe(
                show_unread_indicator
                    .then_some(icon::dot().size(6).style(theme::text::unread_indicator)),
            )
            .push(unread_dot_indicator_spacing)
            .push(
                text(channel.to_string())
                    .style(buffer_title_style)
                    .shaping(text::Shaping::Advanced),
            )
            .push(horizontal_space().width(3))
            .align_y(iced::Alignment::Center),
        buffer::Upstream::Query(_, query) => row![]
            .push(horizontal_space().width(3))
            .push_maybe(
                show_unread_indicator
                    .then_some(icon::dot().size(6).style(theme::text::unread_indicator)),
            )
            .push(unread_dot_indicator_spacing)
            .push(
                text(query.to_string())
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
        context_menu(Default::default(), base, entries, move |entry, length| {
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
                        buffer::Upstream::Server(_) => "Leave server",
                        buffer::Upstream::Channel(_, _) => "Leave channel",
                        buffer::Upstream::Query(_, _) => "Close query",
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
