use data::config;
use iced::widget::{button, column, container, text};
use iced::{Length, alignment};

use super::Message;
use crate::widget::Element;
use crate::{Theme, font, theme};

pub fn view<'a>(error: &config::Error, theme: &Theme) -> Element<'a, Message> {
    container(
        column![
            text("Error reloading configuration file"),
            text(error.to_string())
                .style(theme::text::error)
                .font_maybe(font::get(theme::font_style::error(theme))),
            button(
                container(text("Close"))
                    .align_x(alignment::Horizontal::Center)
                    .width(Length::Fill),
            )
            .style(|theme, status| theme::button::secondary(
                theme, status, false
            ))
            .padding(5)
            .width(Length::Fixed(250.0))
            .on_press(Message::Cancel)
        ]
        .spacing(20)
        .align_x(iced::Alignment::Center),
    )
    .width(Length::Shrink)
    .style(theme::container::error_tooltip)
    .padding(25)
    .into()
}
