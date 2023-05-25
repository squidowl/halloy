use crate::widget::Element;

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

#[derive(Debug, Clone)]
pub enum Event {
    Empty(empty::Event),
    Channel(channel::Event),
}

impl Buffer {
    pub fn update(&mut self, message: Message, clients: &mut data::client::Map) -> Option<Event> {
        match (self, message) {
            (Buffer::Empty(state), Message::Empty(message)) => {
                state.update(message).map(Event::Empty)
            }
            (Buffer::Channel(state), Message::Channel(message)) => {
                state.update(message, clients).map(Event::Channel)
            }
            _ => None,
        }
    }

    pub fn view<'a>(
        &'a self,
        clients: &data::client::Map,
        is_focused: bool,
    ) -> Element<'a, Message> {
        match self {
            Buffer::Empty(state) => empty::view(state, clients).map(Message::Empty),
            Buffer::Channel(state) => {
                channel::view(state, clients, is_focused).map(Message::Channel)
            }
            Buffer::Server(state) => server::view(state, clients, is_focused).map(Message::Server),
        }
    }
}
