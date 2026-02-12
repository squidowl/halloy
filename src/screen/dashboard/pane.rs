use data::user::ChannelUsers;
use data::{Config, file_transfer, history, preview};
use iced::Size;
use iced::widget::{button, center, container, pane_grid, row, text};

use super::sidebar;
use crate::buffer::{self, Buffer};
use crate::widget::{on_resize, tooltip};
use crate::{Theme, font, icon, theme, widget};

#[derive(Debug, Clone)]
pub enum Message {
    PaneClicked(pane_grid::Pane),
    PaneResized(pane_grid::ResizeEvent),
    PaneDragged(pane_grid::DragEvent),
    Buffer(pane_grid::Pane, buffer::Message),
    ClosePane,
    SplitPane(pane_grid::Axis),
    MaximizePane,
    ToggleShowUserList,
    ToggleShowTopic,
    Popout,
    Merge,
    ScrollToBottom,
    MarkAsRead,
    ContentResized(pane_grid::Pane, Size),
}

#[derive(Clone, Debug)]
pub struct Pane {
    pub buffer: Buffer,
    pub size: Size,
    title_bar: TitleBar,
}

#[derive(Debug, Clone, Default)]
pub struct TitleBar {}

impl Pane {
    pub fn new(buffer: Buffer) -> Self {
        Self {
            buffer,
            size: Size::default(), // Will get set initially via `Message::Resized`
            title_bar: TitleBar::default(),
        }
    }

    pub fn view<'a>(
        &'a self,
        id: pane_grid::Pane,
        panes: usize,
        is_focused: bool,
        maximized: bool,
        clients: &'a data::client::Map,
        file_transfers: &'a file_transfer::Manager,
        history: &'a history::Manager,
        previews: &'a preview::Collection,
        sidebar: &'a sidebar::Sidebar,
        config: &'a Config,
        theme: &'a Theme,
        settings: Option<&'a buffer::Settings>,
        is_popout: bool,
    ) -> widget::Content<'a, Message> {
        let title_bar_text = match &self.buffer {
            Buffer::Empty => String::new(),
            Buffer::Channel(state) => {
                let channel = state.target.as_str();
                let server = &state.server;
                if let Some(mode) =
                    clients.get_channel_mode(&state.server, &state.target)
                {
                    let users = clients
                        .get_channel_users(&state.server, &state.target)
                        .map(ChannelUsers::len)
                        .unwrap_or_default();

                    format!("{channel} ({mode}) @ {server} - {users} users")
                } else {
                    format!("{channel} @ {server}")
                }
            }
            Buffer::Server(state) => state.server.to_string(),
            Buffer::Query(state) => {
                let nick = state.target.as_str();
                let server = &state.server;

                format!("{nick} @ {server}")
            }
            Buffer::FileTransfers(_) => "File Transfers".to_string(),
            Buffer::ChannelDiscovery(state) => {
                let base = "Channel Discovery";
                if let Some(server) = state.server.as_ref() {
                    let base = format!("{base} @ {server}");
                    let channel_count = clients
                        .get_channel_discovery_manager(server)
                        .map(data::channel_discovery::Manager::amount_of_channels)
                        .unwrap_or_default();
                    if channel_count > 0 {
                        format!("{base} - {channel_count} channels")
                    } else {
                        base.to_string()
                    }
                } else {
                    base.to_string()
                }
            }
            Buffer::Logs(_) => "Logs".to_string(),
            Buffer::Highlights(_) => "Highlights".to_string(),
        };

        let title_bar = self.title_bar.view(
            &self.buffer,
            history,
            title_bar_text,
            id,
            panes,
            is_focused,
            maximized,
            clients,
            settings,
            config.tooltips,
            is_popout,
            config,
            theme,
        );

        let content = self
            .buffer
            .view(
                clients,
                file_transfers,
                history,
                previews,
                settings,
                config,
                theme,
                is_focused,
                sidebar,
            )
            .map(move |msg| Message::Buffer(id, msg));

        widget::Content::new(on_resize(content, move |size| {
            Message::ContentResized(id, size)
        }))
        .style(move |theme| theme::container::buffer(theme, is_focused))
        .title_bar(title_bar.style(theme::container::buffer_title_bar))
    }

    pub fn resource(&self) -> Option<history::Resource> {
        match &self.buffer {
            Buffer::Empty => None,
            Buffer::Channel(state) => Some(history::Resource {
                kind: history::Kind::Channel(
                    state.server.clone(),
                    state.target.clone(),
                ),
            }),
            Buffer::Server(state) => Some(history::Resource {
                kind: history::Kind::Server(state.server.clone()),
            }),
            Buffer::Query(state) => Some(history::Resource {
                kind: history::Kind::Query(
                    state.server.clone(),
                    state.target.clone(),
                ),
            }),
            Buffer::Logs(_) => Some(history::Resource::logs()),
            Buffer::Highlights(_) => Some(history::Resource::highlights()),
            Buffer::ChannelDiscovery(_) | Buffer::FileTransfers(_) => None,
        }
    }

    pub fn visible_urls(&self) -> Vec<&url::Url> {
        match &self.buffer {
            Buffer::Channel(channel) => {
                channel.scroll_view.visible_urls().collect()
            }
            Buffer::Query(query) => query.scroll_view.visible_urls().collect(),
            Buffer::Empty
            | Buffer::Server(_)
            | Buffer::FileTransfers(_)
            | Buffer::Logs(_)
            | Buffer::Highlights(_)
            | Buffer::ChannelDiscovery(_) => vec![],
        }
    }
}

