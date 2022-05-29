use core::fmt;

use data::client;
use data::server::Server;
use data::theme::Theme;
use iced::{pure::Element, Length};
use iced_pure::{button, column, container, scrollable, text};

use crate::style;

#[derive(Debug, Clone)]
pub enum Message {
    Noop,
}

pub fn view<'a>(state: &State, clients: &client::Map, theme: &'a Theme) -> Element<'a, Message> {
    let users = clients.get_channel_users(&state.server, &state.channel);

    let mut column = column().width(Length::Fill).spacing(1);

    for user in users {
        column = column.push(
            button(text(user.nickname()))
                .width(Length::Fill)
                .style(style::button::secondary(theme))
                .on_press(Message::Noop),
        );
    }

    let content = scrollable(column).height(Length::Fill);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(8)
        .into()
}

#[derive(Debug, Clone)]
pub struct State {
    server: Server,
    channel: String,
}

impl State {
    pub fn new(server: Server, channel: String) -> Self {
        Self { server, channel }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Noop => todo!(),
        }
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Users in {}", self.channel)
    }
}
