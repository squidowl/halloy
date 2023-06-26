use core::fmt;

use iced::widget::{column, container, text};
use iced::{alignment, Length};

use crate::widget::Element;

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug, Clone)]
pub enum Event {}

pub fn view<'a>(_state: &Empty) -> Element<'a, Message> {
    // TODO: Consider if we can completetly remove this buffer.

    let content = column![]
        .push(text("⟵ select buffer"))
        .align_items(iced::Alignment::Center);

    container(content)
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
        write!(f, "")
    }
}
