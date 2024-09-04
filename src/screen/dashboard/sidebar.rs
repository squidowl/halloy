use std::time::{Duration, Instant};

use data::config::sidebar;
use data::dashboard::{BufferAction, BufferFocusedAction};
use data::{file_transfer, history, Buffer, Version};
use iced::widget::{
    button, center, column, container, horizontal_space, pane_grid, row, scrollable, text,
    vertical_rule, vertical_space, Column, Row, Scrollable,
};
use iced::{padding, Alignment, Length};

use super::pane::Pane;
use crate::widget::{context_menu, tooltip, Element};
use crate::{icon, theme};

#[derive(Debug, Clone)]
pub enum Message {
    Open(Buffer),
    Replace(Buffer, pane_grid::Pane),
    Close(pane_grid::Pane),
    Swap(pane_grid::Pane, pane_grid::Pane),
    Leave(Buffer),
    ToggleFileTransfers,
    ToggleCommandBar,
    ReloadConfigFile,
    OpenReleaseWebsite,
}

#[derive(Debug, Clone)]
pub enum Event {
    Open(Buffer),
    Replace(Buffer, pane_grid::Pane),
    Close(pane_grid::Pane),
    Swap(pane_grid::Pane, pane_grid::Pane),
    Leave(Buffer),
    ToggleFileTransfers,
    ToggleCommandBar,
    ReloadConfigFile,
    OpenReleaseWebsite,
}

#[derive(Clone)]
pub struct Sidebar {
    pub hidden: bool,
    timestamp: Option<Instant>,
}

impl Sidebar {
    pub fn new() -> Self {
        Self {
            hidden: false,
            timestamp: None,
        }
    }

    pub fn toggle_visibility(&mut self) {
        self.hidden = !self.hidden
    }

    pub fn update(&mut self, message: Message) -> Event {
        match message {
            Message::Open(source) => Event::Open(source),
            Message::Replace(source, pane) => Event::Replace(source, pane),
            Message::Close(pane) => Event::Close(pane),
            Message::Swap(from, to) => Event::Swap(from, to),
            Message::Leave(buffer) => Event::Leave(buffer),
            Message::ToggleFileTransfers => Event::ToggleFileTransfers,
            Message::ToggleCommandBar => Event::ToggleCommandBar,
            Message::ReloadConfigFile => {
                self.timestamp = Some(Instant::now());
                Event::ReloadConfigFile
            }
            Message::OpenReleaseWebsite => Event::OpenReleaseWebsite,
        }
    }

