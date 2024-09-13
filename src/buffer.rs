pub use data::buffer::Settings;
use data::user::Nick;
use data::{buffer, file_transfer, history, Config};
use iced::Task;

use self::channel::Channel;
use self::file_transfers::FileTransfers;
use self::query::Query;
use self::server::Server;
use crate::screen::dashboard::sidebar;
use crate::widget::Element;
use crate::Theme;

pub mod channel;
pub mod empty;
pub mod file_transfers;
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
    FileTransfers(FileTransfers),
}

#[derive(Debug, Clone)]
pub enum Message {
    Channel(channel::Message),
    Server(server::Message),
    Query(query::Message),
    FileTransfers(file_transfers::Message),
}

#[derive(Debug, Clone)]
pub enum Event {
    UserContext(user_context::Event),
    ScrolledToTop,
    ChatHistoryBeforeRequest,
}

impl Buffer {
    pub fn empty() -> Self {
        Self::Empty
    }

    pub fn data(&self) -> Option<data::Buffer> {
        match self {
            Buffer::Empty => None,
            Buffer::Channel(state) => Some(state.buffer()),
            Buffer::Server(state) => Some(state.buffer()),
            Buffer::Query(state) => Some(state.buffer()),
            Buffer::FileTransfers(_) => None,
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        clients: &mut data::client::Map,
        history: &mut history::Manager,
        file_transfers: &mut file_transfer::Manager,
        config: &Config,
    ) -> (Task<Message>, Option<Event>) {
        match (self, message) {
            (Buffer::Channel(state), Message::Channel(message)) => {
                let (command, event) = state.update(message, clients, history, config);

                let event = event.map(|event| match event {
                    channel::Event::UserContext(event) => Event::UserContext(event),
                    channel::Event::ScrolledToTop => Event::ScrolledToTop,
                    channel::Event::ChatHistoryBeforeRequest => Event::ChatHistoryBeforeRequest,
                });

                (command.map(Message::Channel), event)
            }
            (Buffer::Server(state), Message::Server(message)) => {
                let command = state.update(message, clients, history, config);

                (command.map(Message::Server), None)
            }
            (Buffer::Query(state), Message::Query(message)) => {
                let (command, event) = state.update(message, clients, history, config);

                let event = event.map(|event| match event {
                    query::Event::UserContext(event) => Event::UserContext(event),
                    query::Event::ScrolledToTop => Event::ScrolledToTop,
                    query::Event::ChatHistoryBeforeRequest => Event::ChatHistoryBeforeRequest,
                });

                (command.map(Message::Query), event)
            }
            (Buffer::FileTransfers(state), Message::FileTransfers(message)) => {
                let command = state.update(message, file_transfers, config);

                (command.map(Message::FileTransfers), None)
            }
            _ => (Task::none(), None),
        }
    }

    pub fn view<'a>(
        &'a self,
        clients: &'a data::client::Map,
        file_transfers: &'a file_transfer::Manager,
        history: &'a history::Manager,
        settings: &'a buffer::Settings,
        config: &'a Config,
        theme: &'a Theme,
        is_focused: bool,
        sidebar: &'a sidebar::Sidebar,
    ) -> Element<'a, Message> {
        match self {
            Buffer::Empty => empty::view(config, sidebar),
            Buffer::Channel(state) => channel::view(
                state,
                clients,
                history,
                &settings.channel,
                config,
                theme,
                is_focused,
            )
            .map(Message::Channel),
            Buffer::Server(state) => {
                server::view(state, clients, history, config, theme, is_focused)
                    .map(Message::Server)
            }
            Buffer::Query(state) => {
                query::view(state, clients, history, config, theme, is_focused).map(Message::Query)
            }
            Buffer::FileTransfers(state) => {
                file_transfers::view(state, file_transfers).map(Message::FileTransfers)
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

    pub fn focus(&self) -> Task<Message> {
        match self {
            Buffer::Empty | Buffer::FileTransfers(_) => Task::none(),
            Buffer::Channel(channel) => channel.focus().map(Message::Channel),
            Buffer::Server(server) => server.focus().map(Message::Server),
            Buffer::Query(query) => query.focus().map(Message::Query),
        }
    }

    pub fn reset(&mut self) {
        match self {
            Buffer::Empty | Buffer::FileTransfers(_) => {}
            Buffer::Channel(channel) => channel.reset(),
            Buffer::Server(server) => server.reset(),
            Buffer::Query(query) => query.reset(),
        }
    }

    pub fn insert_user_to_input(
        &mut self,
        nick: Nick,
        history: &mut history::Manager,
    ) -> Task<Message> {
        if let Some(buffer) = self.data() {
            match self {
                Buffer::Empty | Buffer::Server(_) | Buffer::FileTransfers(_) => Task::none(),
                Buffer::Channel(channel) => channel
                    .input_view
                    .insert_user(nick, buffer, history)
                    .map(|message| Message::Channel(channel::Message::InputView(message))),
                Buffer::Query(query) => query
                    .input_view
                    .insert_user(nick, buffer, history)
                    .map(|message| Message::Query(query::Message::InputView(message))),
            }
        } else {
            Task::none()
        }
    }

    pub fn scroll_to_start(&mut self) -> Task<Message> {
        match self {
            Buffer::Empty | Buffer::FileTransfers(_) => Task::none(),
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

    pub fn scroll_to_end(&mut self) -> Task<Message> {
        match self {
            Buffer::Empty | Buffer::FileTransfers(_) => Task::none(),
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
