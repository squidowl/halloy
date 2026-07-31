use chrono::{DateTime, Utc};
use data::config::buffer::nickname::ShownStatus;
use data::dashboard::BufferAction;
use data::target::{self, Target};
use data::{
    Config, Image, Preview, Server, User, history, message, metadata, preview,
};
use iced::widget::{container, row, span};
use iced::{Color, Length, Size, Task};

use super::context_menu::{
    self, ChannelContext, Context, UrlContext, UserContext,
};
use super::scroll_view;
use crate::widget::user_display::UserDisplay;
use crate::widget::{
    Element, message_content, selectable_rich_text, selectable_text,
};
use crate::{Theme, font, theme};

#[derive(Debug, Clone)]
pub enum Message {
    ScrollView(scroll_view::Message),
}

pub enum Event {
    ContextMenu(context_menu::Event),
    OpenBuffer(Server, Target, BufferAction),
    GoToMessage(Server, target::Channel, message::Hash, BufferAction),
    History(Task<history::manager::Message>),
    OpenUrl(String),
    MarkAsRead,
    ImagePreview(Image),
    ExpandMessage(DateTime<Utc>, message::Hash),
    ContractMessage(DateTime<Utc>, message::Hash),
}

#[derive(Debug, Clone, Copy)]
pub enum Kind {
    Highlights,
    ChannelMonitor,
}

impl Kind {
    pub(super) fn scroll_view(self) -> scroll_view::Kind<'static> {
        match self {
            Self::Highlights => scroll_view::Kind::Highlights,
            Self::ChannelMonitor => scroll_view::Kind::ChannelMonitor,
        }
    }

    pub fn history(self) -> history::Kind {
        match self {
            Self::Highlights => history::Kind::Highlights,
            Self::ChannelMonitor => history::Kind::ChannelMonitor,
        }
    }
}

pub fn view<'a>(
    state: &'a MessageFeed,
    clients: &'a data::client::Map,
    history: &'a history::Manager,
    previews: &'a preview::Collection,
    config: &'a Config,
    theme: &'a Theme,
    channel_is_focused: impl Fn(&Server, &target::Channel) -> bool + Copy + 'a,
    channel_is_open: impl Fn(&Server, &target::Channel) -> bool + Copy + 'a,
) -> Element<'a, Message> {
    let messages = scroll_view::view(
        &state.scroll_view,
        state.kind.scroll_view(),
        history,
        None,
        Option::<fn(&Preview, &message::Source) -> bool>::None,
        None,
        0.0,
        config,
        theme,
        move |message: &'a data::Message, _, _, _| match &message.target {
            message::Target::Highlights {
                server,
                channel,
                source: message::Source::User(user),
            }
            | message::Target::ChannelMonitor {
                server,
                channel,
                source: message::Source::User(user),
            } => {
                let users = clients.get_channel_users(server, channel);

                let timestamp = config
                    .buffer
                    .format_timestamp(&message.server_time)
                    .map(|timestamp| {
                        context_menu::timestamp(
                            selectable_text(timestamp)
                                .font_maybe(
                                    theme::font_style::timestamp(theme)
                                        .map(font::get),
                                )
                                .style(theme::selectable_text::timestamp),
                            &message.server_time,
                            config,
                            theme,
                        )
                        .map(scroll_view::Message::ContextMenu)
                    });

                let channel_text = selectable_rich_text::<
                    _,
                    message::Link,
                    context_menu::Entry,
                    _,
                    _,
                >(vec![
                    span(channel.as_str())
                        .font_maybe(
                            theme.styles().buffer.url.font_style.map(font::get),
                        )
                        .color(theme.styles().buffer.url.color)
                        .link(message::Link::GoToMessage(
                            server.clone(),
                            channel.clone(),
                            message.hash,
                            config
                                .actions
                                .buffer
                                .click_highlight
                                .buffer_action(),
                        )),
                    span(" "),
                ])
                .on_link(scroll_view::Message::Link)
                .context_menu(
                    move |link| {
                        context_menu::Entry::link_list(
                                    link,
                                    Option::<
                                        fn(&User) -> Vec<context_menu::Entry>,
                                    >::None,
                                    Option::<
                                        fn(&str) -> Vec<context_menu::Entry>,
                                    >::None,
                                    Some(|server, channel| {
                                        context_menu::Entry::channel_list(
                                            channel_is_open(server, channel),
                                            channel_is_focused(server, channel),
                                        )
                                    }),
                                )
                    },
                    move |link, entry, length| {
                        entry
                            .view(
                                Context::link(
                                    link,
                                    Option::<fn(&User) -> UserContext>::None,
                                    Option::<fn(&str) -> UrlContext>::None,
                                    Some(|server, channel| ChannelContext {
                                        server,
                                        channel,
                                        is_open: channel_is_open(
                                            server, channel,
                                        ),
                                    }),
                                ),
                                length,
                                config,
                                theme,
                            )
                            .map(scroll_view::Message::ContextMenu)
                    },
                );

                let current_user = users.and_then(|users| users.resolve(user));
                let is_user_away = match config.buffer.nickname.shown_status {
                    ShownStatus::Current => current_user.unwrap_or(user),
                    ShownStatus::Historical => user,
                }
                .is_away();
                let is_user_offline = match config.buffer.nickname.shown_status
                {
                    ShownStatus::Current => current_user.is_none(),
                    ShownStatus::Historical => false,
                };

                let registry = clients.get_registry(server);

                let user_display = UserDisplay::new(
                    user,
                    config.buffer.nickname.show_access_levels,
                    config.buffer.nickname.show_bot_icon,
                    false,
                    registry,
                    &config.display.nickname,
                    config.buffer.nickname.truncate,
                    config.display.truncation_character,
                    Some(&config.buffer.nickname.brackets),
                    true,
                );

                let nick_text = user_display.into_element(
                    user,
                    is_user_away,
                    is_user_offline,
                    None,
                    None,
                    false,
                    true,
                    theme,
                    config,
                );

                let chantypes = clients.get_server_chantypes_or_default(server);
                let casemapping =
                    clients.get_server_casemapping_or_default(server);
                let prefix = clients.get_server_prefix_or_default(server);

                let nick = context_menu::user(
                    nick_text,
                    server,
                    prefix,
                    Some(channel),
                    clients.get_registry(server),
                    previews,
                    user,
                    current_user,
                    None,
                    config,
                    theme,
                    &config.actions.buffer.click_username,
                )
                .map(scroll_view::Message::ContextMenu);

                let text = message_content::with_context(
                    &message.content,
                    &[],
                    server,
                    registry,
                    chantypes,
                    casemapping,
                    theme,
                    scroll_view::Message::Link,
                    None,
                    theme::selectable_text::default,
                    theme::font_style::primary,
                    Option::<fn(Color) -> Color>::None,
                    move |link| {
                        context_menu::Entry::link_list(
                            link,
                            Some(|user| {
                                context_menu::Entry::user_list(
                                    true,
                                    current_user,
                                    None,
                                    config.file_transfer.enabled,
                                    context_menu::has_user_metadata(
                                        user,
                                        clients.get_registry(server),
                                        config,
                                    ),
                                    true,
                                )
                            }),
                            Some(|_| {
                                context_menu::Entry::url_list(
                                    false, None, None, false, false, false,
                                )
                            }),
                            Some(|server, channel| {
                                context_menu::Entry::channel_list(
                                    channel_is_open(server, channel),
                                    channel_is_focused(server, channel),
                                )
                            }),
                        )
                    },
                    move |link, entry, length| {
                        let context = Context::link(
                            link,
                            Some(|user| UserContext {
                                server,
                                prefix,
                                channel: Some(channel),
                                registry: clients.get_registry(server),
                                avatar: context_menu::user_avatar(
                                    user,
                                    clients.get_registry(server),
                                    previews,
                                    config.metadata.avatar_size(),
                                ),
                                user,
                                current_user,
                            }),
                            Some(|url| UrlContext {
                                url,
                                message: None,
                                selected_reactions: vec![],
                            }),
                            Some(|server, channel| ChannelContext {
                                server,
                                channel,
                                is_open: channel_is_open(server, channel),
                            }),
                        );

                        entry
                            .view(context, length, config, theme)
                            .map(scroll_view::Message::ContextMenu)
                    },
                    None,
                    config,
                );

                Some(
                    container(row![
                        timestamp,
                        selectable_text(" "),
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
            }
            | message::Target::ChannelMonitor {
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
                                config
                                    .actions
                                    .buffer
                                    .click_highlight
                                    .buffer_action(),
                            )),
                        span(" "),
                    ])
                    .on_link(scroll_view::Message::Link);

                let chantypes = clients.get_server_chantypes_or_default(server);
                let casemapping =
                    clients.get_server_casemapping_or_default(server);

                let text = message_content(
                    &message.content,
                    &[],
                    server,
                    clients.get_registry(server),
                    chantypes,
                    casemapping,
                    theme,
                    scroll_view::Message::Link,
                    None,
                    theme::selectable_text::action,
                    theme::font_style::action,
                    Option::<fn(Color) -> Color>::None,
                    None,
                    config,
                );

                Some(
                    container(row![
                        timestamp,
                        selectable_text(" "),
                        channel_text,
                        text
                    ])
                    .into(),
                )
            }
            _ => None,
        },
        metadata::EMPTY,
        channel_is_focused,
        channel_is_open,
    )
    .map(Message::ScrollView);

    container(messages)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(8)
        .into()
}

