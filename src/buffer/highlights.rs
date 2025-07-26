use std::path::PathBuf;

use data::config::buffer::nickname::ShownStatus;
use data::dashboard::BufferAction;
use data::target::{self, Target};
use data::{Config, Server, history, message};
use iced::widget::{container, row, span};
use iced::{Length, Task};

use super::{scroll_view, user_context};
use crate::widget::{
    Element, message_content, selectable_rich_text, selectable_text, tooltip,
};
use crate::{Theme, font, theme};

#[derive(Debug, Clone)]
pub enum Message {
    ScrollView(scroll_view::Message),
}

pub enum Event {
    UserContext(user_context::Event),
    OpenBuffer(Target, BufferAction),
    GoToMessage(Server, target::Channel, message::Hash),
    History(Task<history::manager::Message>),
    OpenUrl(String),
    ImagePreview(PathBuf, url::Url),
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
            None,
            config,
            theme,
            move |message: &'a data::Message, _, _| match &message.target {
                message::Target::Highlights {
                    server,
                    channel,
                    source: message::Source::User(user),
                } => {
                    let users = clients.get_channel_users(server, channel);

                    let timestamp = config
                        .buffer
                        .format_timestamp(&message.server_time)
                        .map(|timestamp| {
                            selectable_text(timestamp)
                                .font_maybe(
                                    theme::font_style::timestamp(theme)
                                        .map(font::get),
                                )
                                .style(theme::selectable_text::timestamp)
                        });

                    let channel_text =
                        selectable_rich_text::<_, _, (), _, _>(vec![
                            span(channel.as_str())
                                .font_maybe(
                                    theme
                                        .styles()
                                        .buffer
                                        .url
                                        .font_style
                                        .map(font::get),
                                )
                                .color(theme.styles().buffer.url.color)
                                .link(message::Link::GoToMessage(
                                    server.clone(),
                                    channel.clone(),
                                    message.hash,
                                )),
                            span(" "),
                        ])
                        .on_link(scroll_view::Message::Link);

                    let with_access_levels =
                        config.buffer.nickname.show_access_levels;
                    let truncate = config.buffer.nickname.truncate;

                    let current_user =
                        users.and_then(|users| users.resolve(user));
                    let is_user_offline =
                        match config.buffer.nickname.shown_status {
                            ShownStatus::Current => current_user.is_none(),
                            ShownStatus::Historical => false,
                        };

                    let text =
                        selectable_text(
                            config.buffer.nickname.brackets.format(
                                user.display(with_access_levels, truncate),
                            ),
                        )
                        .font_maybe(
                            theme::font_style::nickname(theme, is_user_offline)
                                .map(font::get),
                        )
                        .style(move |theme| {
                            theme::selectable_text::nickname(
                                theme,
                                config,
                                match config.buffer.nickname.shown_status {
                                    ShownStatus::Current => {
                                        current_user.unwrap_or(user)
                                    }
                                    ShownStatus::Historical => user,
                                },
                                is_user_offline,
                            )
                        });

                    let casemapping = clients.get_casemapping(server);
                    let prefix = clients.get_prefix(server);

                    let nick = tooltip(
                        user_context::view(
                            text,
                            server,
                            casemapping,
                            prefix,
                            Some(channel),
                            user,
                            current_user,
                            None,
                            config,
                            theme,
                            &config.buffer.nickname.click,
                        )
                        .map(scroll_view::Message::UserContext),
                        // We show the full nickname in the tooltip if truncation is enabled.
                        truncate.map(|_| user.as_str()),
                        tooltip::Position::Bottom,
                        theme,
                    );

                    let text = message_content::with_context(
                        &message.content,
                        casemapping,
                        theme,
                        scroll_view::Message::Link,
                        theme::selectable_text::default,
                        theme::font_style::primary,
                        move |link| match link {
                            message::Link::User(_) => {
                                user_context::Entry::list(true, None)
                            }
                            _ => vec![],
                        },
                        move |link, entry, length| match link {
                            message::Link::User(user) => entry
                                .view(
                                    server,
                                    casemapping,
                                    prefix,
                                    Some(channel),
                                    user,
                                    current_user,
                                    length,
                                    config,
                                    theme,
                                )
                                .map(scroll_view::Message::UserContext),
                            _ => row![].into(),
                        },
                        config,
                    );

                    Some(
                        container(row![
                            timestamp,
                            channel_text,
                            nick,
                            selectable_text(" "),
                            text,
                        ])
                        .into(),
                    )
                }
                message::Target::Highlights {
                    server,
                    channel,
                    source: message::Source::Action(_),
                } => {
                    let timestamp = config
                        .buffer
                        .format_timestamp(&message.server_time)
                        .map(|timestamp| {
                            selectable_text(timestamp)
                                .font_maybe(
                                    theme::font_style::timestamp(theme)
                                        .map(font::get),
                                )
                                .style(theme::selectable_text::timestamp)
                        });

                    let channel_text =
                        selectable_rich_text::<_, _, (), _, _>(vec![
                            span(channel.as_str())
                                .color(theme.styles().buffer.url.color)
                                .link(message::Link::GoToMessage(
                                    server.clone(),
                                    channel.clone(),
                                    message.hash,
                                )),
                            span(" "),
                        ])
                        .on_link(scroll_view::Message::Link);

                    let casemapping = clients.get_casemapping(server);

                    let text = message_content(
                        &message.content,
                        casemapping,
                        theme,
                        scroll_view::Message::Link,
                        theme::selectable_text::action,
                        theme::font_style::action,
                        config,
                    );

                    Some(container(row![timestamp, channel_text, text]).into())
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

    pub fn update(
        &mut self,
        message: Message,
        history: &history::Manager,
        clients: &data::client::Map,
        config: &Config,
    ) -> (Task<Message>, Option<Event>) {
        match message {
            Message::ScrollView(message) => {
                let (command, event) = self.scroll_view.update(
                    message,
                    false,
                    scroll_view::Kind::Highlights,
                    history,
                    clients,
                    config,
                );

                let event = event.and_then(|event| match event {
                    scroll_view::Event::UserContext(event) => {
                        Some(Event::UserContext(event))
                    }
                    scroll_view::Event::OpenBuffer(target, buffer_action) => {
                        Some(Event::OpenBuffer(target, buffer_action))
                    }
                    scroll_view::Event::GoToMessage(
                        server,
                        channel,
                        message,
                    ) => Some(Event::GoToMessage(server, channel, message)),
                    scroll_view::Event::RequestOlderChatHistory => None,
                    scroll_view::Event::PreviewChanged => None,
                    scroll_view::Event::HidePreview(..) => None,
                    scroll_view::Event::MarkAsRead => None,
                    scroll_view::Event::OpenUrl(url) => {
                        Some(Event::OpenUrl(url))
                    }
                    scroll_view::Event::ImagePreview(path, url) => {
                        Some(Event::ImagePreview(path, url))
                    }
                });

                (command.map(Message::ScrollView), event)
            }
        }
    }
}
