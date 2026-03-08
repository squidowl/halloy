use std::path::PathBuf;

use chrono::{DateTime, Utc};
pub use data::buffer::{Internal, Settings, Upstream};
use data::config::buffer::text_input::Autocomplete;
use data::dashboard::BufferAction;
use data::target::{self, Target};
use data::user::Nick;
use data::{Config, buffer, file_transfer, history, message, preview};
use iced::{Size, Task};

pub use self::channel::Channel;
pub use self::channel_discovery::ChannelDiscovery;
pub use self::file_transfers::FileTransfers;
pub use self::highlights::Highlights;
pub use self::logs::Logs;
pub use self::query::Query;
pub use self::scripts::Scripts;
pub use self::server::Server;
use crate::Theme;
use crate::screen::dashboard::sidebar;
use crate::widget::Element;
use crate::window::Window;

pub mod channel;
pub mod channel_discovery;
pub mod context_menu;
pub mod empty;
pub mod file_transfers;
pub mod highlights;
mod input_view;
pub mod logs;
mod message_view;
pub mod query;
pub mod scripts;
mod scroll_view;
pub mod server;

#[derive(Clone, Debug)]
pub enum Buffer {
    Empty,
    Channel(Channel),
    Server(Server),
    Query(Query),
    FileTransfers(FileTransfers),
    Scripts(Scripts),
    Logs(Logs),
    Highlights(Highlights),
    ChannelDiscovery(ChannelDiscovery),
}

#[derive(Debug, Clone)]
pub enum Message {
    Channel(channel::Message),
    Server(server::Message),
    Query(query::Message),
    FileTransfers(file_transfers::Message),
    Scripts(scripts::Message),
    Logs(logs::Message),
    Highlights(highlights::Message),
    ChannelList(channel_discovery::Message),
}

pub enum Event {
    ContextMenu(context_menu::Event),
    OpenBuffers(data::Server, Vec<(Target, BufferAction)>),
    OpenInternalBuffer(buffer::Internal),
    ToggleScript(String),
    OpenServer(String),
    Reconnect(data::Server),
    LeaveBuffers(Vec<Target>, Option<String>),
    SelectedServer(data::Server),
    GoToMessage(data::Server, target::Channel, message::Hash),
    History(Task<history::manager::Message>),
    RequestOlderChatHistory,
    PreviewChanged,
    HidePreview(history::Kind, message::Hash, url::Url),
    MarkAsRead(history::Kind),
    OpenUrl(String),
    ImagePreview(PathBuf, url::Url),
    ExpandCondensedMessage(DateTime<Utc>, message::Hash),
    ContractCondensedMessage(DateTime<Utc>, message::Hash),
    InputSent {
        history_task: Task<history::manager::Message>,
        open_buffers: Vec<(Target, BufferAction)>,
    },
    SendUnsafeList(data::Server),
}

impl Buffer {
    pub fn from_data(
        buffer: data::Buffer,
        history: &history::Manager,
        pane_size: Size,
        config: &Config,
    ) -> Self {
        match buffer {
            data::Buffer::Upstream(upstream) => match upstream {
                buffer::Upstream::Server(server) => {
                    Self::Server(Server::new(server, pane_size, config))
                }
                buffer::Upstream::Channel(server, channel) => Self::Channel(
                    Channel::new(server, channel, history, pane_size, config),
                ),
                buffer::Upstream::Query(server, query) => Self::Query(
                    Query::new(server, query, history, pane_size, config),
                ),
            },
            data::Buffer::Internal(internal) => match internal {
                buffer::Internal::FileTransfers => {
                    Self::FileTransfers(FileTransfers::new())
                }
                buffer::Internal::Scripts => Self::Scripts(Scripts::new()),
                buffer::Internal::Logs => {
                    Self::Logs(Logs::new(pane_size, config))
                }
                buffer::Internal::Highlights => {
                    Self::Highlights(Highlights::new(pane_size, config))
                }
                buffer::Internal::ChannelDiscovery(server) => {
                    Self::ChannelDiscovery(ChannelDiscovery::new(server))
                }
            },
        }
    }
    pub fn empty() -> Self {
        Self::Empty
    }

