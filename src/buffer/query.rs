use std::path::PathBuf;

use chrono::{DateTime, Utc};
use data::dashboard::BufferAction;
use data::preview::{self, Previews};
use data::target::{self, Target};
use data::user::Nick;
use data::{Config, Preview, Server, User, buffer, client, history, message};
use iced::widget::{column, container, space};
use iced::{Length, Size, Task};

use super::message_view::{ChannelQueryLayout, TargetInfo};
use super::{context_menu, input_view, scroll_view};
use crate::Theme;
use crate::widget::Element;
use crate::window::Window;

#[derive(Debug, Clone)]
pub enum Message {
    ScrollView(scroll_view::Message),
    InputView(input_view::Message),
}

pub enum Event {
    ContextMenu(context_menu::Event),
    OpenBuffers(Server, Vec<(Target, BufferAction)>),
    OpenInternalBuffer(buffer::Internal),
    OpenServer(String),
    LeaveBuffers(Vec<Target>, Option<String>),
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
}

pub fn view<'a>(
    state: &'a Query,
    clients: &'a data::client::Map,
    history: &'a history::Manager,
    previews: &'a preview::Collection,
    config: &'a Config,
    theme: &'a Theme,
    is_focused: bool,
) -> Element<'a, Message> {
    let server = &state.server;
    let connected = matches!(clients.status(server), client::Status::Connected);
    let chantypes = clients.get_chantypes(server);
    let casemapping = clients.get_casemapping(server);
    let prefix = clients.get_prefix(server);
    let supports_echoes = clients.get_server_supports_echoes(server);
    let query = &state.target;
    let status = clients.status(server);
    let our_nick = clients.nickname(server);
    let our_user = our_nick.map(|our_nick| User::from(Nick::from(our_nick)));

    let chathistory_state =
        clients.get_chathistory_state(server, &query.to_target());

    let previews = Some(Previews::new(
        previews,
        &query.to_target(),
        server,
        &config.preview,
        casemapping,
    ));

    let message_formatter = ChannelQueryLayout {
        config,
        chantypes,
        casemapping,
        prefix,
        supports_echoes,
        connected,
        server,
        theme,
        target: TargetInfo::Query,
    };

    let messages = container(
        scroll_view::view(
            &state.scroll_view,
            scroll_view::Kind::Query(server, query),
            history,
            previews,
            Option::<fn(&Preview, &message::Source) -> bool>::None,
            chathistory_state,
            config,
            theme,
            message_formatter,
        )
        .map(Message::ScrollView),
    )
    .height(Length::Fill);

    let show_text_input = match config.buffer.text_input.visibility {
        data::config::buffer::text_input::Visibility::Focused => is_focused,
        data::config::buffer::text_input::Visibility::Always => true,
    };

    let text_input = show_text_input.then(|| {
        column![
            space::vertical().height(4),
            input_view::view(
                &state.input_view,
                our_user.as_ref(),
                !status.connected(),
                config,
                theme,
            )
            .map(Message::InputView)
        ]
        .width(Length::Fill)
    });

    let scrollable = column![messages, text_input].height(Length::Fill);

    container(scrollable)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

#[derive(Debug, Clone)]
pub struct Query {
    pub buffer: buffer::Upstream,
    pub server: Server,
    pub target: target::Query,
    pub scroll_view: scroll_view::State,
    pub input_view: input_view::State,
}

impl Query {
    pub fn new(
        server: Server,
        target: target::Query,
        history: &history::Manager,
        pane_size: Size,
        config: &Config,
    ) -> Self {
        let buffer = buffer::Upstream::Query(server.clone(), target.clone());

        Self {
            input_view: input_view::State::new(Some(
                history.input(&buffer).draft,
            )),
            buffer,
            server,
            target,
            scroll_view: scroll_view::State::new(pane_size, config),
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        clients: &mut data::client::Map,
        history: &mut history::Manager,
        main_window: &Window,
        config: &Config,
    ) -> (Task<Message>, Option<Event>) {
        match message {
            Message::ScrollView(message) => {
                let (command, event) = self.scroll_view.update(
                    message,
                    config.buffer.chathistory.infinite_scroll,
                    scroll_view::Kind::Query(&self.server, &self.target),
                    history,
                    clients,
                    config,
                );

                let event = event.and_then(|event| match event {
                    scroll_view::Event::ContextMenu(event) => {
                        Some(Event::ContextMenu(event))
                    }
                    scroll_view::Event::OpenBuffer(
                        server,
                        target,
                        buffer_action,
                    ) => Some(Event::OpenBuffers(
                        server,
                        vec![(target, buffer_action)],
                    )),
                    scroll_view::Event::GoToMessage(_, _, _) => None,
                    scroll_view::Event::RequestOlderChatHistory => {
                        Some(Event::RequestOlderChatHistory)
                    }
                    scroll_view::Event::PreviewChanged => {
                        Some(Event::PreviewChanged)
                    }
                    scroll_view::Event::HidePreview(kind, hash, url) => {
                        Some(Event::HidePreview(kind, hash, url))
                    }
                    scroll_view::Event::MarkAsRead => {
                        history::Kind::from_buffer(data::Buffer::Upstream(
                            self.buffer.clone(),
                        ))
                        .map(Event::MarkAsRead)
                    }
                    scroll_view::Event::OpenUrl(url) => {
                        Some(Event::OpenUrl(url))
                    }
                    scroll_view::Event::ImagePreview(path, url) => {
                        Some(Event::ImagePreview(path, url))
                    }
                    scroll_view::Event::ExpandCondensedMessage(
                        server_time,
                        hash,
                    ) => Some(Event::ExpandCondensedMessage(server_time, hash)),
                    scroll_view::Event::ContractCondensedMessage(
                        server_time,
                        hash,
                    ) => {
                        Some(Event::ContractCondensedMessage(server_time, hash))
                    }
                });

                (command.map(Message::ScrollView), event)
            }
            Message::InputView(message) => {
                let (command, event) = self.input_view.update(
                    message,
                    &self.buffer,
                    clients,
                    history,
                    main_window,
                    config,
                );
                let command = command.map(Message::InputView);

                match event {
                    Some(input_view::Event::InputSent {
                        history_task,
                        open_buffers,
                    }) => {
                        let command = Task::batch(vec![
                            command,
                            self.scroll_view
                                .scroll_to_end(config)
                                .map(Message::ScrollView),
                        ]);

                        (
                            command,
                            Some(Event::InputSent {
                                history_task,
                                open_buffers,
                            }),
                        )
                    }
                    Some(input_view::Event::OpenBuffers {
                        server,
                        targets,
                    }) => (command, Some(Event::OpenBuffers(server, targets))),
                    Some(input_view::Event::LeaveBuffers {
                        targets,
                        reason,
                    }) => (command, Some(Event::LeaveBuffers(targets, reason))),
                    Some(input_view::Event::Cleared { history_task }) => {
                        (command, Some(Event::History(history_task)))
                    }
                    Some(input_view::Event::OpenInternalBuffer(buffer)) => {
                        (command, Some(Event::OpenInternalBuffer(buffer)))
                    }
                    Some(input_view::Event::OpenServer(server)) => {
                        (command, Some(Event::OpenServer(server)))
                    }
                    None => (command, None),
                }
            }
        }
    }

    pub fn focus(&self) -> Task<Message> {
        self.input_view.focus().map(Message::InputView)
    }

    pub fn reset(&mut self) {
        self.input_view.reset();
    }
}
