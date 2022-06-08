use iced::pure::Element;

use data::theme::Theme;

pub mod channel;
pub mod empty;
pub mod server;

#[derive(Clone)]
pub enum Buffer {
    Empty(empty::State),
    Channel(channel::State),
    Server(server::State),
}

#[derive(Debug, Clone)]
pub enum Message {
    Empty(empty::Message),
    Channel(channel::Message),
    Server(server::Message),
}

impl Buffer {
    pub fn update(&mut self, message: Message, clients: &mut data::client::Map) {
        match (self, message) {
            (Buffer::Empty(state), Message::Empty(message)) => state.update(message),
            (Buffer::Channel(state), Message::Channel(message)) => state.update(message, clients),
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
            Buffer::Empty(state) => empty::view(state, clients, theme).map(Message::Empty),
            Buffer::Channel(state) => {
                channel::view(state, clients, is_focused, theme).map(Message::Channel)
            }
            Buffer::Server(state) => {
                server::view(state, clients, is_focused, theme).map(Message::Server)
            }
        }
    }
}
