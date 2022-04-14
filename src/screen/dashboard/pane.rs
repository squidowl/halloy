use iced::pane_grid::Axis;
use iced::pure::widget::pane_grid::{self, Content};
use iced::pure::{button, column, container, row, text, Element};
use iced::Length;
use iced_lazy::responsive::{self};

use crate::style;
use crate::theme::Theme;

#[derive(Debug, Clone, Copy)]
pub enum Message {}

#[derive(Clone, Copy)]
pub struct Mapper<Message> {
    pub pane: fn(self::Message) -> Message,
    pub on_close: Message,
    pub on_split: fn(Axis) -> Message,
}

#[derive(Clone)]
pub struct Pane {
    pub id: usize,
    pub responsive: responsive::State,
    title_bar: TitleBar,
}

#[derive(Debug, Clone, Default)]
pub struct TitleBar {}

impl Pane {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            responsive: responsive::State::new(),
            title_bar: TitleBar::default(),
        }
    }

    pub fn _update(&mut self, _message: Message) {}

    pub fn view<'a, M: 'static + Clone>(
        &'a self,
        theme: &'a Theme,
        mapper: Mapper<M>,
        id: pane_grid::Pane,
        is_focused: bool,
    ) -> Content<'a, M> {
        // let Pane { responsive, .. } = self;

        let title_bar = self
            .title_bar
            .view(theme, &mapper, id, is_focused)
            .style(style::container::header(theme));

        pane_grid::Content::new(container(text("content").size(style::TEXT_SIZE)).padding(4))
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
        _is_focused: bool,
    ) -> pane_grid::TitleBar<'a, M> {
        let delete: Element<M> = button(text("delete").size(style::TEXT_SIZE))
            .on_press(mapper.on_close.clone())
            .style(style::button::destruction(theme))
            .into();
        let split_h: Element<M> = button(text("horizontal").size(style::TEXT_SIZE))
            .on_press((mapper.on_split)(Axis::Horizontal))
            .style(style::button::primary(theme))
            .into();
        let split_v: Element<M> = button(text("vertical").size(style::TEXT_SIZE))
            .on_press((mapper.on_split)(Axis::Vertical))
            .style(style::button::primary(theme))
            .into();
        let controls = row()
            .push(split_h)
            .push(split_v)
            .push(delete)
            .spacing(4)
            .padding(4);
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
