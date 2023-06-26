use data::history;
use iced::widget::{button, container, pane_grid, row, text};
use iced::Length;
use uuid::Uuid;

use crate::buffer::{self, Buffer};
use crate::{icon, theme, widget};

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
}

#[derive(Clone)]
pub struct Pane {
    pub id: Uuid,
    pub buffer: Buffer,
    title_bar: TitleBar,
    settings: buffer::Settings,
}

#[derive(Debug, Clone, Default)]
pub struct TitleBar {}

impl Pane {
    pub fn new(buffer: Buffer, settings: buffer::Settings) -> Self {
        Self {
            id: Uuid::new_v4(),
            buffer,
            title_bar: TitleBar::default(),
            settings,
        }
    }

    pub fn view<'a>(
        &'a self,
        id: pane_grid::Pane,
        panes: usize,
        is_focused: bool,
        maximized: bool,
        clients: &'a data::client::Map,
        history: &'a history::Manager,
    ) -> widget::Content<'a, Message> {
        let title_bar_text = match &self.buffer {
            Buffer::Empty(state) => state.to_string(),
            Buffer::Channel(state) => state.to_string(),
            Buffer::Server(state) => state.to_string(),
            Buffer::Query(state) => state.to_string(),
        };

        let title_bar = self.title_bar.view(
            &self.buffer,
            title_bar_text,
            id,
            panes,
            is_focused,
            maximized,
            &self.settings,
        );

        let content = self
            .buffer
            .view(clients, history, &self.settings, is_focused)
            .map(move |msg| Message::Buffer(id, msg));

        widget::Content::new(content)
            .style(theme::Container::PaneBody {
                selected: is_focused,
            })
            .title_bar(title_bar.style(theme::Container::PaneHeader))
    }

    pub fn resource(&self) -> Option<history::Resource> {
        match &self.buffer {
            Buffer::Empty(_) => None,
            Buffer::Channel(channel) => Some(history::Resource {
                server: channel.server.clone(),
                kind: history::Kind::Channel(channel.channel.clone()),
            }),
            Buffer::Server(server) => Some(history::Resource {
                server: server.server.clone(),
                kind: history::Kind::Server,
            }),
            Buffer::Query(query) => Some(history::Resource {
                server: query.server.clone(),
                kind: history::Kind::Query(query.nick.clone()),
            }),
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
        settings: &'a buffer::Settings,
    ) -> widget::TitleBar<'a, Message> {
        // Pane controls.
        let mut controls = row![].spacing(2);

        if let Buffer::Channel(_) = &buffer {
            let users = button(
                container(icon::people())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x()
                    .center_y(),
            )
            .width(22)
            .height(22)
            .on_press(Message::ToggleShowUserList)
            .style(theme::Button::Pane {
                selected: settings.channel.users.visible,
            });

            controls = controls.push(users);
        }

        // If we have more than one pane open, show delete and maximize button.
        if panes > 1 {
            let maximize = button(
                container(if maximized {
                    icon::restore()
                } else {
                    icon::maximize()
                })
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x()
                .center_y(),
            )
            .width(22)
            .height(22)
            .on_press(Message::MaximizePane)
            .style(theme::Button::Pane {
                selected: maximized,
            });

            controls = controls.push(maximize);

            let delete = button(
                container(icon::close())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x()
                    .center_y(),
            )
            .width(22)
            .height(22)
            .on_press(Message::ClosePane)
            .style(theme::Button::Pane { selected: false });

            controls = controls.push(delete);
        }

        let title = container(text(value).style(theme::Text::Alpha04))
            .height(22)
            .padding([0, 4])
            .align_y(iced::alignment::Vertical::Center);

        widget::TitleBar::new(title).controls(controls).padding(6)
    }
}

impl From<Pane> for data::Pane {
    fn from(pane: Pane) -> Self {
        let buffer = match pane.buffer {
            Buffer::Empty(_) => return data::Pane::Empty,
            Buffer::Channel(state) => data::Buffer::Channel(state.server, state.channel),
            Buffer::Server(state) => data::Buffer::Server(state.server),
            Buffer::Query(state) => data::Buffer::Query(state.server, state.nick),
        };

        data::Pane::Buffer {
            buffer,
            settings: pane.settings,
        }
    }
}