    pub fn upstream(&self) -> Option<&buffer::Upstream> {
        match self {
            Buffer::Channel(state) => Some(&state.buffer),
            Buffer::Server(state) => Some(&state.buffer),
            Buffer::Query(state) => Some(&state.buffer),
            Buffer::Empty
            | Buffer::FileTransfers(_)
            | Buffer::Scripts(_)
            | Buffer::Logs(_)
            | Buffer::Highlights(_)
            | Buffer::ChannelDiscovery(_) => None,
        }
    }

    pub fn internal(&self) -> Option<buffer::Internal> {
        match self {
            Buffer::Empty
            | Buffer::Channel(_)
            | Buffer::Server(_)
            | Buffer::Query(_) => None,
            Buffer::FileTransfers(_) => Some(buffer::Internal::FileTransfers),
            Buffer::Scripts(_) => Some(buffer::Internal::Scripts),
            Buffer::Logs(_) => Some(buffer::Internal::Logs),
            Buffer::Highlights(_) => Some(buffer::Internal::Highlights),
            Buffer::ChannelDiscovery(state) => {
                Some(buffer::Internal::ChannelDiscovery(state.server.clone()))
            }
        }
    }

    pub fn data(&self) -> Option<data::Buffer> {
        match self {
            Buffer::Empty => None,
            Buffer::Channel(state) => {
                Some(data::Buffer::Upstream(state.buffer.clone()))
            }
            Buffer::Server(state) => {
                Some(data::Buffer::Upstream(state.buffer.clone()))
            }
            Buffer::Query(state) => {
                Some(data::Buffer::Upstream(state.buffer.clone()))
            }
            Buffer::FileTransfers(_) => {
                Some(data::Buffer::Internal(buffer::Internal::FileTransfers))
            }
            Buffer::Scripts(_) => {
                Some(data::Buffer::Internal(buffer::Internal::Scripts))
            }
            Buffer::Logs(_) => {
                Some(data::Buffer::Internal(buffer::Internal::Logs))
            }
            Buffer::Highlights(_) => {
                Some(data::Buffer::Internal(buffer::Internal::Highlights))
            }
            Buffer::ChannelDiscovery(state) => Some(data::Buffer::Internal(
                buffer::Internal::ChannelDiscovery(state.server.clone()),
            )),
        }
    }

    pub fn server(&self) -> Option<data::Server> {
        match self {
            Buffer::Channel(state) => Some(state.server.clone()),
            Buffer::Query(state) => Some(state.server.clone()),
            Buffer::Server(state) => Some(state.server.clone()),
            Buffer::Empty
            | Buffer::FileTransfers(_)
            | Buffer::Scripts(_)
            | Buffer::Logs(_)
            | Buffer::Highlights(_)
            | Buffer::ChannelDiscovery(_) => None,
        }
    }

