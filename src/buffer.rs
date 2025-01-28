pub use data::buffer::{Internal, Settings, Upstream};
use data::user::Nick;
use data::{buffer, file_transfer, history, message, preview, target, Config};
use iced::Task;

pub use self::channel::Channel;
pub use self::file_transfers::FileTransfers;
pub use self::highlights::Highlights;
pub use self::logs::Logs;
pub use self::query::Query;
pub use self::server::Server;
use crate::screen::dashboard::sidebar;
use crate::widget::Element;
use crate::Theme;

pub mod channel;
pub mod empty;
pub mod file_transfers;
pub mod highlights;
mod input_view;
pub mod logs;
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
    Logs(Logs),
    Highlights(Highlights),
}

#[derive(Debug, Clone)]
pub enum Message {
    Channel(channel::Message),
    Server(server::Message),
    Query(query::Message),
    FileTransfers(file_transfers::Message),
    Logs(logs::Message),
    Highlights(highlights::Message),
}

pub enum Event {
    UserContext(user_context::Event),
    OpenChannel(target::Channel),
    GoToMessage(data::Server, target::Channel, message::Hash),
    History(Task<history::manager::Message>),
    RequestOlderChatHistory,
    PreviewChanged,
    HidePreview(history::Kind, message::Hash, url::Url),
}

impl Buffer {
    pub fn empty() -> Self {
        Self::Empty
    }

    pub fn upstream(&self) -> Option<&buffer::Upstream> {
        match self {
            Buffer::Channel(state) => Some(&state.buffer),
            Buffer::Server(state) => Some(&state.buffer),
            Buffer::Query(state) => Some(&state.buffer),
            Buffer::Empty | Buffer::FileTransfers(_) | Buffer::Logs(_) | Buffer::Highlights(_) => {
                None
            }
        }
    }

    pub fn internal(&self) -> Option<buffer::Internal> {
        match self {
            Buffer::Empty | Buffer::Channel(_) | Buffer::Server(_) | Buffer::Query(_) => None,
            Buffer::FileTransfers(_) => Some(buffer::Internal::FileTransfers),
            Buffer::Logs(_) => Some(buffer::Internal::Logs),
            Buffer::Highlights(_) => Some(buffer::Internal::Highlights),
        }
    }

    pub fn data(&self) -> Option<data::Buffer> {
        match self {
            Buffer::Empty => None,
            Buffer::Channel(state) => Some(data::Buffer::Upstream(state.buffer.clone())),
            Buffer::Server(state) => Some(data::Buffer::Upstream(state.buffer.clone())),
            Buffer::Query(state) => Some(data::Buffer::Upstream(state.buffer.clone())),
            Buffer::FileTransfers(_) => {
                Some(data::Buffer::Internal(buffer::Internal::FileTransfers))
            }
            Buffer::Logs(_) => Some(data::Buffer::Internal(buffer::Internal::Logs)),
            Buffer::Highlights(_) => Some(data::Buffer::Internal(buffer::Internal::Highlights)),
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
                    channel::Event::OpenChannel(channel) => Event::OpenChannel(channel),
                    channel::Event::History(task) => Event::History(task),
                    channel::Event::RequestOlderChatHistory => Event::RequestOlderChatHistory,
                    channel::Event::PreviewChanged => Event::PreviewChanged,
                    channel::Event::HidePreview(kind, hash, url) => {
                        Event::HidePreview(kind, hash, url)
                    }
                });

