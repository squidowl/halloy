use data::config;
use iced::widget::text::Wrapping;
use iced::widget::{button, column, container, text};
use iced::{Length, alignment};

use super::Message;
use crate::widget::{self, Element};
use crate::{Theme, font, theme};

pub fn view<'a>(
    error: &config::Error,
    font_config: &config::Font,
    theme: &Theme,
) -> Element<'a, Message> {
    let content = column![
        text("Error reloading configuration file"),
        text(error.to_string())
            .style(theme::text::error)
            .font_maybe(theme::font_style::error(theme).map(font::get))
            .width(Length::Fill)
            .wrapping(Wrapping::WordOrGlyph)
            .align_x(iced::widget::text::Alignment::Center),
        button(
            container(text("Close"))
                .align_x(alignment::Horizontal::Center)
                .width(Length::Fill),
        )
        .style(|theme, status| theme::button::secondary(theme, status, false))
        .padding(5)
        .width(widget::modal::button_width(font_config))
        .on_press(Message::Cancel)
    ]
    .spacing(20)
    .align_x(iced::Alignment::Center);

    widget::modal::container(content, font_config)
        .style(theme::container::error_tooltip)
        .into()
}
