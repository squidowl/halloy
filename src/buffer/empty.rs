use core::fmt;

use iced::{alignment, pure::Element, Length};
use iced_pure::{button, column, container, horizontal_space, row, text};

use crate::{icon, style, theme::Theme};

#[derive(Debug, Clone)]
pub enum Message {
    Users, // TODO: Client has access to list_users: https://docs.rs/irc/0.15.0/irc/client/struct.Client.html#method.list_users.
    Noop,
}

pub fn view<'a>(state: &State, theme: &'a Theme) -> Element<'a, Message> {
    // TODO: Get open channels. For now just dummy hardcoded to work out UI.
    // TODO: Rewrite to function to reduce repetivness.

    let column = column()
        .spacing(1)
        .push(
            row()
                .spacing(1)
                .push(
                    button(icon::lightning())
                        .style(style::button::destruction(theme))
                        .on_press(Message::Noop),
                )
                .push(
                    button(
                        row()
                            .push(icon::house())
                            .push(horizontal_space(Length::Units(5)))
                            .push(text("irc.quakenet.org")),
                    )
                    .style(style::button::primary(theme))
                    .on_press(Message::Noop),
                ),
        )
        .push(
            row()
                .spacing(1)
                .push(
                    button(icon::door())
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
        )
        .push(
            row()
                .spacing(1)
                .push(
                    button(icon::door())
                        .style(style::button::destruction(theme))
                        .on_press(Message::Noop),
                )
                .push(
                    button(
                        row()
                            .push(icon::person())
                            .push(horizontal_space(Length::Units(5)))
                            .push(text("Tarkah")),
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

#[derive(Debug, Default, Clone)]
pub struct State {}

impl State {
    pub fn new() -> Self {
        State {}
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Noop => todo!(),
            Message::Users => todo!(),
        }
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Dashboard")
    }
}