                (command.map(Message::Channel), event)
            }
            (Buffer::Server(state), Message::Server(message)) => {
                let (command, event) = state.update(message, clients, history, config);

                let event = event.map(|event| match event {
                    server::Event::UserContext(event) => Event::UserContext(event),
                    server::Event::OpenChannel(channel) => Event::OpenChannel(channel),
                    server::Event::History(task) => Event::History(task),
                });

                (command.map(Message::Server), event)
            }
            (Buffer::Query(state), Message::Query(message)) => {
                let (command, event) = state.update(message, clients, history, config);

                let event = event.map(|event| match event {
                    query::Event::UserContext(event) => Event::UserContext(event),
                    query::Event::OpenChannel(channel) => Event::OpenChannel(channel),
                    query::Event::History(task) => Event::History(task),
                    query::Event::RequestOlderChatHistory => Event::RequestOlderChatHistory,
                    query::Event::PreviewChanged => Event::PreviewChanged,
                    query::Event::HidePreview(kind, hash, url) => {
                        Event::HidePreview(kind, hash, url)
                    }
                });

                (command.map(Message::Query), event)
            }
            (Buffer::FileTransfers(state), Message::FileTransfers(message)) => {
                let command = state.update(message, file_transfers, config);

                (command.map(Message::FileTransfers), None)
            }
            (Buffer::Logs(state), Message::Logs(message)) => {
                let (command, event) = state.update(message, history, clients, config);

                let event = event.map(|event| match event {
                    logs::Event::UserContext(event) => Event::UserContext(event),
                    logs::Event::OpenChannel(channel) => Event::OpenChannel(channel),
                    logs::Event::History(task) => Event::History(task),
                });

                (command.map(Message::Logs), event)
            }
            (Buffer::Highlights(state), Message::Highlights(message)) => {
                let (command, event) = state.update(message, history, clients, config);

                let event = event.map(|event| match event {
                    highlights::Event::UserContext(event) => Event::UserContext(event),
                    highlights::Event::OpenChannel(channel) => Event::OpenChannel(channel),
                    highlights::Event::GoToMessage(server, channel, message) => {
                        Event::GoToMessage(server, channel, message)
                    }
                    highlights::Event::History(task) => Event::History(task),
                });

                (command.map(Message::Highlights), event)
            }
            _ => (Task::none(), None),
        }
    }

    pub fn view<'a>(
        &'a self,
        clients: &'a data::client::Map,
        file_transfers: &'a file_transfer::Manager,
        history: &'a history::Manager,
        previews: &'a preview::Collection,
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
                previews,
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
                query::view(state, clients, history, previews, config, theme, is_focused)
                    .map(Message::Query)
            }
            Buffer::FileTransfers(state) => {
                file_transfers::view(state, file_transfers).map(Message::FileTransfers)
            }
            Buffer::Logs(state) => logs::view(state, history, config, theme).map(Message::Logs),
            Buffer::Highlights(state) => {
                highlights::view(state, clients, history, config, theme).map(Message::Highlights)
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
    pub fn get_channel(
        &self,
        server: &data::Server,
        channel: &target::Channel,
    ) -> Option<&Channel> {
        if let Buffer::Channel(state) = self {
            (&state.server == server && state.target == *channel).then_some(state)
        } else {
            None
        }
    }

    pub fn focus(&self) -> Task<Message> {
        match self {
            Buffer::Empty | Buffer::FileTransfers(_) | Buffer::Logs(_) | Buffer::Highlights(_) => {
                Task::none()
            }
            Buffer::Channel(channel) => channel.focus().map(Message::Channel),
            Buffer::Server(server) => server.focus().map(Message::Server),
            Buffer::Query(query) => query.focus().map(Message::Query),
        }
    }

    pub fn reset(&mut self) {
        match self {
            Buffer::Empty | Buffer::FileTransfers(_) | Buffer::Logs(_) | Buffer::Highlights(_) => {}
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
        match self {
            Buffer::Empty
            | Buffer::Server(_)
            | Buffer::FileTransfers(_)
            | Buffer::Logs(_)
            | Buffer::Highlights(_) => Task::none(),
            Buffer::Channel(state) => state
                .input_view
                .insert_user(nick, state.buffer.clone(), history)
                .map(|message| Message::Channel(channel::Message::InputView(message))),
            Buffer::Query(state) => state
                .input_view
                .insert_user(nick, state.buffer.clone(), history)
                .map(|message| Message::Query(query::Message::InputView(message))),
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
            Buffer::Logs(log) => log
                .scroll_view
                .scroll_to_start()
                .map(|message| Message::Logs(logs::Message::ScrollView(message))),
            Buffer::Highlights(highlights) => highlights
                .scroll_view
                .scroll_to_start()
                .map(|message| Message::Highlights(highlights::Message::ScrollView(message))),
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
            Buffer::Logs(log) => log
                .scroll_view
                .scroll_to_end()
                .map(|message| Message::Logs(logs::Message::ScrollView(message))),
            Buffer::Highlights(highlights) => highlights
                .scroll_view
                .scroll_to_end()
                .map(|message| Message::Highlights(highlights::Message::ScrollView(message))),
        }
    }

    pub fn scroll_to_message(
        &mut self,
        message: message::Hash,
        history: &history::Manager,
        config: &Config,
    ) -> Task<Message> {
        match self {
            Buffer::Empty | Buffer::FileTransfers(_) => Task::none(),
            Buffer::Channel(state) => state
                .scroll_view
                .scroll_to_message(
                    message,
                    scroll_view::Kind::Channel(&state.server, &state.target),
                    history,
                    config,
                )
                .map(|message| Message::Channel(channel::Message::ScrollView(message))),
            Buffer::Server(state) => state
                .scroll_view
                .scroll_to_message(
                    message,
                    scroll_view::Kind::Server(&state.server),
                    history,
                    config,
                )
                .map(|message| Message::Server(server::Message::ScrollView(message))),
            Buffer::Query(state) => state
                .scroll_view
                .scroll_to_message(
                    message,
                    scroll_view::Kind::Query(&state.server, &state.target),
                    history,
                    config,
                )
                .map(|message| Message::Query(query::Message::ScrollView(message))),
            Buffer::Logs(state) => state
                .scroll_view
                .scroll_to_message(message, scroll_view::Kind::Logs, history, config)
                .map(|message| Message::Logs(logs::Message::ScrollView(message))),
            Buffer::Highlights(state) => state
                .scroll_view
                .scroll_to_message(message, scroll_view::Kind::Highlights, history, config)
                .map(|message| Message::Highlights(highlights::Message::ScrollView(message))),
        }
    }

    pub fn scroll_to_backlog(
        &mut self,
        history: &history::Manager,
        config: &Config,
    ) -> Task<Message> {
        match self {
            Buffer::Empty | Buffer::FileTransfers(_) => Task::none(),
            Buffer::Channel(state) => state
                .scroll_view
                .scroll_to_backlog(
                    scroll_view::Kind::Channel(&state.server, &state.target),
                    history,
                    config,
                )
                .map(|message| Message::Channel(channel::Message::ScrollView(message))),
            Buffer::Server(state) => state
                .scroll_view
                .scroll_to_backlog(scroll_view::Kind::Server(&state.server), history, config)
                .map(|message| Message::Server(server::Message::ScrollView(message))),
            Buffer::Query(state) => state
                .scroll_view
                .scroll_to_backlog(
                    scroll_view::Kind::Query(&state.server, &state.target),
                    history,
                    config,
                )
                .map(|message| Message::Query(query::Message::ScrollView(message))),
            Buffer::Logs(state) => state
                .scroll_view
                .scroll_to_backlog(scroll_view::Kind::Logs, history, config)
                .map(|message| Message::Logs(logs::Message::ScrollView(message))),
            Buffer::Highlights(state) => state
                .scroll_view
                .scroll_to_backlog(scroll_view::Kind::Highlights, history, config)
                .map(|message| Message::Highlights(highlights::Message::ScrollView(message))),
        }
    }

    pub fn is_scrolled_to_bottom(&self) -> Option<bool> {
        match self {
            Buffer::Empty | Buffer::FileTransfers(_) => None,
            Buffer::Channel(channel) => Some(channel.scroll_view.is_scrolled_to_bottom()),
            Buffer::Server(server) => Some(server.scroll_view.is_scrolled_to_bottom()),
            Buffer::Query(query) => Some(query.scroll_view.is_scrolled_to_bottom()),
            Buffer::Logs(log) => Some(log.scroll_view.is_scrolled_to_bottom()),
            Buffer::Highlights(highlights) => Some(highlights.scroll_view.is_scrolled_to_bottom()),
        }
    }
}

impl From<data::Buffer> for Buffer {
    fn from(buffer: data::Buffer) -> Self {
        match buffer {
            data::Buffer::Upstream(upstream) => match upstream {
                buffer::Upstream::Server(server) => Self::Server(Server::new(server)),
                buffer::Upstream::Channel(server, channel) => {
                    Self::Channel(Channel::new(server, channel))
                }
                buffer::Upstream::Query(server, query) => Self::Query(Query::new(server, query)),
            },
            data::Buffer::Internal(internal) => match internal {
                buffer::Internal::FileTransfers => Self::FileTransfers(FileTransfers::new()),
                buffer::Internal::Logs => Self::Logs(Logs::new()),
                buffer::Internal::Highlights => Self::Highlights(Highlights::new()),
            },
        }
    }
}
