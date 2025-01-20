use data::{file_transfer, history, preview, Config};
use iced::widget::{button, center, container, pane_grid, row, text};

use crate::buffer::{self, Buffer};
use crate::widget::tooltip;
use crate::window::{self, Window};
use crate::{icon, theme, widget, Theme};

use super::sidebar;

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
}

#[derive(Clone)]
pub struct Pane {
    pub buffer: Buffer,
    title_bar: TitleBar,
    pub settings: buffer::Settings,
}

#[derive(Debug, Clone, Default)]
pub struct TitleBar {}

impl Pane {
    pub fn new(buffer: Buffer, config: &Config) -> Self {
        Self::with_settings(buffer, buffer::Settings::from(config.buffer.clone()))
    }

    pub fn with_settings(buffer: Buffer, settings: buffer::Settings) -> Self {
        Self {
            buffer,
            title_bar: TitleBar::default(),
            settings,
        }
    }

    pub fn view<'a>(
        &'a self,
        id: pane_grid::Pane,
        window: window::Id,
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
        main_window: &'a Window,
    ) -> widget::Content<'a, Message> {
        let is_popout = window != main_window.id;

        let title_bar_text = match &self.buffer {
            Buffer::Empty => "".to_string(),
            Buffer::Channel(state) => {
                let channel = state.target.as_str();
                let server = &state.server;
                let users = clients
                    .get_channel_users(&state.server, &state.target)
                    .len();

                format!("{channel} @ {server} - {users} users")
            }
            Buffer::Server(state) => state.server.to_string(),
            Buffer::Query(state) => {
                let nick = state.target.as_str();
                let server = &state.server;

                format!("{nick} @ {server}")
            }
            Buffer::FileTransfers(_) => "File Transfers".to_string(),
            Buffer::Logs(_) => "Logs".to_string(),
            Buffer::Highlights(_) => "Highlights".to_string(),
        };

        let title_bar = self.title_bar.view(
            &self.buffer,
            title_bar_text,
            id,
            panes,
            is_focused,
            maximized,
            clients,
            &self.settings,
            config.tooltips,
            is_popout,
        );

        let content = self
            .buffer
            .view(
                clients,
                file_transfers,
                history,
                previews,
                &self.settings,
                config,
                theme,
                is_focused,
                sidebar,
            )
            .map(move |msg| Message::Buffer(id, msg));

        widget::Content::new(content)
            .style(move |theme| theme::container::buffer(theme, is_focused))
            .title_bar(title_bar.style(theme::container::buffer_title_bar))
    }

    pub fn resource(&self) -> Option<history::Resource> {
        match &self.buffer {
            Buffer::Empty => None,
            Buffer::Channel(state) => Some(history::Resource {
                kind: history::Kind::Channel(state.server.clone(), state.target.clone()),
            }),
            Buffer::Server(state) => Some(history::Resource {
                kind: history::Kind::Server(state.server.clone()),
            }),
            Buffer::Query(state) => Some(history::Resource {
                kind: history::Kind::Query(state.server.clone(), state.target.clone()),
            }),
            Buffer::FileTransfers(_) => None,
            Buffer::Logs(_) => Some(history::Resource::logs()),
            Buffer::Highlights(_) => Some(history::Resource::highlights()),
        }
    }

    pub fn visible_urls(&self) -> Vec<&url::Url> {
        match &self.buffer {
            Buffer::Channel(channel) => channel.scroll_view.visible_urls().collect(),
            Buffer::Query(query) => query.scroll_view.visible_urls().collect(),
            Buffer::Empty
            | Buffer::Server(_)
            | Buffer::FileTransfers(_)
            | Buffer::Logs(_)
            | Buffer::Highlights(_) => vec![],
        }
    }

    pub fn update_settings(&mut self, f: impl FnOnce(&mut buffer::Settings)) {
        f(&mut self.settings);
    }
}

