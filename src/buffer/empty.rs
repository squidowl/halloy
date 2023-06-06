use core::fmt;

use iced::widget::{container, text};
use iced::{alignment, Length};

use crate::{theme, widget::Element};

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug, Clone)]
pub enum Event {}

pub fn view<'a>(_state: &Empty, _clients: &data::client::Map) -> Element<'a, Message> {
    container(text("Welcome to Halloy"))
        .style(theme::Container::Pane { selected: false })
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

#[derive(Debug, Default, Clone)]
pub struct Empty {}

impl Empty {
    pub fn update(&mut self, _message: Message) -> Option<Event> {
        None
    }
}

impl fmt::Display for Empty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: Not sure if this should be empty.
        write!(f, "Hello world")
    }
}
