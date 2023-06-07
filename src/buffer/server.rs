use std::fmt;

use crate::widget::Collection;
use crate::widget::{Column, Element};
use iced::Command;
use iced::{
    widget::{column, container, scrollable, text, text_input, vertical_space},
    Length,
};

#[derive(Debug, Clone)]
pub enum Message {
    Send,
    Input(String),
}

#[derive(Debug, Clone)]
pub enum Event {}

pub fn view<'a>(
    state: &'a Server,
    clients: &data::client::Map,
    is_focused: bool,
) -> Element<'a, Message> {
    let messages: Vec<Element<'a, Message>> = clients
        .get_server_messages(&state.server)
        .into_iter()
        .filter_map(|message| Some(container(text(message.text()?)).into()))
        .collect();

    let messages = container(
        scrollable(Column::with_children(messages).width(Length::Fill))
            .id(state.scrollable.clone()),
    )
    .height(Length::Fill);
    let spacing = is_focused.then_some(vertical_space(4));
    let text_input = is_focused.then_some(
        text_input("Send message...", &state.input)
            .on_input(Message::Input)
            .on_submit(Message::Send)
            .id(state.text_input.clone())
            .padding(8),
    );

    let scrollable = column![messages]
        .push_maybe(spacing)
        .push_maybe(text_input)
        .height(Length::Fill);

    container(scrollable)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(8)
        .into()
}

#[derive(Debug, Clone)]
pub struct Server {
    pub server: data::server::Server,
    pub scrollable: scrollable::Id,
    text_input: text_input::Id,
    input: String,
}

impl Server {
    pub fn new(server: data::server::Server) -> Self {
        Self {
            server,
            input: String::new(),
            text_input: text_input::Id::unique(),
            scrollable: scrollable::Id::unique(),
        }
    }

    pub fn update(&mut self, message: Message, _clients: &data::client::Map) -> Option<Event> {
        match message {
            Message::Send => {
                // TODO: You can't send messages to a server,
                // however I would make sense to allow slash (`/`) commands.
                // Eg. /auth.

                None
            }
            Message::Input(input) => {
                self.input = input;

                None
            }
        }
    }

    pub fn focus(&self) -> Command<Message> {
        text_input::focus(self.text_input.clone())
    }
}

impl fmt::Display for Server {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.server)
    }
}
