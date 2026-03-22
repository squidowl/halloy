use iced::widget::{button, column, container, row, span, text};
use iced::Length;

use super::Message;
use crate::widget::{Element, selectable_rich_text};
use crate::{Theme, theme};

pub fn view<'a>(
    url: &'a str,
    has_credentials: bool,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let url_color = theme.styles().buffer.url.color;
    let error_color = theme.styles().text.error.color;

    let mut spans = vec![
        span("You are about to upload to the filehost "),
        span(url).color(url_color).link(()),
        span("\nMake sure you trust this domain."),
    ];

    if has_credentials {
        spans.push(
            span("\n\nYour server credentials will be sent with this request!")
                .color(error_color),
        );
    }


    container(
        column![
            selectable_rich_text::<Message, (), (), _, _>(spans)
            .on_link(|()| Message::OpenURL(url.to_string()))
            .align_x(iced::Alignment::Center),
            row![
                button(text("Upload"))
                    .padding(5)
                    .style(|theme, status| theme::button::secondary(
                        theme, status, false
                    ))
                    .on_press(Message::ConfirmFileUpload),
                button(text("Do not"))
                    .padding(5)
                    .style(|theme, status| theme::button::secondary(
                        theme, status, false
                    ))
                    .on_press(Message::Cancel),
            ]
            .spacing(4),
        ]
        .align_x(iced::Alignment::Center)
        .spacing(12),
    )
    .max_width(400)
    .width(Length::Shrink)
    .style(theme::container::tooltip)
    .padding(25)
    .into()
}
