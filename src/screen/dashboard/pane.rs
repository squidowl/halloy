use iced::pane_grid::Axis;
use iced::pure::widget::pane_grid::{self, Content};
use iced::pure::widget::Container;
use iced::pure::{self, button, column, container, row, scrollable, text, text_input};
use iced::{alignment, Length};
use uuid::Uuid;

use crate::buffer::{self, Buffer};
use crate::theme::Theme;
use crate::{icon, style};

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Clone, Copy)]
pub struct Mapper<Message> {
    pub pane: fn(self::Message) -> Message,
    pub buffer: fn(pane_grid::Pane, buffer::Message) -> Message,
    pub on_close: Message,
    pub on_split: fn(Axis) -> Message,
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

    pub fn update(&mut self, message: Message) {}

    pub fn view<'a, M: 'static + Clone>(
        &'a self,
        theme: &'a Theme,
        mapper: Mapper<M>,
        id: pane_grid::Pane,
        panes: usize,
        is_focused: bool,
        clients: &data::client::Map,
    ) -> Content<'a, M> {
        let title_bar = self
            .title_bar
            .view(theme, &mapper, id, panes, is_focused)
            .style(style::container::header(theme));

        let content = self
            .buffer
            .view(clients, theme)
            .map(move |msg| (mapper.buffer)(id, msg));

        pane_grid::Content::new(content)
            .title_bar(title_bar)
            .style(style::container::pane(theme, is_focused))
    }
}

impl TitleBar {
    fn view<'a, M: 'static + Clone>(
        &'a self,
        theme: &'a Theme,
        mapper: &Mapper<M>,
        id: iced::pane_grid::Pane,
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

        let mut controls = row().spacing(4).padding(4).push(split_h).push(split_v);

        if panes > 1 {
            controls = controls.push(delete);
        }

        let title = column()
            .push(
                container(text(format!("title {:?}", &id)).size(style::TITLE_SIZE))
                    .padding(4)
                    .center_y()
                    .height(Length::Units(35)),
            )
            .spacing(5);

        pane_grid::TitleBar::new(title).controls(controls)
    }
}
