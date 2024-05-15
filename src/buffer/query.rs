use data::user::Nick;
use data::{history, message, Config, Server};
use iced::widget::{column, container, row, vertical_space};
use iced::{alignment, Length, Task};

use super::{input_view, scroll_view, user_context};
use crate::widget::{message_content, message_marker, selectable_text, Element};
use crate::{theme, Theme};

#[derive(Debug, Clone)]
pub enum Message {
    ScrollView(scroll_view::Message),
    InputView(input_view::Message),
}

#[derive(Debug, Clone)]
pub enum Event {
    UserContext(user_context::Event),
    ScrolledToTop,
    ChatHistoryBeforeRequest,
}

pub fn view<'a>(
    state: &'a Query,
    clients: &'a data::client::Map,
    history: &'a history::Manager,
    config: &'a Config,
    theme: &'a Theme,
    is_focused: bool,
) -> Element<'a, Message> {
    let status = clients.status(&state.server);
    let buffer = state.buffer();
    let input = history.input(&buffer);

    let chathistory_before_button = if clients.get_server_supports_chathistory(&state.server) {
        Some((
            clients
                .get_chathistory_request(&state.server, state.nick.as_ref())
                .is_some(),
            clients.get_chathistory_exhausted(&state.server, state.nick.as_ref()),
        ))
    } else {
        None
    };

    let messages = container(
        scroll_view::view(
            &state.scroll_view,
            scroll_view::Kind::Query(&state.server, &state.nick, chathistory_before_button),
            history,
            config,
            move |message, max_nick_width, _| {
                let timestamp =
                    config
                        .buffer
                        .format_timestamp(&message.server_time)
                        .map(|timestamp| {
                            selectable_text(timestamp).style(theme::selectable_text::timestamp)
                        });

                let space = selectable_text(" ");

                match message.target.source() {
                    message::Source::User(user) => {
                        let with_access_levels = config.buffer.nickname.show_access_levels;
                        let mut text = selectable_text(
                            config
                                .buffer
                                .nickname
                                .brackets
                                .format(user.display(with_access_levels)),
                        )
                        .style(|theme| {
                            theme::selectable_text::nickname(
                                theme,
                                user.nick_color(theme.colors(), &config.buffer.nickname.color),
                                false,
                            )
                        });

                        if let Some(width) = max_nick_width {
                            text = text
                                .width(width)
                                .horizontal_alignment(alignment::Horizontal::Right);
                        }

                        let nick = user_context::view(text, user, None, state.buffer(), None)
                            .map(scroll_view::Message::UserContext);

                        let message = message_content(
                            &message.content,
                            theme,
                            scroll_view::Message::Link,
                            theme::selectable_text::default,
                        );

                        Some(
                            container(
                                row![]
                                    .push_maybe(timestamp)
                                    .push(nick)
                                    .push(space)
                                    .push(message),
                            )
                            .into(),
                        )
                    }
                    message::Source::Server(server) => {
                        let message_style = move |message_theme: &Theme| {
                            theme::selectable_text::server(message_theme, server.as_ref())
                        };

                        let marker = message_marker(max_nick_width, message_style);

                        let message = message_content(
                            &message.content,
                            theme,
                            scroll_view::Message::Link,
                            message_style,
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
                    message::Source::Action => {
                        let marker = message_marker(max_nick_width, theme::selectable_text::action);

                        let message = message_content(
                            &message.content,
                            theme,
                            scroll_view::Message::Link,
                            theme::selectable_text::action,
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
                    message::Source::Internal(message::source::Internal::Status(status)) => {
                        let message_style = move |message_theme: &Theme| {
                            theme::selectable_text::status(message_theme, *status)
                        };

                        let marker = message_marker(max_nick_width, message_style);

                        let message = message_content(
                            &message.content,
                            theme,
                            scroll_view::Message::Link,
                            message_style,
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
            input_view::view(&state.input_view, input, is_focused, !status.connected())
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
    pub server: Server,
    pub nick: Nick,
    pub scroll_view: scroll_view::State,
    pub input_view: input_view::State,
}

impl Query {
    pub fn new(server: Server, nick: Nick) -> Self {
        Self {
            server,
            nick,
            scroll_view: scroll_view::State::new(),
            input_view: input_view::State::new(),
        }
    }

    pub fn buffer(&self) -> data::Buffer {
        data::Buffer::Query(self.server.clone(), self.nick.clone())
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
                let (command, event) = self.scroll_view.update(message);

                let event = event.map(|event| match event {
                    scroll_view::Event::UserContext(event) => Event::UserContext(event),
                    scroll_view::Event::ScrolledToTop => Event::ScrolledToTop,
                    scroll_view::Event::ChatHistoryBeforeRequest => Event::ChatHistoryBeforeRequest,
                });

                (command.map(Message::ScrollView), event)
            }
            Message::InputView(message) => {
                let buffer = self.buffer();

                let (command, event) = self
                    .input_view
                    .update(message, buffer, clients, history, config);
                let command = command.map(Message::InputView);

                match event {
                    Some(input_view::Event::InputSent) => {
                        let command = Task::batch(vec![
                            command,
                            self.scroll_view.scroll_to_end().map(Message::ScrollView),
                        ]);

                        (command, None)
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
