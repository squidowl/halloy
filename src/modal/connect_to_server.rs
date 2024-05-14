use data::config;
use iced::{
    alignment,
    widget::{button, checkbox, column, container, text},
    Length,
};

use super::Message;
use crate::{theme, widget::Element};

pub fn view<'a>(raw: &'a str, config: &config::Server) -> Element<'a, Message> {
    container(
        column![
            text("Connect to server?"),
            text(raw).style(theme::text::info),
        ]
        .push_maybe(config.use_tls.then(|| {
            checkbox(
                "Accept invalid certificates",
                config.dangerously_accept_invalid_certs,
            )
            .on_toggle(Message::DangerouslyAcceptInvalidCerts)
        }))
        .push(
            column![
                button(
                    container(text("Accept"))
                        .align_x(alignment::Horizontal::Center)
                        .width(Length::Fill),
                )
                .padding(5)
                .width(Length::Fixed(250.0))
                .style(theme::button::primary)
                .on_press(Message::AcceptNewServer),
                button(
                    container(text("Close"))
                        .align_x(alignment::Horizontal::Center)
                        .width(Length::Fill),
                )
                .padding(5)
                .width(Length::Fixed(250.0))
                .style(theme::button::primary)
                .on_press(Message::Cancel),
            ]
            .spacing(4),
        )
        .spacing(20)
        .align_items(iced::Alignment::Center),
    )
    .width(Length::Shrink)
    .style(theme::container::default_banner)
    .padding(25)
    .into()
}
