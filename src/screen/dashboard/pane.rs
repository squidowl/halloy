use data::server::Server;
use iced::pane_grid::Axis;
use iced::pure::widget::pane_grid::{self, Content};
use iced::pure::{button, column, container, row, text};
use iced::Length;
use uuid::Uuid;

use crate::buffer::{self, Buffer};
use crate::theme::Theme;
use crate::{font, icon, style};

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Clone, Copy)]
pub struct Mapper<Message> {
    pub pane: fn(self::Message) -> Message,
    pub buffer: fn(pane_grid::Pane, buffer::Message) -> Message,
    pub on_close: Message,
    pub on_split: fn(Axis) -> Message,
    pub on_users: fn(Server, String) -> Message,
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
        theme: &'a Theme,
        mapper: Mapper<M>,
        id: pane_grid::Pane,
        panes: usize,
        is_focused: bool,
        clients: &data::client::Map,
    ) -> Content<'a, M> {
        let title_bar_text = match &self.buffer {
            Buffer::Empty(state) => state.to_string(),
            Buffer::Channel(state) => state.to_string(),
            Buffer::Server(state) => state.to_string(),
            Buffer::Users(state) => state.to_string(),
        };

        let title_bar = self
            .title_bar
            .view(
                &self.buffer,
                title_bar_text,
                theme,
                &mapper,
                id,
                panes,
                is_focused,
            )
            .style(style::container::header(theme));

        let content = self
            .buffer
            .view(clients, is_focused, theme)
            .map(move |msg| (mapper.buffer)(id, msg));

        pane_grid::Content::new(content)
            .title_bar(title_bar)
            .style(style::container::pane(theme, is_focused))
    }
}

impl TitleBar {
    fn view<'a, M: 'static + Clone>(
        &'a self,
        buffer: &Buffer,
        value: String,
        theme: &'a Theme,
        mapper: &Mapper<M>,
        _id: iced::pane_grid::Pane,
        panes: usize,
        _is_focused: bool,
    ) -> pane_grid::TitleBar<'a, M> {
        let delete = button(icon::close())
            .style(style::button::destruction(theme))
            .on_press(mapper.on_close.clone());
        let split_h = button(icon::box_arrow_right())
            .on_press((mapper.on_split)(Axis::Horizontal))
            .style(style::button::primary(theme));
        let split_v = button(icon::box_arrow_down())
            .on_press((mapper.on_split)(Axis::Vertical))
            .style(style::button::primary(theme));

        let mut controls = row().spacing(4).padding(4);

        if let Buffer::Channel(state) = &buffer {
            let users = button(icon::people())
                .on_press((mapper.on_users)(
                    state.server().clone(),
                    state.channel().to_string(),
                ))
                .style(style::button::primary(theme));

            controls = controls.push(users);
        }

        controls = controls.push(split_h).push(split_v);

        if panes > 1 {
            controls = controls.push(delete);
        }

        let title = column()
            .push(
                container(text(value).font(font::BOLD).size(style::TEXT_SIZE))
                    .padding([0, 8])
                    .center_y()
                    .height(Length::Units(35)),
            )
            .spacing(5);

        pane_grid::TitleBar::new(title).controls(controls)
    }
}
