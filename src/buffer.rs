use iced::pure::Element;

use crate::theme::Theme;

pub mod channel;
pub mod empty;
pub mod server;
pub mod users;

#[derive(Clone)]
pub enum Buffer {
    Empty(empty::State),
    Channel(channel::State),
    Server(server::State),
    Users(users::State),
}

#[derive(Debug, Clone)]
pub enum Message {
    Empty(empty::Message),
    Channel(channel::Message),
    Server(server::Message),
    Users(users::Message),
}

impl Buffer {
    pub fn update(&mut self, message: Message, clients: &mut data::client::Map) {
        match (self, message) {
            (Buffer::Channel(state), Message::Channel(message)) => state.update(message, clients),
            (Buffer::Users(state), Message::Users(message)) => state.update(message),
            (Buffer::Empty(state), Message::Empty(message)) => state.update(message),
            _ => {}
        }
    }

    pub fn view<'a>(
        &'a self,
        clients: &data::client::Map,
        is_focused: bool,
        theme: &'a Theme,
    ) -> Element<'a, Message> {
        match self {
            Buffer::Empty(state) => empty::view(state, theme).map(Message::Empty),
            Buffer::Channel(state) => {
                channel::view(state, clients, is_focused, theme).map(Message::Channel)
            }
            Buffer::Server(state) => {
                server::view(state, clients, is_focused, theme).map(Message::Server)
            }
            Buffer::Users(state) => users::view(state, theme).map(Message::Users),
        }
    }
}
