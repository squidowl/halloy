use data::server::Server;
use data::user::Nick;
use data::User;
use data::isupport;
use data::{channel, history, message, Config};
use iced::widget::{column, container, row};
use iced::{padding, Length, Task};

use pad::{PadStr, Alignment};

use super::{input_view, scroll_view, user_context};
use crate::widget::{message_content, selectable_text, Element};
use crate::{theme, Theme};

mod topic;

#[derive(Debug, Clone)]
pub enum Message {
    ScrollView(scroll_view::Message),
    InputView(input_view::Message),
    UserContext(user_context::Message),
}

#[derive(Debug, Clone)]
pub enum Event {
    UserContext(user_context::Event),
}

pub fn view<'a>(
    state: &'a Channel,
    clients: &'a data::client::Map,
    history: &'a history::Manager,
    settings: &'a channel::Settings,
    config: &'a Config,
    theme: &'a Theme,
    is_focused: bool,
) -> Element<'a, Message> {
    let buffer = state.buffer();
    let input = history.input(&buffer);
    let our_nick = clients.nickname(&state.server);

    let our_user = our_nick
        .map(|our_nick| User::from(Nick::from(our_nick.as_ref())))
        .and_then(|user| clients.resolve_user_attributes(&state.server, &state.channel, &user));

    let users = clients.get_channel_users(&state.server, &state.channel);

    let isupport = clients.get_isupport(&state.server);

    let nick_length = if let Some(isupport::Parameter::NICKLEN(max_len)) =
        isupport.get(&isupport::Kind::NICKLEN)
        {
            Some(max_len)
        } else {
            None
        }.unwrap_or(&0).to_owned();

    let messages = container(
        scroll_view::view(
            &state.scroll_view,
            scroll_view::Kind::Channel(&state.server, &state.channel),
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
                            selectable_text(config.buffer.nickname.brackets.format(user).pad_to_width_with_alignment(nick_length.into(),Alignment::Right)).style(
                                |theme| {
                                    theme::selectable_text::nickname(
                                        theme,
                                        user.nick_color(
                                            theme.colors(),
                                            &config.buffer.nickname.color,
                                        ),
                                        user.is_away(),
                                    )
                                },
                            ),
                            user,
                            users.iter().find(|current_user| *current_user == user),
                            state.buffer(),
                            our_user,
                        )
                        .map(scroll_view::Message::UserContext);

                        let space = selectable_text(" ");
                        let text = message_content(
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
                                    .push(text),
                            )
                            .style(move |theme| match our_nick {
                                Some(nick)
                                    if message::reference_user(user.nickname(), nick, message) =>
                                {
                                    theme::container::highlight(theme)
                                }
                                _ => Default::default(),
                            })
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
    .width(Length::FillPortion(2))
    .height(Length::Fill);

    let nick_list = nick_list::view(users, &buffer, our_user, config).map(Message::UserContext);

    // If topic toggles from None to Some then it messes with messages' scroll state,
    // so produce a zero-height placeholder when topic is None.
    let topic = topic(state, clients, users, our_user, settings, config, theme)
        .unwrap_or_else(|| column![].into());

    let show_text_input = match config.buffer.text_input.visibility {
        data::buffer::TextInputVisibility::Focused => is_focused,
        data::buffer::TextInputVisibility::Always => true,
    };

    let channels = clients.get_channels(&state.server);
    let is_connected_to_channel = channels.iter().any(|c| c == &state.channel);

    let text_input = show_text_input.then(move || {
        input_view::view(
            &state.input_view,
            input,
            is_focused,
            !is_connected_to_channel,
        )
        .map(Message::InputView)
    });

    let content = column![topic, messages].spacing(4);

    let content = match (
        settings.nicklist.enabled,
        config.buffer.channel.nicklist.position,
    ) {
        (true, data::channel::Position::Left) => {
            row![nick_list, content]
        }
        (true, data::channel::Position::Right) => {
            row![content, nick_list]
        }
        (false, _) => { row![content] }.height(Length::Fill),
    };

    let body = column![]
        .push(container(content).height(Length::Fill))
        .push_maybe(text_input)
        .spacing(4)
        .height(Length::Fill);

    container(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(padding::all(8).top(4))
        .into()
}

#[derive(Debug, Clone)]
pub struct Channel {
    pub server: Server,
    pub channel: String,

    pub scroll_view: scroll_view::State,
    pub input_view: input_view::State,
}

impl Channel {
    pub fn new(server: Server, channel: String) -> Self {
        Self {
            server,
            channel,
            scroll_view: scroll_view::State::new(),
            input_view: input_view::State::new(),
        }
    }

    pub fn buffer(&self) -> data::Buffer {
        data::Buffer::Channel(self.server.clone(), self.channel.clone())
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
            Message::UserContext(message) => (
                Task::none(),
                user_context::update(message).map(Event::UserContext),
            ),
        }
    }

    pub fn focus(&self) -> Task<Message> {
        self.input_view.focus().map(Message::InputView)
    }

    pub fn reset(&mut self) {
        self.input_view.reset();
    }
}

fn topic<'a>(
    state: &'a Channel,
    clients: &'a data::client::Map,
    users: &'a [User],
    our_user: Option<&'a User>,
    settings: &'a channel::Settings,
    config: &'a Config,
    theme: &'a Theme,
) -> Option<Element<'a, Message>> {
    if !settings.topic.enabled {
        return None;
    }

    let topic = clients.get_channel_topic(&state.server, &state.channel)?;

    Some(
        topic::view(
            topic.content.as_ref()?,
            topic.who.as_deref(),
            topic.time.as_ref(),
            config.buffer.channel.topic.max_lines,
            users,
            &state.buffer(),
            our_user,
            config,
            theme,
        )
        .map(Message::UserContext),
    )
}

mod nick_list {
    use data::{Buffer, Config, User};
    use iced::widget::{column, container, scrollable, text, Scrollable};
    use iced::Length;
    use user_context::Message;

    use crate::buffer::user_context;
    use crate::theme;
    use crate::widget::Element;

    pub fn view<'a>(
        users: &'a [User],
        buffer: &Buffer,
        our_user: Option<&'a User>,
        config: &'a Config,
    ) -> Element<'a, Message> {
        let column = column(users.iter().map(|user| {
            let content = text(user.to_string()).style(|theme| {
                theme::text::nickname(
                    theme,
                    user.nick_color(theme.colors(), &config.buffer.channel.nicklist.color),
                    user.is_away(),
                )
            });

            user_context::view(content, user, Some(user), buffer.clone(), our_user)
        }))
        .padding(4)
        .spacing(1);

        container(
            Scrollable::new(column)
                .direction(scrollable::Direction::Vertical(
                    scrollable::Scrollbar::new().width(1).scroller_width(1),
                ))
                .style(theme::scrollable::hidden),
        )
        .width(Length::Shrink)
        .max_width(120)
        .height(Length::Fill)
        .into()
    }
}