#[derive(Debug, Clone)]
pub struct MessageFeed {
    pub kind: Kind,
    pub scroll_view: scroll_view::State,
}

impl MessageFeed {
    pub fn new(kind: Kind, pane_size: Size, config: &Config) -> Self {
        Self {
            kind,
            scroll_view: scroll_view::State::new(pane_size, config),
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        history: &mut history::Manager,
        clients: &mut data::client::Map,
        previews: &preview::Collection,
        config: &Config,
    ) -> (Task<Message>, Option<Event>) {
        match message {
            Message::ScrollView(message) => {
                let (command, event) = self.scroll_view.update(
                    message,
                    false,
                    self.kind.scroll_view(),
                    None,
                    history,
                    clients,
                    previews,
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
                    ) => Some(Event::OpenBuffer(server, target, buffer_action)),
                    scroll_view::Event::GoToMessage(
                        server,
                        channel,
                        message,
                        action,
                    ) => Some(Event::GoToMessage(
                        server, channel, message, action,
                    )),
                    scroll_view::Event::RequestOlderChatHistory => None,
                    scroll_view::Event::PreviewChanged => None,
                    scroll_view::Event::HidePreview(..) => None,
                    scroll_view::Event::MarkAsRead => Some(Event::MarkAsRead),
                    scroll_view::Event::OpenUrl(url) => {
                        Some(Event::OpenUrl(url))
                    }
                    scroll_view::Event::ImagePreview(image) => {
                        Some(Event::ImagePreview(image))
                    }
                    scroll_view::Event::ExpandMessage(server_time, hash) => {
                        Some(Event::ExpandMessage(server_time, hash))
                    }
                    scroll_view::Event::ContractMessage(server_time, hash) => {
                        Some(Event::ContractMessage(server_time, hash))
                    }
                });

                (command.map(Message::ScrollView), event)
            }
        }
    }
}
