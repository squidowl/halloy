use std::path::PathBuf;

use data::dashboard::BufferAction;
use data::preview::{self, Previews};
use data::target::{self, Target};
use data::{Config, Server, buffer, history, message};
use iced::advanced::text;
use iced::widget::{column, container, row, vertical_space};
use iced::{Length, Task};

use super::{input_view, scroll_view, user_context};
use crate::widget::{
    Element, message_content, message_marker, selectable_text,
};
use crate::{Theme, theme};

#[derive(Debug, Clone)]
pub enum Message {
    ScrollView(scroll_view::Message),
    InputView(input_view::Message),
}

pub enum Event {
    UserContext(user_context::Event),
    OpenBuffers(Vec<(Target, BufferAction)>),
    History(Task<history::manager::Message>),
    RequestOlderChatHistory,
    PreviewChanged,
    HidePreview(history::Kind, message::Hash, url::Url),
    MarkAsRead(history::Kind),
    OpenUrl(String),
    ImagePreview(PathBuf, url::Url),
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
    let casemapping = clients.get_casemapping(server);
    let query = &state.target;
    let status = clients.status(server);
    let buffer = &state.buffer;
    let input = history.input(buffer);

    let chathistory_state =
        clients.get_chathistory_state(server, &query.to_target());

    let previews = Some(Previews::new(
        previews,
        &query.to_target(),
        &config.preview,
        casemapping,
    ));

    let messages = container(
        scroll_view::view(
            &state.scroll_view,
            scroll_view::Kind::Query(server, query),
            history,
            previews,
            chathistory_state,
            config,
            move |message, max_nick_width, _| {
                let timestamp = config
                    .buffer
                    .format_timestamp(&message.server_time)
                    .map(|timestamp| {
                        selectable_text(timestamp)
                            .style(theme::selectable_text::timestamp)
                    });

                let space = selectable_text(" ");

                match message.target.source() {
                    message::Source::User(user) => {
                        let with_access_levels =
                            config.buffer.nickname.show_access_levels;
                        let mut text = selectable_text(
                            config
                                .buffer
                                .nickname
                                .brackets
                                .format(user.display(with_access_levels)),
                        )
                        .style(|theme| {
                            theme::selectable_text::nickname(
                                theme, config, user,
                            )
                        });

                        if let Some(width) = max_nick_width {
                            text = text
                                .width(width)
                                .align_x(text::Alignment::Right);
                        }

                        let nick = user_context::view(
                            text,
                            server,
                            casemapping,
                            None,
                            user,
                            None,
                            None,
                            config,
                            &config.buffer.nickname.click,
                        )
                        .map(scroll_view::Message::UserContext);

                        let message_content = message_content::with_context(
                            &message.content,
                            casemapping,
                            theme,
                            scroll_view::Message::Link,
                            theme::selectable_text::default,
                            move |link| match link {
                                message::Link::User(_) => {
                                    user_context::Entry::list(false, None)
                                }
                                _ => vec![],
                            },
                            move |link, entry, length| match link {
                                message::Link::User(user) => entry
                                    .view(
                                        server,
                                        casemapping,
                                        None,
                                        user,
                                        None,
                                        length,
                                        config,
                                    )
                                    .map(scroll_view::Message::UserContext),
                                _ => row![].into(),
                            },
                            config,
                        );

                        let timestamp_nickname_row =
                            row![].push_maybe(timestamp).push(nick).push(space);

                        let text_container = container(message_content);

                        match &config.buffer.nickname.alignment {
                            data::buffer::Alignment::Left
                            | data::buffer::Alignment::Right => Some(
                                row![]
                                    .push(timestamp_nickname_row)
                                    .push(text_container)
                                    .into(),
                            ),
                            data::buffer::Alignment::Top => Some(
                                column![]
                                    .push(timestamp_nickname_row)
                                    .push(text_container)
                                    .into(),
                            ),
                        }
                    }
                    message::Source::Server(server) => {
                        let message_style = move |message_theme: &Theme| {
                            theme::selectable_text::server(
                                message_theme,
                                server.as_ref(),
                            )
                        };

                        let marker =
                            message_marker(max_nick_width, message_style);

                        let message = message_content(
                            &message.content,
                            casemapping,
                            theme,
                            scroll_view::Message::Link,
                            message_style,
                            config,
                        );

                        Some(
                            container(
                                row![]
                                    .push_maybe(timestamp)
                                    .push(marker)
                                    .push(space)
                                    .push(message),
                            )
                            .into(),
                        )
                    }
                    message::Source::Action(_) => {
                        let marker = message_marker(
                            max_nick_width,
                            theme::selectable_text::action,
                        );

                        let message_content = message_content(
                            &message.content,
                            casemapping,
                            theme,
                            scroll_view::Message::Link,
                            theme::selectable_text::action,
                            config,
                        );

                        let text_container = container(message_content);

                        Some(
                            container(
                                row![]
                                    .push_maybe(timestamp)
                                    .push(marker)
                                    .push(space)
                                    .push(text_container),
                            )
                            .into(),
                        )
                    }
                    message::Source::Internal(
                        message::source::Internal::Status(status),
                    ) => {
                        let message_style = move |message_theme: &Theme| {
                            theme::selectable_text::status(
                                message_theme,
                                *status,
                            )
                        };

                        let marker =
                            message_marker(max_nick_width, message_style);

                        let message = message_content(
                            &message.content,
                            casemapping,
                            theme,
                            scroll_view::Message::Link,
                            message_style,
                            config,
                        );

                        Some(
                            container(
                                row![]
                                    .push_maybe(timestamp)
                                    .push(marker)
                                    .push(space)
                                    .push(message),
                            )
                            .into(),
                        )
                    }
                    message::Source::Internal(
                        message::source::Internal::Logs,
                    ) => None,
                }
            },
        )
        .map(Message::ScrollView),
    )
    .height(Length::Fill);

