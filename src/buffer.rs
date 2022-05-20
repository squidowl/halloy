use iced::pure::Element;

use crate::theme::Theme;

pub mod channel;
pub mod empty;
pub mod server;

#[derive(Clone)]
pub enum Buffer {
    Empty,
    Channel(channel::State),
    Server,
}

#[derive(Debug, Clone)]
pub enum Message {
    Channel(channel::Message),
    Server(server::Message),
}

impl Buffer {
    pub fn update(&mut self, message: Message, clients: &mut data::client::Map) {
        match (self, message) {
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
            Buffer::Empty => empty::view(theme),
            Buffer::Channel(state) => {
                channel::view(state, clients, is_focused, theme).map(Message::Channel)
            }
            Buffer::Server => server::view(clients, is_focused, theme).map(Message::Server),
        }
    }
}
