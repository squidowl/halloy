use data::{message::Channel, server::Server};
use iced::{
    pure::{container, text, text_input, widget::Column, Element},
    Length, Space,
};

use crate::theme::Theme;

#[derive(Debug, Clone)]
pub enum Message {
    Send,
    Input(String),
}

pub fn view<'a>(
    state: &State,
    clients: &data::client::Map,
    _theme: &'a Theme,
) -> Element<'a, Message> {
    let messages = clients
        .get_messages(&state.server, &state.channel)
        .into_iter()
        .map(|message| text(format!("{:?}", message)).into())
        .collect();

    let content = Column::with_children(messages)
        .push(Space::with_height(Length::Fill))
        .push(text_input("", &state.input, Message::Input).on_submit(Message::Send));

    // TODO: Scrollable with chat messages.

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

#[derive(Debug, Clone)]
pub struct State {
    server: Server,
    channel: Channel,
    input: String,
}

impl State {
    pub fn new(server: Server, channel: Channel) -> Self {
        Self {
            server,
            channel,
            input: String::new(),
        }
    }

    pub fn update(&mut self, message: Message, clients: &data::client::Map) {
        match message {
            Message::Send => {
                clients.send_message(&self.server, &self.channel, &self.input);
                self.input = String::new();
            }
            Message::Input(input) => self.input = input,
        }
    }
}
