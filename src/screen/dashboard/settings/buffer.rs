use iced::widget::{container, text};

use super::Message;

use crate::widget::Element;

pub fn view<'a>() -> Element<'a, Message> {
    container(text("tba")).into()
}