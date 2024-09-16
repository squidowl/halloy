use data::server::Server;
use data::user::Nick;
use data::User;
use data::{channel, history, message, Config};
use iced::widget::{column, container, row};
use iced::{alignment, padding, Length, Task};

use super::{input_view, scroll_view, user_context};
use crate::widget::{message_content, message_marker, selectable_text, Element};
use crate::{theme, Theme};

mod topic;

#[derive(Debug, Clone)]
pub enum Message {
    ScrollView(scroll_view::Message),
    InputView(input_view::Message),
    UserContext(user_context::Message),
    Topic(topic::Message),
}

#[derive(Debug, Clone)]
pub enum Event {
    UserContext(user_context::Event),
    OpenChannel(String),
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
    let buffer = &state.buffer;
    let input = history.input(buffer);
    let our_nick = clients.nickname(&state.server);

    let our_user = our_nick
        .map(|our_nick| User::from(Nick::from(our_nick.as_ref())))
        .and_then(|user| clients.resolve_user_attributes(&state.server, &state.channel, &user));

    let users = clients.get_channel_users(&state.server, &state.channel);

    let messages = container(
        scroll_view::view(
            &state.scroll_view,
            scroll_view::Kind::Channel(&state.server, &state.channel),
            history,
            config,
            move |message, max_nick_width, max_prefix_width| {
                let timestamp =
                    config
                        .buffer
                        .format_timestamp(&message.server_time)
                        .map(|timestamp| {
                            selectable_text(timestamp).style(theme::selectable_text::timestamp)
                        });

                let prefix = message.target.prefix().map_or(
                    max_nick_width.and_then(|_| {
                        max_prefix_width.map(|width| {
                            selectable_text("")
                                .width(width)
                                .horizontal_alignment(alignment::Horizontal::Right)
                        })
                    }),
                    |prefix| {
                        let text = selectable_text(format!(
                            "{} ",
                            config.buffer.status_message_prefix.brackets.format(prefix)
                        ))
                        .style(theme::selectable_text::tertiary);

                        if let Some(width) = max_prefix_width {
                            Some(
                                text.width(width)
                                    .horizontal_alignment(alignment::Horizontal::Right),
                            )
                        } else {
                            Some(text)
                        }
                    },
                );

                let space = selectable_text(" ");
                let with_access_levels = config.buffer.nickname.show_access_levels;

                match message.target.source() {
                    message::Source::User(user) => {
                        let current_user = users.iter().find(|current_user| *current_user == user);

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
                                user.nick_color(theme.colors(), config.buffer.nickname.color),
                                user.is_away(),
                            )
                        });

                        if let Some(width) = max_nick_width {
                            text = text
                                .width(width)
                                .horizontal_alignment(alignment::Horizontal::Right);
                        }

                        let nick = user_context::view(text, user, current_user, buffer, our_user)
                            .map(scroll_view::Message::UserContext);

                        let text = message_content::with_context(
                            &message.content,
                            theme,
                            scroll_view::Message::Link,
                            theme::selectable_text::default,
                            move |link| match link {
                                message::Link::User(_) => {
                                    user_context::Entry::list(buffer, our_user)
                                }
                                _ => vec![],
                            },
                            move |link, entry, length| match link {
                                message::Link::User(user) => entry
                                    .view(user, current_user, length)
                                    .map(scroll_view::Message::UserContext),
                                _ => row![].into(),
                            },
                            config,
                        );

                        Some(
                            row![]
                                .push(container(
                                    row![]
                                        .push_maybe(timestamp)
                                        .push_maybe(prefix)
                                        .push(nick)
                                        .push(space),
                                ))
                                .push(container(text).style(move |theme| match our_nick {
                                    Some(nick)
                                        if message::reference_user(
                                            user.nickname(),
                                            nick,
                                            message,
                                        ) =>
                                    {
                                        theme::container::highlight(theme)
                                    }
                                    _ => Default::default(),
                                }))
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
                            config,
                        );

                        Some(
                            container(
                                row![]
                                    .push_maybe(timestamp)
                                    .push_maybe(prefix)
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
                            config,
                        );

                        Some(
                            container(
                                row![]
                                    .push_maybe(timestamp)
                                    .push_maybe(prefix)
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
                            config,
                        );

                        Some(
                            container(
                                row![]
                                    .push_maybe(timestamp)
                                    .push_maybe(prefix)
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
    .width(Length::FillPortion(2))
    .height(Length::Fill);

    let nick_list = nick_list::view(users, buffer, our_user, config).map(Message::UserContext);

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
        (true, data::channel::Position::Left) => row![nick_list, content],
        (true, data::channel::Position::Right) => row![content, nick_list],
        (false, _) => { row![content] }.height(Length::Fill),
    }
    .spacing(4);

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
    pub buffer: data::Buffer,
    pub server: Server,
    pub channel: String,

    pub scroll_view: scroll_view::State,
    pub input_view: input_view::State,
}

impl Channel {
    pub fn new(server: Server, channel: String) -> Self {
        Self {
            buffer: data::Buffer::Channel(server.clone(), channel.clone()),
            server,
            channel,
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
                let (command, event) = self.scroll_view.update(message);

                let event = event.map(|event| match event {
                    scroll_view::Event::UserContext(event) => Event::UserContext(event),
                    scroll_view::Event::OpenChannel(channel) => Event::OpenChannel(channel),
                });

                (command.map(Message::ScrollView), event)
            }
            Message::InputView(message) => {
                let (command, event) =
                    self.input_view
                        .update(message, &self.buffer, clients, history, config);
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
            Message::Topic(message) => (
                Task::none(),
                topic::update(message).map(|event| match event {
                    topic::Event::UserContext(event) => Event::UserContext(event),
                    topic::Event::OpenChannel(channel) => Event::OpenChannel(channel),
                }),
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
            &state.buffer,
            our_user,
            config,
            theme,
        )
        .map(Message::Topic),
    )
}

mod nick_list {
    use data::{config, Buffer, Config, User};
    use iced::widget::{column, scrollable, Scrollable};
    use iced::{alignment, Length};
    use user_context::Message;

    use crate::buffer::user_context;
    use crate::widget::{selectable_text, Element};
    use crate::{font, theme};

    pub fn view<'a>(
        users: &'a [User],
        buffer: &'a Buffer,
        our_user: Option<&'a User>,
        config: &'a Config,
    ) -> Element<'a, Message> {
        let nicklist_config = &config.buffer.channel.nicklist;

        let width = match nicklist_config.width {
            Some(width) => width,
            None => {
                let max_nick_length = users
                    .iter()
                    .map(|user| {
                        user.display(nicklist_config.show_access_levels)
                            .chars()
                            .count()
                    })
                    .max()
                    .unwrap_or_default();

                font::width_from_chars(max_nick_length, &config.font)
            }
        };

        let content = column(users.iter().map(|user| {
            let content = selectable_text(user.display(nicklist_config.show_access_levels))
                .style(|theme| {
                    theme::selectable_text::nickname(
                        theme,
                        user.nick_color(theme.colors(), nicklist_config.color),
                        user.is_away(),
                    )
                })
                .horizontal_alignment(match nicklist_config.alignment {
                    config::channel::Alignment::Left => alignment::Horizontal::Left,
                    config::channel::Alignment::Right => alignment::Horizontal::Right,
                })
                .width(Length::Fixed(width));

            user_context::view(content, user, Some(user), buffer, our_user)
        }));

        Scrollable::new(content)
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(1).scroller_width(1),
            ))
            .width(Length::Shrink)
            .style(theme::scrollable::hidden)
            .into()
    }
}
