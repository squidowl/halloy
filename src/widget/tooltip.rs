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
            container(text(tooltip).style(theme::text::secondary).font_maybe(
                theme::font_style::secondary(theme).map(font::get),
            ))
            .style(theme::container::tooltip)
            .padding(8),
            position,
        )
        .delay(iced::time::Duration::ZERO)
        .into(),
        None => content.into(),
    }
}
