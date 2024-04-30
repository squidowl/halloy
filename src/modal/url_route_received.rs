use iced::{
    alignment,
    widget::{button, column, container, text},
    Length,
};

use super::Message;
use crate::{theme, widget::Element};

pub fn view<'a>(route: &ipc::Route) -> Element<'a, Message> {
    container(
        column![
            text("Create new connection?"),
            text(route.to_string()).style(theme::text::info),
            column![
                button(
                    container(text("Accept"))
                        .align_x(alignment::Horizontal::Center)
                        .width(Length::Fill),
                )
                .padding(5)
                .width(Length::Fixed(250.0))
                .style(theme::button::primary)
                .on_press(Message::Accept),
                button(
                    container(text("Close"))
                        .align_x(alignment::Horizontal::Center)
                        .width(Length::Fill),
                )
                .padding(5)
                .width(Length::Fixed(250.0))
                .style(theme::button::secondary)
                .on_press(Message::Cancel),
            ]
            .spacing(4)
        ]
        .spacing(20)
        .align_items(iced::Alignment::Center),
    )
    .width(Length::Shrink)
    .style(theme::container::default_banner)
    .padding(25)
    .into()
}
