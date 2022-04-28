use std::fmt;

use data::{message::Channel, server::Server};
use iced::{
    pure::{
        self, container, scrollable, text, text_input, vertical_space, widget::Column, Element,
    },
    Length, Space,
};

use crate::{style, theme::Theme};

#[derive(Debug, Clone)]
pub enum Message {}

pub fn view<'a>(
    state: &State,
    clients: &data::client::Map,
    is_focused: bool,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let components: Vec<Element<'a, Message>> = clients
        .get_messages_for_server(&state.server)
        .into_iter()
        .filter_map(|message| match message.command() {
            data::message::Command::Response { response, text } => {
                if let Some(value) = response.parse(text) {
                    Some(container(pure::text(value).size(style::TEXT_SIZE)).into())
                } else {
                    None
                }
            }
            _ => None,
        })
        .collect();

    let content = Column::with_children(components).push(vertical_space(Length::Fill));

    scrollable(content).height(Length::Fill).into()
}

#[derive(Debug, Clone)]
pub struct State {
    server: Server,
}

impl State {
    pub fn new(server: Server) -> Self {
        Self { server }
    }

    pub fn _update(&mut self, message: Message, clients: &data::client::Map) {}
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let server: String = self.server.clone().into();

        write!(f, "{}", server)
    }
}
