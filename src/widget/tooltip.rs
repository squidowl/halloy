use iced::widget::{container, text};

use crate::theme;

pub use iced::widget::tooltip::Position;

use super::Element;

pub fn tooltip<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    tooltip: Option<&'a str>,
    position: Position,
) -> Element<'a, Message> {
    match tooltip {
        Some(tooltip) => iced::widget::tooltip(
            content,
            container(text(tooltip).style(theme::text::transparent))
                .style(theme::container::context)
                .padding(8),
            position,
        )
        .into(),
        None => content.into(),
    }
}
