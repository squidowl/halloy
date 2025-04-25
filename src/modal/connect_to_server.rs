use data::config;
use iced::widget::{button, checkbox, column, container, text};
use iced::{Length, alignment};

use super::Message;
use crate::theme;
use crate::widget::Element;

pub fn view<'a>(raw: &'a str, config: &config::Server) -> Element<'a, Message> {
    container(
        column![
            text("Connect to server?"),
            text(raw).style(theme::text::tertiary),
        ]
        .push(
            checkbox(
                "Accept invalid certificates",
                config.dangerously_accept_invalid_certs,
            )
            .on_toggle(|toggle| {
                Message::ServerConnect(
                    super::ServerConnect::DangerouslyAcceptInvalidCerts(toggle),
                )
            }),
        )
        .push(
            column![
                button(
                    container(text("Accept"))
                        .align_x(alignment::Horizontal::Center)
                        .width(Length::Fill),
                )
                .padding(5)
                .width(Length::Fixed(250.0))
                .style(|theme, status| theme::button::secondary(
                    theme, status, false
                ))
                .on_press(Message::ServerConnect(
                    super::ServerConnect::AcceptNewServer
                )),
                button(
                    container(text("Close"))
                        .align_x(alignment::Horizontal::Center)
                        .width(Length::Fill),
                )
                .padding(5)
                .width(Length::Fixed(250.0))
                .style(|theme, status| theme::button::secondary(
                    theme, status, false
                ))
                .on_press(Message::Cancel),
            ]
            .spacing(4),
        )
        .spacing(20)
        .align_x(iced::Alignment::Center),
    )
    .width(Length::Shrink)
    .style(theme::container::tooltip)
    .padding(25)
    .into()
}
