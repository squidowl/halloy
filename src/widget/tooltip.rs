pub use iced::widget::tooltip::Position;
use iced::widget::{container, text};

use super::Element;
use crate::{Theme, font, theme};

pub fn tooltip<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    tooltip: Option<&'a str>,
    position: Position,
    theme: &'a Theme,
) -> Element<'a, Message> {
    match tooltip {
        Some(tooltip) => iced::widget::tooltip(
            content,
            container(
                text(tooltip)
                    .style(theme::text::secondary)
                    .font_maybe(font::get(theme::font_style::secondary(theme))),
            )
            .style(theme::container::tooltip)
            .padding(8),
            position,
        )
        .into(),
        None => content.into(),
    }
}
