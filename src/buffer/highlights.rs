use data::{history, message, Config, Server};
use iced::widget::{container, row, span};
use iced::{Length, Task};

use super::{scroll_view, user_context};
use crate::widget::{message_content, selectable_rich_text, selectable_text, Element};
use crate::{theme, Theme};

#[derive(Debug, Clone)]
pub enum Message {
    ScrollView(scroll_view::Message),
}

pub enum Event {
    UserContext(user_context::Event),
    OpenChannel(String),
    GoToMessage(Server, String, message::Hash),
    History(Task<history::manager::Message>),
}

pub fn view<'a>(
    state: &'a Highlights,
    clients: &'a data::client::Map,
    history: &'a history::Manager,
    config: &'a Config,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let messages = container(
        scroll_view::view(
            &state.scroll_view,
            scroll_view::Kind::Highlights,
            history,
            None,
            config,
            move |message, _, _| match &message.target {
                message::Target::Highlights {
                    server,
                    channel,
                    source: message::Source::User(user),
                } => {
                    let users = clients.get_channel_users(server, channel);

                    let timestamp =
                        config
                            .buffer
                            .format_timestamp(&message.server_time)
                            .map(|timestamp| {
                                selectable_text(timestamp).style(theme::selectable_text::timestamp)
                            });

                    let channel_text = selectable_rich_text::<_, _, (), _, _>(vec![
                        span(channel).color(theme.colors().buffer.url).link(
                            message::Link::GoToMessage(
                                server.clone(),
                                channel.to_string(),
                                message.hash,
                            ),
                        ),
                        span(" "),
                    ])
                    .on_link(scroll_view::Message::Link);

                    let with_access_levels = config.buffer.nickname.show_access_levels;

                    let current_user = users.iter().find(|current_user| *current_user == user);

                    let text = selectable_text(
                        config
                            .buffer
                            .nickname
                            .brackets
                            .format(user.display(with_access_levels)),
                    )
                    .style(|theme| theme::selectable_text::nickname(theme, config, user));

                    let nick =
                        user_context::view(text, server, Some(channel), user, current_user, None)
                            .map(scroll_view::Message::UserContext);

                    let text = message_content::with_context(
                        &message.content,
                        theme,
                        scroll_view::Message::Link,
                        theme::selectable_text::default,
                        move |link| match link {
                            message::Link::User(_) => user_context::Entry::list(true, None),
                            _ => vec![],
                        },
                        move |link, entry, length| match link {
                            message::Link::User(user) => entry
                                .view(server, Some(channel), user, current_user, length)
                                .map(scroll_view::Message::UserContext),
                            _ => row![].into(),
                        },
                        config,
                    );

                    Some(
                        container(
                            row![]
                                .push_maybe(timestamp)
                                .push(channel_text)
                                .push(nick)
                                .push(selectable_text(" "))
                                .push(text),
                        )
                        .into(),
                    )
                }
                _ => None,
            },
        )
        .map(Message::ScrollView),
    )
    .height(Length::Fill);

    container(messages)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(8)
        .into()
}

#[derive(Debug, Clone, Default)]
pub struct Highlights {
    pub scroll_view: scroll_view::State,
}

impl Highlights {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, message: Message) -> (Task<Message>, Option<Event>) {
        match message {
            Message::ScrollView(message) => {
                let (command, event) = self.scroll_view.update(message, false);

                let event = event.and_then(|event| match event {
                    scroll_view::Event::UserContext(event) => Some(Event::UserContext(event)),
                    scroll_view::Event::OpenChannel(channel) => Some(Event::OpenChannel(channel)),
                    scroll_view::Event::GoToMessage(server, channel, message) => {
                        Some(Event::GoToMessage(server, channel, message))
                    }
                    scroll_view::Event::RequestOlderChatHistory => None,
                });

                (command.map(Message::ScrollView), event)
            }
        }
    }
}