    pub fn view<'a>(
        &'a self,
        now: Instant,
        clients: &data::client::Map,
        history: &'a history::Manager,
        panes: &pane_grid::State<Pane>,
        focus: Option<pane_grid::Pane>,
        config: data::config::Sidebar,
        show_tooltips: bool,
        file_transfers: &'a file_transfer::Manager,
        version: &'a Version,
    ) -> Option<Element<'a, Message>> {
        if self.hidden {
            return None;
        }

        let menu_buttons = menu_buttons(
            now,
            self.timestamp,
            panes,
            config,
            show_tooltips,
            file_transfers,
            version,
        );

        let mut buffers = vec![];

        for (i, (server, state)) in clients.iter().enumerate() {
            match state {
                data::client::State::Disconnected => {
                    buffers.push(buffer_button(
                        panes,
                        focus,
                        Buffer::Server(server.clone()),
                        false,
                        config.buffer_action,
                        config.buffer_focused_action,
                        config.position,
                        config.unread_indicator,
                        false,
                    ));
                }
                data::client::State::Ready(connection) => {
                    buffers.push(buffer_button(
                        panes,
                        focus,
                        Buffer::Server(server.clone()),
                        true,
                        config.buffer_action,
                        config.buffer_focused_action,
                        config.position,
                        config.unread_indicator,
                        false,
                    ));

                    for channel in connection.channels() {
                        buffers.push(buffer_button(
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
    Replace(pane_grid::Pane),
    Close(pane_grid::Pane),
    Swap(pane_grid::Pane, pane_grid::Pane),
    Leave,
}

impl Entry {
    fn list(
        num_panes: usize,
        open: Option<pane_grid::Pane>,
        focus: Option<pane_grid::Pane>,
    ) -> Vec<Self> {
        match (open, focus) {
            (None, None) => vec![Entry::NewPane, Entry::Leave],
            (None, Some(focus)) => vec![Entry::NewPane, Entry::Replace(focus), Entry::Leave],
            (Some(open), None) => (num_panes > 1)
                .then_some(Entry::Close(open))
                .into_iter()
                .chain(Some(Entry::Leave))
                .collect(),
            (Some(open), Some(focus)) => (num_panes > 1)
                .then_some(Entry::Close(open))
                .into_iter()
                .chain((open != focus).then_some(Entry::Swap(open, focus)))
                .chain(Some(Entry::Leave))
                .collect(),
        }
    }
}

fn buffer_button<'a>(
    panes: &pane_grid::State<Pane>,
    focus: Option<pane_grid::Pane>,
    buffer: Buffer,
    connected: bool,
    buffer_action: BufferAction,
    focused_buffer_action: Option<BufferFocusedAction>,
    position: sidebar::Position,
    unread_indicator: sidebar::UnreadIndicator,
    has_unread: bool,
) -> Element<'a, Message> {
    let open = panes
        .iter()
        .find_map(|(pane, state)| (state.buffer.data().as_ref() == Some(&buffer)).then_some(*pane));
    let is_focused = panes
        .iter()
        .find(|(id, _)| Some(**id) == focus)
        .and_then(|(id, pane)| {
            if pane.buffer.data().as_ref() == Some(&buffer) {
                Some(*id)
            } else {
                None
            }
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
                theme::text::primary
            } else {
                theme::text::error
            }),
            text(server.to_string())
                .style(theme::text::primary)
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
        .on_press({
            match is_focused {
                Some(id) => {
                    if let Some(focus_action) = focused_buffer_action {
                        match focus_action {
                            BufferFocusedAction::ClosePane => Message::Close(id),
                        }
                    } else {
                        Message::Open(buffer.clone())
                    }
                }
                None => match buffer_action {
                    BufferAction::NewPane => Message::Open(buffer.clone()),
                    BufferAction::ReplacePane => match focus {
                        Some(pane) => Message::Replace(buffer.clone(), pane),
                        None => Message::Open(buffer.clone()),
                    },
                },
            }
        });

    let entries = Entry::list(panes.len(), open, focus);

    if entries.is_empty() || !connected {
        base.into()
    } else {
        context_menu(base, entries, move |entry, length| {
            let (content, message) = match entry {
                Entry::NewPane => ("Open in new pane", Message::Open(buffer.clone())),
                Entry::Replace(pane) => (
                    "Replace current pane",
                    Message::Replace(buffer.clone(), pane),
                ),
                Entry::Close(pane) => ("Close pane", Message::Close(pane)),
                Entry::Swap(from, to) => ("Swap with current pane", Message::Swap(from, to)),
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
    }
}

fn menu_buttons<'a>(
    now: Instant,
    timestamp: Option<Instant>,
    panes: &pane_grid::State<Pane>,
    config: data::config::Sidebar,
    show_tooltips: bool,
    file_transfers: &'a file_transfer::Manager,
    version: &'a Version,
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

    if version.is_old() {
        let button = button(center(icon::megaphone().style(theme::text::tertiary)))
            .on_press(Message::OpenReleaseWebsite)
            .padding(5)
            .width(22)
            .height(22)
            .style(|theme, status| theme::button::primary(theme, status, false));

        let button_with_tooltip = tooltip(
            button,
            show_tooltips.then_some("New Halloy version is available!"),
            tooltip_position,
        );

        menu_buttons = menu_buttons.push(button_with_tooltip);
    }

    if config.buttons.reload_config {
        let icon = timestamp
            .filter(|&timestamp| now.saturating_duration_since(timestamp) < Duration::new(1, 0))
            .map_or_else(
                || icon::refresh().style(theme::text::primary),
                |_| icon::checkmark().style(theme::text::success),
            );

        let button = button(center(icon))
            .on_press(Message::ReloadConfigFile)
            .padding(5)
            .width(22)
            .height(22)
            .style(|theme, status| theme::button::primary(theme, status, false));

        let button_with_tooltip = tooltip(
            button,
            show_tooltips.then_some("Reload config file"),
            tooltip_position,
        );

        menu_buttons = menu_buttons.push(button_with_tooltip);
    }

    if config.buttons.command_bar {
        let button = button(center(icon::search()))
            .on_press(Message::ToggleCommandBar)
            .padding(5)
            .width(22)
            .height(22)
            .style(|theme, status| theme::button::primary(theme, status, false));

        let button_with_tooltip = tooltip(
            button,
            show_tooltips.then_some("Command Bar"),
            tooltip_position,
        );

        menu_buttons = menu_buttons.push(button_with_tooltip);
    }

    if config.buttons.file_transfer {
        let file_transfers_open = panes
            .iter()
            .any(|(_, pane)| matches!(pane.buffer, crate::buffer::Buffer::FileTransfers(_)));

        let button = button(center(icon::file_transfer().style(
            if file_transfers.is_empty() {
                theme::text::primary
            } else {
                theme::text::action
            },
        )))
        .on_press(Message::ToggleFileTransfers)
        .padding(5)
        .width(22)
        .height(22)
        .style(move |theme, status| theme::button::primary(theme, status, file_transfers_open));

        let button_with_tooltip = tooltip(
            button,
            show_tooltips.then_some("File Transfers"),
            tooltip_position,
        );

        menu_buttons = menu_buttons.push(button_with_tooltip);
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