    pub fn target(&self) -> Option<Target> {
        match self {
            Buffer::Channel(state) => {
                Some(Target::Channel(state.target.clone()))
            }
            Buffer::Query(state) => Some(Target::Query(state.target.clone())),
            Buffer::Empty
            | Buffer::Server(_)
            | Buffer::FileTransfers(_)
            | Buffer::Scripts(_)
            | Buffer::Logs(_)
            | Buffer::Highlights(_)
            | Buffer::ChannelDiscovery(_) => None,
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        clients: &mut data::client::Map,
        history: &mut history::Manager,
        file_transfers: &mut file_transfer::Manager,
        main_window: &Window,
        config: &Config,
    ) -> (Task<Message>, Option<Event>) {
        match (self, message) {
            (Buffer::Channel(state), Message::Channel(message)) => {
                let (command, event) = state.update(
                    message,
                    clients,
                    history,
                    main_window,
                    config,
                );

                let event = event.map(|event| match event {
                    channel::Event::ContextMenu(event) => {
                        Event::ContextMenu(event)
                    }
                    channel::Event::OpenBuffers(server, targets) => {
                        Event::OpenBuffers(server, targets)
                    }
                    channel::Event::OpenInternalBuffer(buffer) => {
                        Event::OpenInternalBuffer(buffer)
                    }
                    channel::Event::OpenServer(server) => {
                        Event::OpenServer(server)
                    }
                    channel::Event::Reconnect(server) => {
                        Event::Reconnect(server)
                    }
                    channel::Event::LeaveBuffers(targets, reason) => {
                        Event::LeaveBuffers(targets, reason)
                    }
                    channel::Event::History(task) => Event::History(task),
                    channel::Event::RequestOlderChatHistory => {
                        Event::RequestOlderChatHistory
                    }
                    channel::Event::PreviewChanged => Event::PreviewChanged,
                    channel::Event::HidePreview(kind, hash, url) => {
                        Event::HidePreview(kind, hash, url)
                    }
                    channel::Event::MarkAsRead(kind) => Event::MarkAsRead(kind),
                    channel::Event::OpenUrl(url) => Event::OpenUrl(url),
                    channel::Event::ImagePreview(path, url) => {
                        Event::ImagePreview(path, url)
                    }
                    channel::Event::ExpandCondensedMessage(
                        server_time,
                        hash,
                    ) => Event::ExpandCondensedMessage(server_time, hash),
                    channel::Event::ContractCondensedMessage(
                        server_time,
                        hash,
                    ) => Event::ContractCondensedMessage(server_time, hash),
                    channel::Event::InputSent {
                        history_task,
                        open_buffers,
                    } => Event::InputSent {
                        history_task,
                        open_buffers,
                    },
                });

                (command.map(Message::Channel), event)
            }
            (Buffer::Server(state), Message::Server(message)) => {
                let (command, event) = state.update(
                    message,
                    clients,
                    history,
                    main_window,
                    config,
                );

                let event = event.map(|event| match event {
                    server::Event::ContextMenu(event) => {
                        Event::ContextMenu(event)
                    }
                    server::Event::OpenInternalBuffer(buffer) => {
                        Event::OpenInternalBuffer(buffer)
                    }
                    server::Event::OpenServer(server) => {
                        Event::OpenServer(server)
                    }
                    server::Event::Reconnect(server) => {
                        Event::Reconnect(server)
                    }
                    server::Event::OpenBuffers(server, targets) => {
                        Event::OpenBuffers(server, targets)
                    }
                    server::Event::LeaveBuffers(targets, reason) => {
                        Event::LeaveBuffers(targets, reason)
                    }
                    server::Event::History(task) => Event::History(task),
                    server::Event::MarkAsRead(kind) => Event::MarkAsRead(kind),
                    server::Event::OpenUrl(url) => Event::OpenUrl(url),
                    server::Event::ImagePreview(path, url) => {
                        Event::ImagePreview(path, url)
                    }
                    server::Event::ExpandCondensedMessage(
                        server_time,
                        hash,
                    ) => Event::ExpandCondensedMessage(server_time, hash),
                    server::Event::ContractCondensedMessage(
                        server_time,
                        hash,
                    ) => Event::ContractCondensedMessage(server_time, hash),
                    server::Event::InputSent {
                        history_task,
                        open_buffers,
                    } => Event::InputSent {
                        history_task,
                        open_buffers,
                    },
                });

                (command.map(Message::Server), event)
            }
            (Buffer::Query(state), Message::Query(message)) => {
                let (command, event) = state.update(
                    message,
                    clients,
                    history,
                    main_window,
                    config,
                );

                let event = event.map(|event| match event {
                    query::Event::ContextMenu(event) => {
                        Event::ContextMenu(event)
                    }
                    query::Event::OpenBuffers(server, targets) => {
                        Event::OpenBuffers(server, targets)
                    }
                    query::Event::OpenInternalBuffer(buffer) => {
                        Event::OpenInternalBuffer(buffer)
                    }
                    query::Event::OpenServer(server) => {
                        Event::OpenServer(server)
                    }
                    query::Event::Reconnect(server) => Event::Reconnect(server),
                    query::Event::LeaveBuffers(targets, reason) => {
                        Event::LeaveBuffers(targets, reason)
                    }
                    query::Event::History(task) => Event::History(task),
                    query::Event::RequestOlderChatHistory => {
                        Event::RequestOlderChatHistory
                    }
                    query::Event::PreviewChanged => Event::PreviewChanged,
                    query::Event::HidePreview(kind, hash, url) => {
                        Event::HidePreview(kind, hash, url)
                    }
                    query::Event::MarkAsRead(kind) => Event::MarkAsRead(kind),
                    query::Event::OpenUrl(url) => Event::OpenUrl(url),
                    query::Event::ImagePreview(path, url) => {
                        Event::ImagePreview(path, url)
                    }
                    query::Event::ExpandCondensedMessage(server_time, hash) => {
                        Event::ExpandCondensedMessage(server_time, hash)
                    }
                    query::Event::ContractCondensedMessage(
                        server_time,
                        hash,
                    ) => Event::ContractCondensedMessage(server_time, hash),
                    query::Event::InputSent {
                        history_task,
                        open_buffers,
                    } => Event::InputSent {
                        history_task,
                        open_buffers,
                    },
                });

                (command.map(Message::Query), event)
            }
            (Buffer::FileTransfers(state), Message::FileTransfers(message)) => {
                let command = state.update(message, file_transfers, config);

                (command.map(Message::FileTransfers), None)
            }
            (Buffer::Scripts(state), Message::Scripts(message)) => {
                let (command, event) = state.update(message);
                let event = event.map(|event| match event {
                    scripts::Event::Toggle(name) => Event::ToggleScript(name),
                });

                (command.map(Message::Scripts), event)
            }
            (
                Buffer::ChannelDiscovery(state),
                Message::ChannelList(message),
            ) => {
                let (command, event) = state.update(message, config);

                let event = event.map(|event| match event {
                    channel_discovery::Event::SelectedServer(server) => {
                        Event::SelectedServer(server)
                    }
                    channel_discovery::Event::SendUnsafeList(server) => {
                        Event::SendUnsafeList(server)
                    }
                    channel_discovery::Event::OpenUrl(url) => {
                        Event::OpenUrl(url)
                    }
                    channel_discovery::Event::OpenChannelForServer(
                        server,
                        channel,
                    ) => Event::OpenBuffers(
                        server,
                        vec![(
                            Target::Channel(channel),
                            config.actions.buffer.click_channel_name,
                        )],
                    ),
                    channel_discovery::Event::ContextMenu(event) => {
                        Event::ContextMenu(event)
                    }
                });

                (command.map(Message::ChannelList), event)
            }
            (Buffer::Logs(state), Message::Logs(message)) => {
                let (command, event) =
                    state.update(message, history, clients, config);

                let event = event.map(|event| match event {
                    logs::Event::ContextMenu(event) => {
                        Event::ContextMenu(event)
                    }
                    logs::Event::History(task) => Event::History(task),
                    logs::Event::MarkAsRead => {
                        Event::MarkAsRead(history::Kind::Logs)
                    }
                    logs::Event::OpenUrl(url) => Event::OpenUrl(url),
                    logs::Event::ImagePreview(path, url) => {
                        Event::ImagePreview(path, url)
                    }
                    logs::Event::ExpandCondensedMessage(server_time, hash) => {
                        Event::ExpandCondensedMessage(server_time, hash)
                    }
                    logs::Event::ContractCondensedMessage(
                        server_time,
                        hash,
                    ) => Event::ContractCondensedMessage(server_time, hash),
                });

                (command.map(Message::Logs), event)
            }
            (Buffer::Highlights(state), Message::Highlights(message)) => {
                let (command, event) =
                    state.update(message, history, clients, config);

                let event = event.map(|event| match event {
                    highlights::Event::ContextMenu(event) => {
                        Event::ContextMenu(event)
                    }
                    highlights::Event::OpenBuffer(
                        server,
                        target,
                        buffer_action,
                    ) => Event::OpenBuffers(
                        server,
                        vec![(target, buffer_action)],
                    ),
                    highlights::Event::GoToMessage(
                        server,
                        channel,
                        message,
                    ) => Event::GoToMessage(server, channel, message),
                    highlights::Event::History(task) => Event::History(task),
                    highlights::Event::OpenUrl(url) => Event::OpenUrl(url),
                    highlights::Event::ImagePreview(path, url) => {
                        Event::ImagePreview(path, url)
                    }
                    highlights::Event::ExpandCondensedMessage(
                        server_time,
                        hash,
                    ) => Event::ExpandCondensedMessage(server_time, hash),
                    highlights::Event::ContractCondensedMessage(
                        server_time,
                        hash,
                    ) => Event::ContractCondensedMessage(server_time, hash),
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
        script_manager: &'a data::scripts::Manager,
        history: &'a history::Manager,
        previews: &'a preview::Collection,
        settings: Option<&'a buffer::Settings>,
        config: &'a Config,
        theme: &'a Theme,
        is_focused: bool,
        sidebar: &'a sidebar::Sidebar,
    ) -> Element<'a, Message> {
        match self {
            Buffer::Empty => empty::view(config, sidebar),
            Buffer::Channel(state) => channel::view(
                state, clients, history, previews, settings, config, theme,
                is_focused,
            )
            .map(Message::Channel),
            Buffer::Server(state) => {
                server::view(state, clients, history, config, theme, is_focused)
                    .map(Message::Server)
            }
            Buffer::Query(state) => query::view(
                state, clients, history, previews, config, theme, is_focused,
            )
            .map(Message::Query),
            Buffer::FileTransfers(state) => {
                file_transfers::view(state, file_transfers, theme)
                    .map(Message::FileTransfers)
            }
            Buffer::Scripts(_) => {
                scripts::view(script_manager, &config.scripts.autorun, theme)
                    .map(Message::Scripts)
            }
            Buffer::Logs(state) => {
                logs::view(state, history, config, theme).map(Message::Logs)
            }
            Buffer::Highlights(state) => {
                highlights::view(state, clients, history, config, theme)
                    .map(Message::Highlights)
            }
            Buffer::ChannelDiscovery(state) => {
                channel_discovery::view(state, clients, config, theme)
                    .map(Message::ChannelList)
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
            (&state.server == server && state.target == *channel)
                .then_some(state)
        } else {
            None
        }
    }

    pub fn focus(&self) -> Task<Message> {
        match self {
            Buffer::Empty
            | Buffer::FileTransfers(_)
            | Buffer::Scripts(_)
            | Buffer::Logs(_)
            | Buffer::Highlights(_) => Task::none(),
            Buffer::Channel(channel) => channel.focus().map(Message::Channel),
            Buffer::Server(server) => server.focus().map(Message::Server),
            Buffer::Query(query) => query.focus().map(Message::Query),
            Buffer::ChannelDiscovery(channel_discovery) => {
                channel_discovery.focus().map(Message::ChannelList)
            }
        }
    }

    pub fn reset(&mut self) {
        match self {
            Buffer::Empty
            | Buffer::FileTransfers(_)
            | Buffer::Scripts(_)
            | Buffer::Logs(_)
            | Buffer::Highlights(_)
            | Buffer::ChannelDiscovery(_) => {}
            Buffer::Channel(channel) => channel.reset(),
            Buffer::Server(server) => server.reset(),
            Buffer::Query(query) => query.reset(),
        }
    }

    pub fn insert_user_to_input(
        &mut self,
        nick: Nick,
        history: &mut history::Manager,
        autocomplete: &Autocomplete,
    ) {
        match self {
            Buffer::Empty
            | Buffer::FileTransfers(_)
            | Buffer::Scripts(_)
            | Buffer::Logs(_)
            | Buffer::Highlights(_)
            | Buffer::ChannelDiscovery(_) => (),
            Buffer::Server(state) => state.input_view.insert_user(
                nick,
                state.buffer.clone(),
                history,
                autocomplete,
            ),
            Buffer::Channel(state) => state.input_view.insert_user(
                nick,
                state.buffer.clone(),
                history,
                autocomplete,
            ),
            Buffer::Query(state) => state.input_view.insert_user(
                nick,
                state.buffer.clone(),
                history,
                autocomplete,
            ),
        }
    }

    pub fn scroll_up_page(&mut self) -> Task<Message> {
        match self {
            Buffer::Empty
            | Buffer::FileTransfers(_)
            | Buffer::Scripts(_)
            | Buffer::ChannelDiscovery(_) => Task::none(),
            Buffer::Channel(channel) => {
                channel.scroll_view.scroll_up_page().map(|message| {
                    Message::Channel(channel::Message::ScrollView(message))
                })
            }
            Buffer::Server(server) => {
                server.scroll_view.scroll_up_page().map(|message| {
                    Message::Server(server::Message::ScrollView(message))
                })
            }
            Buffer::Query(query) => {
                query.scroll_view.scroll_up_page().map(|message| {
                    Message::Query(query::Message::ScrollView(message))
                })
            }
            Buffer::Logs(log) => {
                log.scroll_view.scroll_up_page().map(|message| {
                    Message::Logs(logs::Message::ScrollView(message))
                })
            }
            Buffer::Highlights(highlights) => {
                highlights.scroll_view.scroll_up_page().map(|message| {
                    Message::Highlights(highlights::Message::ScrollView(
                        message,
                    ))
                })
            }
        }
    }

    pub fn scroll_down_page(&mut self) -> Task<Message> {
        match self {
            Buffer::Empty
            | Buffer::FileTransfers(_)
            | Buffer::Scripts(_)
            | Buffer::ChannelDiscovery(_) => Task::none(),
            Buffer::Channel(channel) => {
                channel.scroll_view.scroll_down_page().map(|message| {
                    Message::Channel(channel::Message::ScrollView(message))
                })
            }
            Buffer::Server(server) => {
                server.scroll_view.scroll_down_page().map(|message| {
                    Message::Server(server::Message::ScrollView(message))
                })
            }
            Buffer::Query(query) => {
                query.scroll_view.scroll_down_page().map(|message| {
                    Message::Query(query::Message::ScrollView(message))
                })
            }
            Buffer::Logs(log) => {
                log.scroll_view.scroll_down_page().map(|message| {
                    Message::Logs(logs::Message::ScrollView(message))
                })
            }
            Buffer::Highlights(highlights) => {
                highlights.scroll_view.scroll_down_page().map(|message| {
                    Message::Highlights(highlights::Message::ScrollView(
                        message,
                    ))
                })
            }
        }
    }

    pub fn scroll_to_start(&mut self, config: &Config) -> Task<Message> {
        match self {
            Buffer::Empty
            | Buffer::FileTransfers(_)
            | Buffer::Scripts(_)
            | Buffer::ChannelDiscovery(_) => Task::none(),
            Buffer::Channel(channel) => {
                channel.scroll_view.scroll_to_start(config).map(|message| {
                    Message::Channel(channel::Message::ScrollView(message))
                })
            }
            Buffer::Server(server) => {
                server.scroll_view.scroll_to_start(config).map(|message| {
                    Message::Server(server::Message::ScrollView(message))
                })
            }
            Buffer::Query(query) => {
                query.scroll_view.scroll_to_start(config).map(|message| {
                    Message::Query(query::Message::ScrollView(message))
                })
            }
            Buffer::Logs(log) => {
                log.scroll_view.scroll_to_start(config).map(|message| {
                    Message::Logs(logs::Message::ScrollView(message))
                })
            }
            Buffer::Highlights(highlights) => highlights
                .scroll_view
                .scroll_to_start(config)
                .map(|message| {
                    Message::Highlights(highlights::Message::ScrollView(
                        message,
                    ))
                }),
        }
    }

    pub fn scroll_to_end(&mut self, config: &Config) -> Task<Message> {
        match self {
            Buffer::Empty
            | Buffer::FileTransfers(_)
            | Buffer::Scripts(_)
            | Buffer::ChannelDiscovery(_) => Task::none(),
            Buffer::Channel(channel) => {
                channel.scroll_view.scroll_to_end(config).map(|message| {
                    Message::Channel(channel::Message::ScrollView(message))
                })
            }
            Buffer::Server(server) => {
                server.scroll_view.scroll_to_end(config).map(|message| {
                    Message::Server(server::Message::ScrollView(message))
                })
            }
            Buffer::Query(query) => {
                query.scroll_view.scroll_to_end(config).map(|message| {
                    Message::Query(query::Message::ScrollView(message))
                })
            }
            Buffer::Logs(log) => {
                log.scroll_view.scroll_to_end(config).map(|message| {
                    Message::Logs(logs::Message::ScrollView(message))
                })
            }
            Buffer::Highlights(highlights) => {
                highlights.scroll_view.scroll_to_end(config).map(|message| {
                    Message::Highlights(highlights::Message::ScrollView(
                        message,
                    ))
                })
            }
        }
    }

    pub fn scroll_to_message(
        &mut self,
        message: message::Hash,
        history: &history::Manager,
        config: &Config,
    ) -> Task<Message> {
        match self {
            Buffer::Empty
            | Buffer::FileTransfers(_)
            | Buffer::Scripts(_)
            | Buffer::ChannelDiscovery(_) => Task::none(),
            Buffer::Channel(state) => state
                .scroll_view
                .scroll_to_message(
                    message,
                    scroll_view::Kind::Channel(&state.server, &state.target),
                    history,
                    config,
                )
                .map(|message| {
                    Message::Channel(channel::Message::ScrollView(message))
                }),
            Buffer::Server(state) => state
                .scroll_view
                .scroll_to_message(
                    message,
                    scroll_view::Kind::Server(&state.server),
                    history,
                    config,
                )
                .map(|message| {
                    Message::Server(server::Message::ScrollView(message))
                }),
            Buffer::Query(state) => state
                .scroll_view
                .scroll_to_message(
                    message,
                    scroll_view::Kind::Query(&state.server, &state.target),
                    history,
                    config,
                )
                .map(|message| {
                    Message::Query(query::Message::ScrollView(message))
                }),
            Buffer::Logs(state) => state
                .scroll_view
                .scroll_to_message(
                    message,
                    scroll_view::Kind::Logs,
                    history,
                    config,
                )
                .map(|message| {
                    Message::Logs(logs::Message::ScrollView(message))
                }),
            Buffer::Highlights(state) => state
                .scroll_view
                .scroll_to_message(
                    message,
                    scroll_view::Kind::Highlights,
                    history,
                    config,
                )
                .map(|message| {
                    Message::Highlights(highlights::Message::ScrollView(
                        message,
                    ))
                }),
        }
    }

    pub fn scroll_to_backlog(
        &mut self,
        history: &history::Manager,
        config: &Config,
    ) -> Task<Message> {
        match self {
            Buffer::Empty
            | Buffer::FileTransfers(_)
            | Buffer::Scripts(_)
            | Buffer::ChannelDiscovery(_) => Task::none(),
            Buffer::Channel(state) => state
                .scroll_view
                .scroll_to_backlog(
                    scroll_view::Kind::Channel(&state.server, &state.target),
                    history,
                    config,
                )
                .map(|message| {
                    Message::Channel(channel::Message::ScrollView(message))
                }),
            Buffer::Server(state) => state
                .scroll_view
                .scroll_to_backlog(
                    scroll_view::Kind::Server(&state.server),
                    history,
                    config,
                )
                .map(|message| {
                    Message::Server(server::Message::ScrollView(message))
                }),
            Buffer::Query(state) => state
                .scroll_view
                .scroll_to_backlog(
                    scroll_view::Kind::Query(&state.server, &state.target),
                    history,
                    config,
                )
                .map(|message| {
                    Message::Query(query::Message::ScrollView(message))
                }),
            Buffer::Logs(state) => state
                .scroll_view
                .scroll_to_backlog(scroll_view::Kind::Logs, history, config)
                .map(|message| {
                    Message::Logs(logs::Message::ScrollView(message))
                }),
            Buffer::Highlights(state) => state
                .scroll_view
                .scroll_to_backlog(
                    scroll_view::Kind::Highlights,
                    history,
                    config,
                )
                .map(|message| {
                    Message::Highlights(highlights::Message::ScrollView(
                        message,
                    ))
                }),
        }
    }

    pub fn has_pending_scroll_to(&self) -> bool {
        match self {
            Buffer::Empty
            | Buffer::FileTransfers(_)
            | Buffer::Scripts(_)
            | Buffer::ChannelDiscovery(_) => false,
            Buffer::Channel(state) => state.scroll_view.has_pending_scroll_to(),
            Buffer::Server(state) => state.scroll_view.has_pending_scroll_to(),
            Buffer::Query(state) => state.scroll_view.has_pending_scroll_to(),
            Buffer::Logs(state) => state.scroll_view.has_pending_scroll_to(),
            Buffer::Highlights(state) => {
                state.scroll_view.has_pending_scroll_to()
            }
        }
    }

    pub fn prepare_for_pending_scroll_to(
        &mut self,
        history: &history::Manager,
        config: &Config,
    ) -> Task<Message> {
        match self {
            Buffer::Empty
            | Buffer::FileTransfers(_)
            | Buffer::Scripts(_)
            | Buffer::ChannelDiscovery(_) => Task::none(),
            Buffer::Channel(state) => state
                .scroll_view
                .prepare_for_pending_scroll_to(
                    scroll_view::Kind::Channel(&state.server, &state.target),
                    history,
                    config,
                )
                .map(|message| {
                    Message::Channel(channel::Message::ScrollView(message))
                }),
            Buffer::Server(state) => state
                .scroll_view
                .prepare_for_pending_scroll_to(
                    scroll_view::Kind::Server(&state.server),
                    history,
                    config,
                )
                .map(|message| {
                    Message::Server(server::Message::ScrollView(message))
                }),
            Buffer::Query(state) => state
                .scroll_view
                .prepare_for_pending_scroll_to(
                    scroll_view::Kind::Query(&state.server, &state.target),
                    history,
                    config,
                )
                .map(|message| {
                    Message::Query(query::Message::ScrollView(message))
                }),
            Buffer::Logs(state) => state
                .scroll_view
                .prepare_for_pending_scroll_to(
                    scroll_view::Kind::Logs,
                    history,
                    config,
                )
                .map(|message| {
                    Message::Logs(logs::Message::ScrollView(message))
                }),
            Buffer::Highlights(state) => state
                .scroll_view
                .prepare_for_pending_scroll_to(
                    scroll_view::Kind::Highlights,
                    history,
                    config,
                )
                .map(|message| {
                    Message::Highlights(highlights::Message::ScrollView(
                        message,
                    ))
                }),
        }
    }

    pub fn is_scrolled_to_bottom(&self) -> Option<bool> {
        match self {
            Buffer::Empty
            | Buffer::FileTransfers(_)
            | Buffer::Scripts(_)
            | Buffer::ChannelDiscovery(_) => None,
            Buffer::Channel(channel) => {
                Some(channel.scroll_view.is_scrolled_to_bottom())
            }
            Buffer::Server(server) => {
                Some(server.scroll_view.is_scrolled_to_bottom())
            }
            Buffer::Query(query) => {
                Some(query.scroll_view.is_scrolled_to_bottom())
            }
            Buffer::Logs(log) => Some(log.scroll_view.is_scrolled_to_bottom()),
            Buffer::Highlights(highlights) => {
                Some(highlights.scroll_view.is_scrolled_to_bottom())
            }
        }
    }

    pub fn close_picker(&mut self) -> bool {
        match self {
            Buffer::Empty
            | Buffer::FileTransfers(_)
            | Buffer::Scripts(_)
            | Buffer::Logs(_)
            | Buffer::Highlights(_)
            | Buffer::ChannelDiscovery(_) => false,
            Buffer::Server(state) => state.input_view.close_picker(),
            Buffer::Channel(state) => state.input_view.close_picker(),
            Buffer::Query(state) => state.input_view.close_picker(),
        }
    }

    pub fn update_pane_size(&mut self, pane_size: Size, config: &Config) {
        match self {
            Buffer::Empty
            | Buffer::FileTransfers(_)
            | Buffer::Scripts(_)
            | Buffer::ChannelDiscovery(_) => (),
            Buffer::Channel(channel) => {
                channel.scroll_view.update_pane_size(pane_size, config);
            }
            Buffer::Server(server) => {
                server.scroll_view.update_pane_size(pane_size, config);
            }
            Buffer::Query(query) => {
                query.scroll_view.update_pane_size(pane_size, config);
            }
            Buffer::Logs(log) => {
                log.scroll_view.update_pane_size(pane_size, config);
            }
            Buffer::Highlights(highlights) => {
                highlights.scroll_view.update_pane_size(pane_size, config);
            }
        }
    }
}
