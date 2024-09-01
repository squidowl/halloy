use data::config;
use iced::{
    alignment,
    widget::{button, column, container, text},
    Length,
};

use super::Message;
use crate::{theme, widget::Element};

pub fn view<'a>(error: &config::Error) -> Element<'a, Message> {
    container(
        column![
            text("Error reloading configuration file"),
            text(error.to_string()).style(theme::text::error),
            button(
                container(text("Close"))
                    .align_x(alignment::Horizontal::Center)
                    .width(Length::Fill),
            )
            .style(|theme, status| theme::button::secondary(theme, status, false))
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
