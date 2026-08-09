use data::config;
use iced::widget::{button, column, container, space, text};
use iced::{Length, alignment};

use super::Message;
use crate::widget::{self, Element};
use crate::{Theme, font, theme};

pub fn view<'a>(
    payload: &'a str,
    font_config: &config::Font,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let content = column![
        column![
            text("This hyperlink will take you to"),
            text(payload)
                .style(theme::text::url)
                .font_maybe(theme::font_style::url(theme).map(font::get))
                .wrapping(text::Wrapping::Glyph)
                .width(Length::Shrink),
            space::vertical().height(8),
            text("Are you sure you want to go there?"),
        ]
        .align_x(iced::Alignment::Center)
        .spacing(2),
        column![
            button(
                container(text("Open URL"))
                    .align_x(alignment::Horizontal::Center)
                    .width(Length::Fill),
            )
            .padding(5)
            .width(widget::modal::button_width(font_config))
            .style(|theme, status| theme::button::secondary(
                theme, status, false
            ))
            .on_press(Message::OpenURL(payload.to_string())),
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
    ]
    .spacing(20)
    .align_x(iced::Alignment::Center);

    widget::modal::container(content, font_config)
        .style(theme::container::tooltip)
        .into()
}
