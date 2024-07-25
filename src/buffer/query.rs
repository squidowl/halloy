use data::user::Nick;
use data::{history, message, Config, Server};
use iced::widget::{column, container, row, vertical_space};
use iced::{Length, Task};

use super::{input_view, scroll_view, user_context};
use crate::widget::{message_content, selectable_text, Element};
use crate::{theme, Theme};

#[derive(Debug, Clone)]
pub enum Message {
    ScrollView(scroll_view::Message),
    InputView(input_view::Message),
}

#[derive(Debug, Clone)]
pub enum Event {
    UserContext(user_context::Event),
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

    let messages = container(
        scroll_view::view(
            &state.scroll_view,
            scroll_view::Kind::Query(&state.server, &state.nick),
            history,
            config,
            move |message| {
                let timestamp =
                    config
                        .buffer
                        .format_timestamp(&message.server_time)
                        .map(|timestamp| {
                            selectable_text(timestamp).style(theme::selectable_text::transparent)
                        });

                match message.target.source() {
                    message::Source::User(user) => {
                        let nick = user_context::view(
                            selectable_text(config.buffer.nickname.brackets.format(user)).style(
                                |theme| {
                                    theme::selectable_text::nickname(
                                        theme,
                                        user.nick_color(
                                            theme.colors(),
                                            &config.buffer.nickname.color,
                                        ),
                                        false,
                                    )
                                },
                            ),
                            user,
                            None,
                            state.buffer(),
                            None,
                        )
                        .map(scroll_view::Message::UserContext);

                        let space = selectable_text(" ");
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
                        let message = message_content(
                            &message.content,
                            theme,
                            scroll_view::Message::Link,
                            move |theme| {
                                theme::selectable_text::server(
                                    theme,
                                    server.as_ref(),
                                    &config.buffer.server_messages,
                                )
                            },
                        );

                        Some(container(row![].push_maybe(timestamp).push(message)).into())
                    }
                    message::Source::Action => {
                        let message = message_content(
                            &message.content,
                            theme,
                            scroll_view::Message::Link,
                            theme::selectable_text::accent,
                        );

                        Some(container(row![].push_maybe(timestamp).push(message)).into())
                    }
                    message::Source::Internal(message::source::Internal::Status(status)) => {
                        let message = message_content(
                            &message.content,
                            theme,
                            scroll_view::Message::Link,
                            move |theme| {
                                theme::selectable_text::status(
                                    theme,
                                    *status,
                                    &config.buffer.internal_messages,
                                )
                            },
                        );

                        Some(container(row![].push_maybe(timestamp).push(message)).into())
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
