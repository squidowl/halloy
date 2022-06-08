use core::fmt;

use data::theme::Theme;
use iced::{alignment, pure::Element, Length};
use iced_pure::{button, column, container, horizontal_space, row, text, vertical_space};

use crate::{icon, style};

#[derive(Debug, Clone)]
pub enum Message {
    Users, // TODO: Client has access to list_users: https://docs.rs/irc/0.15.0/irc/client/struct.Client.html#method.list_users.
    Noop,
}

pub fn view<'a>(
    _state: &State,
    clients: &data::client::Map,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let mut column = column().spacing(1);

    for (server, channels) in clients.get_channels().iter() {
        column = column.push(
            button(
                row()
                    .push(icon::house())
                    .push(horizontal_space(Length::Units(5)))
                    .push(text(server.to_string())),
            )
            .style(style::button::primary(theme))
            .on_press(Message::Noop),
        );

        for channel in channels {
            column = column.push(
                button(
                    row()
                        .push(icon::chat())
                        .push(horizontal_space(Length::Units(5)))
                        .push(text(channel)),
                )
                .style(style::button::primary(theme))
                .on_press(Message::Noop),
            );
        }

        column = column.push(vertical_space(Length::Units(10)));
    }

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
