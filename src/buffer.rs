pub use data::buffer::Settings;
use data::{buffer, history};
use iced::Command;

use self::channel::Channel;
use self::query::Query;
use self::server::Server;
use crate::widget::Element;

pub mod channel;
pub mod empty;
mod input_view;
pub mod query;
mod scroll_view;
pub mod server;
pub mod user_context;

#[derive(Clone)]
pub enum Buffer {
    Empty,
    Channel(Channel),
    Server(Server),
    Query(Query),
}

#[derive(Debug, Clone)]
pub enum Message {
    Channel(channel::Message),
    Server(server::Message),
    Query(query::Message),
}

#[derive(Debug, Clone)]
pub enum Event {
    UserContext(user_context::Event),
}

impl Buffer {
    pub fn empty() -> Self {
        Self::Empty
    }

    pub fn data(&self) -> Option<data::Buffer> {
        match self {
            Buffer::Empty => None,
            Buffer::Channel(state) => Some(data::Buffer::Channel(
                state.server.clone(),
                state.channel.clone(),
            )),
            Buffer::Server(state) => Some(data::Buffer::Server(state.server.clone())),
            Buffer::Query(state) => Some(data::Buffer::Query(
                state.server.clone(),
                state.nick.clone(),
            )),
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        clients: &mut data::client::Map,
        history: &mut history::Manager,
    ) -> (Command<Message>, Option<Event>) {
        match (self, message) {
            (Buffer::Channel(state), Message::Channel(message)) => {
                let (command, event) = state.update(message, clients, history);

                let event = event.map(|event| match event {
                    channel::Event::UserContext(event) => Event::UserContext(event),
                });

                (command.map(Message::Channel), event)
            }
            (Buffer::Server(state), Message::Server(message)) => {
                let command = state.update(message, clients, history);

                (command.map(Message::Server), None)
            }
            (Buffer::Query(state), Message::Query(message)) => {
                let (command, event) = state.update(message, clients, history);

                let event = event.map(|event| match event {
                    query::Event::UserContext(event) => Event::UserContext(event),
                });

                (command.map(Message::Query), event)
            }
            _ => (Command::none(), None),
        }
    }

    pub fn view<'a>(
        &'a self,
        clients: &'a data::client::Map,
        history: &'a history::Manager,
        settings: &'a buffer::Settings,
        is_focused: bool,
    ) -> Element<'a, Message> {
        match self {
            Buffer::Empty => empty::view(),
            Buffer::Channel(state) => {
                let status = clients.status(&state.server);

                channel::view(state, status, clients, history, settings, is_focused)
                    .map(Message::Channel)
            }
            Buffer::Server(state) => {
                let status = clients.status(&state.server);

                server::view(state, status, history, settings, is_focused).map(Message::Server)
            }
            Buffer::Query(state) => {
                let status = clients.status(&state.server);

                query::view(state, status, history, settings, is_focused).map(Message::Query)
            }
        }
    }

    // TODO: Placeholder in case we need
    #[allow(unused)]
    pub fn get_server(&self, server: &data::Server) -> Option<&Server> {
        if let Buffer::Server(state) = self {
            (&state.server == server).then_some(state)
        } else {
            None
        }
    }

    // TODO: Placeholder in case we need
    #[allow(unused)]
    pub fn get_channel(&self, server: &data::Server, channel: &str) -> Option<&Channel> {
        if let Buffer::Channel(state) = self {
            (&state.server == server && state.channel.as_str() == channel).then_some(state)
        } else {
            None
        }
    }

    pub fn focus(&self) -> Command<Message> {
        match self {
            Buffer::Empty => Command::none(),
            Buffer::Channel(channel) => channel.focus().map(Message::Channel),
            Buffer::Server(server) => server.focus().map(Message::Server),
            Buffer::Query(query) => query.focus().map(Message::Query),
        }
    }

    pub fn scroll_to_start(&mut self) -> Command<Message> {
        match self {
            Buffer::Empty => Command::none(),
            Buffer::Channel(channel) => channel
                .scroll_view
                .scroll_to_start()
                .map(|message| Message::Channel(channel::Message::ScrollView(message))),
            Buffer::Server(server) => server
                .scroll_view
                .scroll_to_start()
                .map(|message| Message::Server(server::Message::ScrollView(message))),
            Buffer::Query(query) => query
                .scroll_view
                .scroll_to_start()
                .map(|message| Message::Query(query::Message::ScrollView(message))),
        }
    }

    pub fn scroll_to_end(&mut self) -> Command<Message> {
        match self {
            Buffer::Empty => Command::none(),
            Buffer::Channel(channel) => channel
                .scroll_view
                .scroll_to_end()
                .map(|message| Message::Channel(channel::Message::ScrollView(message))),
            Buffer::Server(server) => server
                .scroll_view
                .scroll_to_end()
                .map(|message| Message::Server(server::Message::ScrollView(message))),
            Buffer::Query(query) => query
                .scroll_view
                .scroll_to_end()
                .map(|message| Message::Query(query::Message::ScrollView(message))),
        }
    }
}

impl From<data::Buffer> for Buffer {
    fn from(buffer: data::Buffer) -> Self {
        match buffer {
            data::Buffer::Server(server) => Self::Server(Server::new(server)),
            data::Buffer::Channel(server, channel) => Self::Channel(Channel::new(server, channel)),
            data::Buffer::Query(server, user) => Self::Query(Query::new(server, user)),
        }
    }
}
