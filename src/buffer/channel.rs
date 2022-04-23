use iced::{
    alignment,
    pure::{container, text, Element},
    Length,
};

use crate::{style, theme::Theme};

pub fn view<'a, Message: 'a>(_theme: &'a Theme) -> Element<'a, Message> {
    // TODO: Scrollable with chat messages.

    container(
        text("channel")
            .vertical_alignment(alignment::Vertical::Center)
            .horizontal_alignment(alignment::Horizontal::Center)
            .width(Length::Fill)
            .height(Length::Fill)
            .size(style::TEXT_SIZE),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
