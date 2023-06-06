use iced::widget::pane_grid::{self, Axis};
use iced::widget::{button, container, row, text};
use iced::{Command, Length};
use uuid::Uuid;

use crate::buffer::{self, Buffer};
use crate::widget;
use crate::{icon, theme};

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Clone, Copy)]
pub struct Mapper<Message> {
    pub pane: fn(self::Message) -> Message,
    pub buffer: fn(pane_grid::Pane, buffer::Message) -> Message,
    pub on_close: Message,
    pub on_split: fn(Axis) -> Message,
    pub on_maximize: Message,
    pub on_users: Message,
}

#[derive(Clone)]
pub struct Pane {
    pub id: Uuid,
    pub buffer: Buffer,
    title_bar: TitleBar,
}

#[derive(Debug, Clone, Default)]
pub struct TitleBar {}

impl Pane {
    pub fn new(buffer: Buffer) -> Self {
        Self {
            id: Uuid::new_v4(),
            buffer,
            title_bar: TitleBar::default(),
        }
    }

    pub fn update(&mut self, _message: Message) -> Command<Message> {
        Command::none()
    }

    pub fn view<'a, M: 'static + Clone>(
        &'a self,
        mapper: Mapper<M>,
        id: pane_grid::Pane,
        panes: usize,
        is_focused: bool,
        maximized: bool,
        clients: &data::client::Map,
    ) -> widget::Content<'a, M> {
        let title_bar_text = match &self.buffer {
            Buffer::Empty(state) => state.to_string(),
            Buffer::Channel(state) => state.to_string(),
            Buffer::Server(state) => state.to_string(),
        };

        let title_bar = self.title_bar.view(
            &self.buffer,
            title_bar_text,
            &mapper,
            id,
            panes,
            is_focused,
            maximized,
        );

        let content = self
            .buffer
            .view(clients, is_focused)
            .map(move |msg| (mapper.buffer)(id, msg));

        widget::Content::new(content)
            .style(theme::Container::Pane {
                selected: is_focused,
            })
            .title_bar(title_bar.style(theme::Container::Header))
    }
}

impl TitleBar {
    fn view<'a, M: 'static + Clone>(
        &'a self,
        buffer: &Buffer,
        value: String,
        mapper: &Mapper<M>,
        _id: pane_grid::Pane,
        panes: usize,
        _is_focused: bool,
        maximized: bool,
    ) -> widget::TitleBar<'a, M> {
        // Pane controls.
        let mut controls = row![].spacing(2);

        if let Buffer::Channel(state) = &buffer {
            let users = button(
                container(icon::people())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x()
                    .center_y(),
            )
            .width(22)
            .height(22)
            .on_press(mapper.on_users.clone())
            .style(theme::Button::Selectable {
                selected: state.is_showing_users(),
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
            .on_press(mapper.on_maximize.clone())
            .style(theme::Button::Selectable {
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
            .on_press(mapper.on_close.clone())
            .style(theme::Button::Selectable { selected: false });

            controls = controls.push(delete);
        }

        let title = container(text(value))
            .height(22)
            .padding([0, 4])
            .align_y(iced::alignment::Vertical::Center);

        widget::TitleBar::new(title).controls(controls).padding(6)
    }
}
