use iced::widget::{column, container, text};
use iced::{alignment, Length};

use crate::widget::Element;

pub fn view<'a, Message: 'a>() -> Element<'a, Message> {
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
