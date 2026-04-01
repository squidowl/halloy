use std::path::PathBuf;
use std::time::Instant;

use chrono::{DateTime, Utc};
use data::dashboard::BufferAction;
use data::history::filter::FilterChain;
use data::preview::{self, Previews};
use data::target::{self, Target};
use data::user::Nick;
use data::{Config, Preview, Server, User, buffer, client, history, message};
use iced::widget::{column, container, stack};
use iced::{Length, Size, Task, padding};

use super::message_view::{ChannelQueryLayout, TargetInfo};
use super::{context_menu, input_view, scroll_view, typing};
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
    Reconnect(Server),
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
    let can_send_reactions = clients.get_server_can_send_reactions(server);
    let chantypes = clients.get_chantypes(server);
    let casemapping = clients.get_casemapping(server);
    let prefix = clients.get_prefix(server);
    let query = &state.target;
    let confirm_message_delivery = clients.get_server_supports_echoes(server)
        && config.servers.get(server).is_some_and(|server_config| {
            server_config
                .confirm_message_delivery
                .is_target_query_included(query, server, casemapping)
        });
    let our_nick = clients.nickname(server);
    let our_user = our_nick.map(|our_nick| User::from(Nick::from(our_nick)));
    let show_typing = clients.get_server_show_typing(server);
    let typing_style = config.buffer.typing.style;
    let typing_text = state.typing_text(clients, history);
    let has_typing_text = typing_text.is_some();

    let chathistory_state =
        clients.get_chathistory_state(server, &query.to_target());

    let previews = Some(Previews::new(
        previews,
        query.as_target_ref(),
        server,
        &config.preview,
        casemapping,
    ));

    let message_formatter = ChannelQueryLayout {
        config,
        chantypes,
        casemapping,
        prefix,
        confirm_message_delivery,
        can_send_reactions,
        our_nick,
        connected,
        server,
        theme,
        previews,
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
            typing::reserved_bottom_padding(
                has_typing_text,
                typing_style,
                config,
            ),
            config,
            theme,
            message_formatter,
        )
        .map(Message::ScrollView),
    )
    .height(Length::Fill);

    let typing = typing::view(
        typing_text,
        state.typing_animation.as_ref(),
        typing::typing_font_size(config),
        config.buffer.line_spacing,
        &config.buffer.typing.animation,
        theme,
    );

    let show_text_input = match config.buffer.text_input.visibility {
        data::config::buffer::text_input::Visibility::Focused => is_focused,
        data::config::buffer::text_input::Visibility::Always => true,
    };

    let text_input = show_text_input.then(|| {
        input_view::view(
            &state.input_view,
            our_user.as_ref(),
            &state.server,
            config,
            theme,
        )
        .map(Message::InputView)
    });

    let content = column![messages];

    let body: Element<'a, Message> =
        if typing::show_row(show_typing, typing_style, has_typing_text) {
            let typing_overlay: Element<'a, Message> = container(typing)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(padding::left(2))
                .align_y(iced::alignment::Vertical::Bottom)
                .into();

            column![
                stack![content, typing_overlay].height(Length::Fill),
                text_input
            ]
            .height(Length::Fill)
            .into()
        } else {
            column![column![content].height(Length::Fill), text_input]
                .height(Length::Fill)
                .into()
        };

    container(body)
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
    typing_animation: Option<typing::Animation>,
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
            typing_animation: None,
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
                    Some(&self.buffer),
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
                    Some(input_view::Event::Reconnect(server)) => {
                        (command, Some(Event::Reconnect(server)))
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

    pub fn tick(
        &mut self,
        now: Instant,
        clients: &data::client::Map,
        history: &history::Manager,
    ) {
        let is_typing = self.typing_text(clients, history).is_some();
        typing::update(&mut self.typing_animation, is_typing, now);
    }

    fn typing_text(
        &self,
        clients: &data::client::Map,
        history: &history::Manager,
    ) -> Option<String> {
        let server = &self.server;
        let query = &self.target;
        let casemapping = clients.get_casemapping(server);

        typing::typing_text(
            clients.get_server_show_typing(server),
            clients.get_server_supports_typing(server),
            clients
                .nickname(server)
                .as_ref()
                .map(data::user::NickRef::as_str),
            &typing::visible_nicks(
                &clients.get_query_typing_users(server, query),
                None,
                server,
                FilterChain::borrow(history.filters()),
                casemapping,
            ),
            casemapping,
        )
    }
}
