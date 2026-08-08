use data::config;
use iced::widget::text::Wrapping;
use iced::widget::{button, checkbox, column, container, text};
use iced::{Length, alignment};

use super::Message;
use crate::widget::{self, Element};
use crate::{Theme, font, theme};

pub fn view<'a>(
    raw: &'a str,
    config: &config::Server,
    font_config: &config::Font,
    theme: &Theme,
) -> Element<'a, Message> {
    let content = column![
        text("Connect to server?"),
        text(raw)
            .style(theme::text::tertiary)
            .font_maybe(theme::font_style::tertiary(theme).map(font::get))
            .width(Length::Fill)
            .align_x(iced::Alignment::Center)
            .wrapping(Wrapping::WordOrGlyph),
    ]
    .push(
        checkbox(config.dangerously_accept_invalid_certs)
            .on_toggle(|toggle| {
                Message::ServerConnect(
                    super::ServerConnect::DangerouslyAcceptInvalidCerts(toggle),
                )
            })
            .label("Accept invalid certificates"),
    )
    .push(
        column![
            button(
                container(text("Accept"))
                    .align_x(alignment::Horizontal::Center)
                    .width(Length::Fill),
            )
            .padding(5)
            .width(widget::modal::button_width(font_config))
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
            .width(widget::modal::button_width(font_config))
            .style(|theme, status| theme::button::secondary(
                theme, status, false
            ))
            .on_press(Message::Cancel),
        ]
        .spacing(4),
    )
    .spacing(20)
    .align_x(iced::Alignment::Center);

    widget::modal::container(content, font_config)
        .style(theme::container::tooltip)
        .into()
}
