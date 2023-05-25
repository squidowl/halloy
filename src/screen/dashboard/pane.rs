use data::server::Server;
use iced::widget::pane_grid::{self, Axis};
use iced::widget::{button, column, container, row, text};
use iced::Length;
use uuid::Uuid;

use crate::buffer::{self, Buffer};
use crate::widget::{self};
use crate::{font, icon, theme};

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Clone, Copy)]
pub struct Mapper<Message> {
    pub pane: fn(self::Message) -> Message,
    pub buffer: fn(pane_grid::Pane, buffer::Message) -> Message,
    pub on_close: Message,
    pub on_split: fn(Axis) -> Message,
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

    pub fn _update(&mut self, _message: Message) {}

    pub fn view<'a, M: 'static + Clone>(
        &'a self,
        mapper: Mapper<M>,
        id: pane_grid::Pane,
        panes: usize,
        is_focused: bool,
        clients: &data::client::Map,
    ) -> widget::Content<'a, M> {
        let title_bar_text = match &self.buffer {
            Buffer::Empty(state) => state.to_string(),
            Buffer::Channel(state) => state.to_string(),
            Buffer::Server(state) => state.to_string(),
        };

        let title_bar =
            self.title_bar
                .view(&self.buffer, title_bar_text, &mapper, id, panes, is_focused);

        let content = self
            .buffer
            .view(clients, is_focused)
            .map(move |msg| (mapper.buffer)(id, msg));

        widget::Content::new(content).title_bar(title_bar)
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
    ) -> widget::TitleBar<'a, M> {
        // Pane controls.
        let mut controls = row![];

        // If we have more than one pane open, show delete button.
        if panes > 1 {
            let delete = button(
                container(icon::close())
                    .padding([2, 0, 0, 0])
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x()
                    .center_y(),
            )
            .width(28)
            .height(28)
            .on_press(mapper.on_close.clone())
            .style(theme::Button::Primary);

            controls = controls.push(delete);
        }

        // TODO: Re-enable show users button.
        // if let Buffer::Channel(state) = &buffer {
        //     let users = button(icon::people())
        //         .on_press(mapper.on_users.clone())
        //         .style(theme::Button::Selectable {
        //             selected: state.is_showing_users(),
        //         });

        //     controls = controls.push(users);
        // }

        let title = container(text(value).font(font::MONO))
            .height(35)
            .style(theme::Container::Header);

        widget::TitleBar::new(title)
            .controls(controls)
            .padding(6)
            .style(theme::Container::Header)
    }
}
