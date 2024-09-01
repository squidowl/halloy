use iced::widget::{column, container, text};
use iced::{alignment, Length};

use crate::widget::Element;

pub fn view<'a, Message: 'a>() -> Element<'a, Message> {
    let content = column![]
        .push(text("⟵ select buffer").shaping(text::Shaping::Advanced))
        .align_x(iced::Alignment::Center);

    container(content)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