impl TitleBar {
    fn view<'a>(
        &'a self,
        buffer: &Buffer,
        value: String,
        _id: pane_grid::Pane,
        panes: usize,
        _is_focused: bool,
        maximized: bool,
        clients: &'a data::client::Map,
        settings: &'a buffer::Settings,
        show_tooltips: bool,
        is_popout: bool,
    ) -> widget::TitleBar<'a, Message> {
        // Pane controls.
        let mut controls = row![].spacing(2);

        if let Buffer::Channel(state) = &buffer {
            // Show scroll-to-bottom if scrollable isnt scrolled to bottom.
            if !buffer.is_scrolled_to_bottom().unwrap_or_default() {
                let scrollable_button = button(center(icon::scroll_to_bottom()))
                    .padding(5)
                    .width(22)
                    .height(22)
                    .on_press(Message::ScrollToBottom)
                    .style(|theme, status| theme::button::secondary(theme, status, false));

                let scrollable_button_with_tooltip = tooltip(
                    scrollable_button,
                    show_tooltips.then_some("Scroll to Bottom"),
                    tooltip::Position::Bottom,
                );

                controls = controls.push(scrollable_button_with_tooltip);
            }

            // Show topic button only if there is a topic to show
            if let Some(topic) = clients.get_channel_topic(&state.server, &state.target) {
                if topic.content.is_some() {
                    let topic_button = button(center(icon::topic()))
                        .padding(5)
                        .width(22)
                        .height(22)
                        .on_press(Message::ToggleShowTopic)
                        .style(|theme, status| {
                            theme::button::secondary(theme, status, settings.channel.topic.enabled)
                        });

                    let topic_button_with_tooltip = tooltip(
                        topic_button,
                        show_tooltips.then_some("Topic Banner"),
                        tooltip::Position::Bottom,
                    );

                    controls = controls.push(topic_button_with_tooltip);
                }
            }

            let nicklist_button = button(center(icon::people()))
                .padding(5)
                .width(22)
                .height(22)
                .on_press(Message::ToggleShowUserList)
                .style(|theme, status| {
                    theme::button::secondary(theme, status, settings.channel.nicklist.enabled)
                });

            let nicklist_button_with_tooltip = tooltip(
                nicklist_button,
                show_tooltips.then_some("Nicklist"),
                tooltip::Position::Bottom,
            );

            controls = controls.push(nicklist_button_with_tooltip);
        }

        // If we have more than one pane open, show maximize button.
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
            .style(move |theme, status| theme::button::secondary(theme, status, maximized));

            let maximize_button_with_tooltip = tooltip(
                maximize_button,
                show_tooltips.then_some({
                    if maximized {
                        "Restore"
                    } else {
                        "Maximize"
                    }
                }),
                tooltip::Position::Bottom,
            );

            controls = controls.push(maximize_button_with_tooltip);
        }

        // Button to merge popout back in to main window
        if is_popout {
            let merge_button = button(center(icon::popout()))
                .padding(5)
                .width(22)
                .height(22)
                .on_press(Message::Merge)
                .style(|theme, status| theme::button::secondary(theme, status, true));

            let close_button_with_tooltip = tooltip(
                merge_button,
                show_tooltips.then_some("Merge"),
                tooltip::Position::Bottom,
            );

            controls = controls.push(close_button_with_tooltip);
        }
        // Allow pane to be pop'd out if we have >1 pane on main window
        else if panes > 1 {
            let popout_button = button(center(icon::popout()))
                .padding(5)
                .width(22)
                .height(22)
                .on_press(Message::Popout)
                .style(|theme, status| theme::button::secondary(theme, status, false));

            let close_button_with_tooltip = tooltip(
                popout_button,
                show_tooltips.then_some("Pop Out"),
                tooltip::Position::Bottom,
            );

            controls = controls.push(close_button_with_tooltip);
        }

        // Add delete as long as it's not a single empty buffer
        if !(is_popout || panes == 1 && matches!(buffer, Buffer::Empty)) {
            let close_button = button(center(icon::cancel()))
                .padding(5)
                .width(22)
                .height(22)
                .on_press(Message::ClosePane)
                .style(|theme, status| theme::button::secondary(theme, status, false));

            let close_button_with_tooltip = tooltip(
                close_button,
                show_tooltips.then_some("Close"),
                tooltip::Position::Bottom,
            );

            controls = controls.push(close_button_with_tooltip);
        }

        let title = container(
            text(value)
                .style(theme::text::buffer_title_bar)
                .shaping(text::Shaping::Advanced),
        )
        .height(22)
        .padding([0, 10])
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
            Buffer::Channel(state) => {
                data::Buffer::Upstream(buffer::Upstream::Channel(state.server, state.target))
            }
            Buffer::Server(state) => data::Buffer::Upstream(buffer::Upstream::Server(state.server)),
            Buffer::Query(state) => {
                data::Buffer::Upstream(buffer::Upstream::Query(state.server, state.target))
            }
            Buffer::FileTransfers(_) => data::Buffer::Internal(buffer::Internal::FileTransfers),
            Buffer::Logs(_) => data::Buffer::Internal(buffer::Internal::Logs),
            Buffer::Highlights(_) => data::Buffer::Internal(buffer::Internal::Highlights),
        };

        data::Pane::Buffer {
            buffer,
            settings: pane.settings,
        }
    }
}