impl TitleBar {
    fn view<'a>(
        &'a self,
        buffer: &Buffer,
        history: &'a history::Manager,
        value: String,
        _id: pane_grid::Pane,
        panes: usize,
        _is_focused: bool,
        maximized: bool,
        clients: &'a data::client::Map,
        settings: Option<&'a buffer::Settings>,
        show_tooltips: bool,
        is_popout: bool,
        config: &'a Config,
        theme: &'a Theme,
    ) -> widget::TitleBar<'a, Message> {
        let maybe_buffer_kind =
            buffer.data().and_then(history::Kind::from_buffer);
        let can_mark_as_read = if let Some(kind) = &maybe_buffer_kind {
            history.can_mark_as_read(kind)
        } else {
            false
        };

        // Pane controls.
        let controls = row![
            if maybe_buffer_kind.is_some() {
                let mark_as_read_button = button(center(icon::mark_as_read()))
                    .padding(5)
                    .width(22)
                    .height(22)
                    .on_press_maybe(
                        can_mark_as_read.then_some(Message::MarkAsRead),
                    )
                    .style(move |theme, status| {
                        theme::button::secondary(theme, status, false)
                    });

                let mark_as_read_button_with_tooltip = tooltip(
                    mark_as_read_button,
                    show_tooltips.then_some(if can_mark_as_read {
                        "Mark messages as read"
                    } else {
                        "No unread messages"
                    }),
                    tooltip::Position::Bottom,
                    theme,
                );
                Some(mark_as_read_button_with_tooltip)
            } else {
                None
            },
            {
                if maybe_buffer_kind.is_some() {
                    let can_scroll_to_bottom =
                        !buffer.is_scrolled_to_bottom().unwrap_or_default();
                    let scroll_to_bottom_button =
                        button(center(icon::scroll_to_bottom()))
                            .padding(5)
                            .width(22)
                            .height(22)
                            .on_press_maybe(
                                can_scroll_to_bottom
                                    .then_some(Message::ScrollToBottom),
                            )
                            .style(|theme, status| {
                                theme::button::secondary(theme, status, false)
                            });

                    let scroll_to_bottom_button_with_tooltip = tooltip(
                        scroll_to_bottom_button,
                        show_tooltips.then_some(if can_scroll_to_bottom {
                            "Scroll to bottom"
                        } else {
                            "Already at bottom"
                        }),
                        tooltip::Position::Bottom,
                        theme,
                    );
                    Some(scroll_to_bottom_button_with_tooltip)
                } else {
                    None
                }
            },
            if let Buffer::Channel(state) = &buffer {
                if let Some(topic) =
                    clients.get_channel_topic(&state.server, &state.target)
                    && topic.content.is_some()
                {
                    let topic_enabled = settings.map_or(
                        config.buffer.channel.topic_banner.enabled,
                        |settings| settings.channel.topic_banner.enabled,
                    );

                    let topic_button = button(center(icon::topic()))
                        .padding(5)
                        .width(22)
                        .height(22)
                        .on_press(Message::ToggleShowTopic)
                        .style(move |theme, status| {
                            theme::button::secondary(
                                theme,
                                status,
                                topic_enabled,
                            )
                        });

                    let topic_button_with_tooltip = tooltip(
                        topic_button,
                        show_tooltips.then_some("Topic Banner"),
                        tooltip::Position::Bottom,
                        theme,
                    );
                    Some(topic_button_with_tooltip)
                } else {
                    None
                }
            } else {
                None
            },
            if matches!(buffer, Buffer::Channel(_)) {
                let nicklist_enabled = settings.map_or(
                    config.buffer.channel.nicklist.enabled,
                    |settings| settings.channel.nicklist.enabled,
                );

                let nicklist_button = button(center(icon::people()))
                    .padding(5)
                    .width(22)
                    .height(22)
                    .on_press(Message::ToggleShowUserList)
                    .style(move |theme, status| {
                        theme::button::secondary(
                            theme,
                            status,
                            nicklist_enabled,
                        )
                    });

                let nicklist_button_with_tooltip = tooltip(
                    nicklist_button,
                    show_tooltips.then_some("Nicklist"),
                    tooltip::Position::Bottom,
                    theme,
                );
                Some(nicklist_button_with_tooltip)
            } else {
                None
            },
            if panes > 1 {
                let maximize_button = button(center(if maximized {
                    icon::restore()
                } else {
                    icon::maximize()
                }))
                .padding(5)
                .width(22)
                .height(22)
                .on_press(Message::MaximizePane)
                .style(move |theme, status| {
                    theme::button::secondary(theme, status, maximized)
                });

                let maximize_button_with_tooltip = tooltip(
                    maximize_button,
                    show_tooltips.then_some(if maximized {
                        "Restore"
                    } else {
                        "Maximize"
                    }),
                    tooltip::Position::Bottom,
                    theme,
                );
                Some(maximize_button_with_tooltip)
            } else {
                None
            },
            if is_popout {
                let merge_button = button(center(icon::popout()))
                    .padding(5)
                    .width(22)
                    .height(22)
                    .on_press(Message::Merge)
                    .style(|theme, status| {
                        theme::button::secondary(theme, status, true)
                    });

                let merge_button_with_tooltip = tooltip(
                    merge_button,
                    show_tooltips.then_some("Merge"),
                    tooltip::Position::Bottom,
                    theme,
                );
                Some(merge_button_with_tooltip)
            } else if panes > 1 {
                let popout_button = button(center(icon::popout()))
                    .padding(5)
                    .width(22)
                    .height(22)
                    .on_press(Message::Popout)
                    .style(|theme, status| {
                        theme::button::secondary(theme, status, false)
                    });

                let popout_button_with_tooltip = tooltip(
                    popout_button,
                    show_tooltips.then_some("Pop Out"),
                    tooltip::Position::Bottom,
                    theme,
                );
                Some(popout_button_with_tooltip)
            } else {
                None
            },
            if !(is_popout || panes == 1 && matches!(buffer, Buffer::Empty)) {
                let close_button = button(center(icon::cancel()))
                    .padding(5)
                    .width(22)
                    .height(22)
                    .on_press(Message::ClosePane)
                    .style(|theme, status| {
                        theme::button::secondary(theme, status, false)
                    });

                let close_button_with_tooltip = tooltip(
                    close_button,
                    show_tooltips.then_some("Close"),
                    tooltip::Position::Bottom,
                    theme,
                );
                Some(close_button_with_tooltip)
            } else {
                None
            },
        ]
        .spacing(2);

        let title = container(
            text(value)
                .style(theme::text::buffer_title_bar)
                .font_maybe(
                    theme::font_style::buffer_title_bar(theme).map(font::get),
                )
                .shaping(text::Shaping::Advanced),
        )
        .height(theme::resolve_line_height(&config.font).ceil().max(22.0))
        .padding([0, 4])
        .align_y(iced::alignment::Vertical::Center);

        widget::TitleBar::new(title)
            .controls(pane_grid::Controls::new(controls))
            .padding(6)
    }
}

impl From<Pane> for data::Pane {
    fn from(pane: Pane) -> Self {
        let buffer = match pane.buffer {
            Buffer::Empty => return data::Pane::Empty,
            Buffer::Channel(state) => data::Buffer::Upstream(
                buffer::Upstream::Channel(state.server, state.target),
            ),
            Buffer::Server(state) => {
                data::Buffer::Upstream(buffer::Upstream::Server(state.server))
            }
            Buffer::Query(state) => data::Buffer::Upstream(
                buffer::Upstream::Query(state.server, state.target),
            ),
            Buffer::FileTransfers(_) => {
                data::Buffer::Internal(buffer::Internal::FileTransfers)
            }
            Buffer::Logs(_) => data::Buffer::Internal(buffer::Internal::Logs),
            Buffer::Highlights(_) => {
                data::Buffer::Internal(buffer::Internal::Highlights)
            }
            Buffer::ChannelDiscovery(state) => data::Buffer::Internal(
                buffer::Internal::ChannelDiscovery(state.server.clone()),
            ),
        };

        data::Pane::Buffer { buffer }
    }
}
