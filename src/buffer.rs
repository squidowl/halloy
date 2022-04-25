use iced::pure::Element;

use crate::theme::Theme;

pub mod channel;
pub mod empty;

#[derive(Clone)]
pub enum Buffer {
    Empty,
    Channel(channel::State),
}

#[derive(Debug, Clone)]
pub enum Message {
    Channel(channel::Message),
}

impl Buffer {
    pub fn update(&mut self, message: Message, clients: &data::client::Map) {
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
        }
    }
}
