use std::collections::HashMap;

use crate::{
    message::{Channel, Message},
    server::Server,
};

#[derive(Debug, Clone)]
pub enum State {
    Disconnected,
    Ready(Client),
}

#[derive(Debug, Clone)]
pub struct Client {
    sender: Sender,
    messages: Vec<Message>,
}

#[derive(Debug, Clone)]
pub struct Sender(irc::client::Sender);

impl From<irc::client::Sender> for Sender {
    fn from(sender: irc::client::Sender) -> Self {
        Self(sender)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Map(HashMap<Server, State>);

impl Map {
    pub fn disconnected(&mut self, server: Server) {
        self.0.insert(server, State::Disconnected);
    }

    pub fn ready(&mut self, server: Server, sender: Sender) {
        self.0.insert(
            server,
            State::Ready(Client {
                sender,
                messages: vec![],
            }),
        );
    }

    fn client(&self, server: &Server) -> Option<&Client> {
        if let Some(State::Ready(client)) = self.0.get(server) {
            Some(client)
        } else {
            None
        }
    }

    pub fn add_message(&mut self, server: &Server, message: Message) {
        if let Some(State::Ready(client)) = self.0.get_mut(server) {
            client.messages.push(message);
        }
    }

    pub fn get_messages(&self, server: &Server, channel: &Channel) -> Vec<&Message> {
        self.client(server)
            .map(|client| {
                client
                    .messages
                    .iter()
                    .filter(|m| m.is_for_channel(channel))
                    .collect()
            })
            .unwrap_or_default()
    }
}
