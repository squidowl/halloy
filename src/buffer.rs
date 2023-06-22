use data::history;
use iced::Command;

use self::channel::Channel;
use self::empty::Empty;
use self::query::Query;
use self::server::Server;
use crate::widget::Element;

pub mod channel;
pub mod empty;
mod input_view;
pub mod query;
mod scroll_view;
pub mod server;

#[derive(Clone)]
pub enum Buffer {
    Empty(Empty),
    Channel(Channel),
    Server(Server),
    Query(Query),
}

#[derive(Debug, Clone)]
pub enum Message {
    Empty(empty::Message),
    Channel(channel::Message),
    Server(server::Message),
    Query(query::Message),
}

#[derive(Debug, Clone)]
pub enum Event {
    Empty(empty::Event),
    Channel(channel::Event),
    Server(server::Event),
    Query(query::Event),
}

impl Buffer {
    pub fn kind(&self) -> Option<data::Buffer> {
        match self {
            Buffer::Empty(_) => None,
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
            (Buffer::Empty(state), Message::Empty(message)) => {
                (Command::none(), state.update(message).map(Event::Empty))
            }
            (Buffer::Channel(state), Message::Channel(message)) => {
                let (command, event) = state.update(message, clients, history);

                (command.map(Message::Channel), event.map(Event::Channel))
            }
            (Buffer::Server(state), Message::Server(message)) => {
                let (command, event) = state.update(message, clients, history);

                (command.map(Message::Server), event.map(Event::Server))
            }
            (Buffer::Query(state), Message::Query(message)) => {
                let (command, event) = state.update(message, clients, history);

                (command.map(Message::Query), event.map(Event::Query))
            }
            _ => (Command::none(), None),
        }
    }

    pub fn view<'a>(
        &'a self,
        clients: &'a data::client::Map,
        history: &'a history::Manager,
        config: &'a data::config::Config,
        is_focused: bool,
    ) -> Element<'a, Message> {
        let buffer_config = &config.buffer;

        match self {
            Buffer::Empty(state) => empty::view(state, clients, config).map(Message::Empty),
            Buffer::Channel(state) => {
                let channel_config = config.channel_config(&state.server.name, &state.channel);

                channel::view(
                    state,
                    clients,
                    history,
                    &channel_config,
                    buffer_config,
                    is_focused,
                )
                .map(Message::Channel)
            }
            Buffer::Server(state) => {
                server::view(state, history, buffer_config, is_focused).map(Message::Server)
            }
            Buffer::Query(state) => {
                query::view(state, history, buffer_config, is_focused).map(Message::Query)
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
            Buffer::Empty(_) => Command::none(),
            Buffer::Channel(channel) => channel.focus().map(Message::Channel),
            Buffer::Server(server) => server.focus().map(Message::Server),
            Buffer::Query(query) => query.focus().map(Message::Query),
        }
    }

    pub fn scroll_to_start(&mut self) -> Command<Message> {
        match self {
            Buffer::Empty(_) => Command::none(),
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
            Buffer::Empty(_) => Command::none(),
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
    fn from(kind: data::Buffer) -> Self {
        match kind {
            data::Buffer::Server(server) => Self::Server(Server::new(server)),
            data::Buffer::Channel(server, channel) => Self::Channel(Channel::new(server, channel)),
            data::Buffer::Query(server, user) => Self::Query(Query::new(server, user)),
        }
    }
}