    let show_text_input = match config.buffer.text_input.visibility {
        data::buffer::TextInputVisibility::Focused => is_focused,
        data::buffer::TextInputVisibility::Always => true,
    };

    let text_input = show_text_input.then(|| {
        column![
            vertical_space().height(4),
            input_view::view(
                &state.input_view,
                input,
                is_focused,
                !status.connected(),
                config
            )
            .map(Message::InputView)
        ]
        .width(Length::Fill)
    });

    let scrollable = column![messages]
        .push_maybe(text_input)
        .height(Length::Fill);

    container(scrollable)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(8)
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
    pub fn new(server: Server, target: target::Query) -> Self {
        Self {
            buffer: buffer::Upstream::Query(server.clone(), target.clone()),
            server,
            target,
            scroll_view: scroll_view::State::new(),
            input_view: input_view::State::new(),
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        clients: &mut data::client::Map,
        history: &mut history::Manager,
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
                    scroll_view::Event::UserContext(event) => {
                        Some(Event::UserContext(event))
                    }
                    scroll_view::Event::OpenBuffer(target, buffer_action) => {
                        Some(Event::OpenBuffers(vec![(target, buffer_action)]))
                    }
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
                });

                (command.map(Message::ScrollView), event)
            }
            Message::InputView(message) => {
                let (command, event) = self.input_view.update(
                    message,
                    &self.buffer,
                    clients,
                    history,
                    config,
                );
                let command = command.map(Message::InputView);

                match event {
                    Some(input_view::Event::InputSent { history_task }) => {
                        let command = Task::batch(vec![
                            command,
                            self.scroll_view
                                .scroll_to_end()
                                .map(Message::ScrollView),
                        ]);

                        (command, Some(Event::History(history_task)))
                    }
                    Some(input_view::Event::OpenBuffers { targets }) => {
                        (command, Some(Event::OpenBuffers(targets)))
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
