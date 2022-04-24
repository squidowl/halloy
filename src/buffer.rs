use data::{message::Channel, server::Server};
use iced::pure::Element;

use crate::theme::Theme;

pub mod channel;
pub mod empty;

#[derive(Clone)]
pub enum Buffer {
    Empty,
    Channel(Server, Channel),
}

#[derive(Debug, Clone)]
pub enum Message {}

impl Buffer {
    pub fn _update(&mut self, _message: Message) {}

    pub fn view<'a>(
        &'a self,
        clients: &data::client::Map,
        theme: &'a Theme,
    ) -> Element<'a, Message> {
        match self {
            Buffer::Empty => empty::view(theme),
            Buffer::Channel(server, channel) => channel::view(server, channel, clients, theme),
        }
    }
}
