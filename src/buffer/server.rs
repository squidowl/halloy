use std::fmt;

use iced::widget::{column, container, scrollable, text, vertical_space};
use iced::{Command, Length};

use crate::widget::{input, Collection, Column, Element};

#[derive(Debug, Clone)]
pub enum Message {
    Send(String),
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
    let text_input = is_focused.then(|| input(state.input_id.clone(), Message::Send));

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
    input_id: input::Id,
}

impl Server {
    pub fn new(server: data::server::Server) -> Self {
        Self {
            server,
            input_id: input::Id::unique(),
            scrollable: scrollable::Id::unique(),
        }
    }

    pub fn update(&mut self, message: Message, _clients: &data::client::Map) -> Option<Event> {
        match message {
            Message::Send(_message) => {
                // TODO: You can't send messages to a server,
                // however I would make sense to allow slash (`/`) commands.
                // Eg. /auth.

                None
            }
        }
    }

    pub fn focus(&self) -> Command<Message> {
        input::focus(self.input_id.clone())
    }
}

impl fmt::Display for Server {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.server)
    }
}
