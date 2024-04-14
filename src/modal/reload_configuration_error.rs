use data::config;
use iced::{
    alignment,
    widget::{button, column, container, text, vertical_space},
    Length,
};

use crate::{theme, widget::Element};

#[derive(Clone, Debug)]
pub enum Message {
    CloseModal,
}

pub fn view<'a>(error: &config::Error) -> Element<'a, Message> {
    container(
        container(
            column![
                text("Error reloading configuration file"),
                vertical_space().height(20),
                text(error.to_string()).style(theme::text::error),
                vertical_space().height(20),
                button(
                    container(text("Close"))
                        .align_x(alignment::Horizontal::Center)
                        .width(Length::Fill),
                )
                .padding(5)
                .width(Length::Fixed(250.0))
                .style(theme::button::primary)
                .on_press(Message::CloseModal),
            ]
            .align_items(iced::Alignment::Center),
        )
        .width(Length::Shrink)
        .style(theme::container::error_banner)
        .padding(25),
    )
    .align_x(alignment::Horizontal::Center)
    .align_y(alignment::Vertical::Center)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
