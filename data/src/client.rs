use std::collections::HashMap;

use crate::message::Message;

#[derive(Debug, Clone)]
pub struct Client {
    server: String,
    config: irc::client::data::Config,
}

#[derive(Debug, Clone)]
pub enum State {
    Disconnected,
    Ready(Ready),
}

#[derive(Debug, Clone)]
pub struct Ready {
    sender: Sender,
    messages: Vec<Message>,
}

#[derive(Debug, Clone)]
pub struct Sender(irc::client::Sender);

#[derive(Debug, Clone, Default)]
pub struct Map(HashMap<String, State>);

impl Map {
    pub fn disconnected(&mut self, server: String) {
        self.0.insert(server, State::Disconnected);
    }

    pub fn ready(&mut self, server: String, sender: Sender) {
        self.0.insert(
            server,
            State::Ready(Ready {
                sender,
                messages: vec![],
            }),
        );
    }

    pub fn add_message(&mut self, server: &str, message: Message) {
        if let Some(State::Ready(ready)) = self.0.get_mut(server) {
            ready.messages.push(message);
        }
    }
}
