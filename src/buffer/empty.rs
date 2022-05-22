use iced::{alignment, pure::Element, Length};
use iced_pure::{button, column, container, horizontal_space, row, text, vertical_space};

use crate::{icon, style, theme::Theme};

#[derive(Debug, Clone)]
pub enum Message {
    Noop,
}

pub fn view(theme: &Theme) -> Element<Message> {
    // TODO: Get open channels. For now just dummy hardcoded to work out UI.
    // TODO: Rewrite to function to reduce repetivness.

    let column = column()
        .spacing(1)
        .push(
            row()
                .spacing(1)
                .push(
                    button(icon::close())
                        .style(style::button::destruction(theme))
                        .on_press(Message::Noop),
                )
                .push(
                    button(
                        row()
                            .push(icon::people())
                            .push(horizontal_space(Length::Units(5)))
                            .push(text("Tarkah")),
                    )
                    .style(style::button::primary(theme))
                    .on_press(Message::Noop),
                ),
        )
        .push(
            row()
                .spacing(1)
                .push(
                    button(icon::close())
                        .style(style::button::destruction(theme))
                        .on_press(Message::Noop),
                )
                .push(
                    button(
                        row()
                            .push(icon::chat())
                            .push(horizontal_space(Length::Units(5)))
                            .push(text("#rust-is-nice")),
                    )
                    .style(style::button::primary(theme))
                    .on_press(Message::Noop),
                ),
        );

    container(column)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
