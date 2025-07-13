use iced::widget::{button, column, container, text, vertical_space};
use iced::{Length, alignment};

use super::Message;
use crate::widget::Element;
use crate::{Theme, font, theme};

pub fn view<'a>(payload: &'a str, theme: &'a Theme) -> Element<'a, Message> {
    container(
        column![
            column![
                text("This hyperlink will take you to"),
                text(payload)
                    .style(theme::text::url)
                    .font_maybe(theme::font_style::url(theme).map(font::get))
                    .wrapping(text::Wrapping::Glyph)
                    .width(Length::Shrink),
                vertical_space().height(8),
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
                .width(Length::Fixed(250.0))
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
                .width(Length::Fixed(250.0))
                .style(|theme, status| theme::button::secondary(
                    theme, status, false
                ))
                .on_press(Message::Cancel),
            ]
            .spacing(4),
        ]
        .spacing(20)
        .align_x(iced::Alignment::Center),
    )
    .max_width(400)
    .width(Length::Shrink)
    .style(theme::container::tooltip)
    .padding(25)
    .into()
}
