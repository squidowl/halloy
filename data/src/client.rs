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
