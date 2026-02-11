use chrono::{DateTime, Local, Utc};
use data::dashboard::BufferAction;
use data::user::Nick;
use data::{Config, Server, User, config, ctcp, isupport, message, target};
use iced::widget::{Space, button, column, container, row, rule};
use iced::{Length, Padding};

use crate::widget::{Element, context_menu, double_pass, text};
use crate::{Theme, font, theme, widget};

pub enum Context<'a> {
    User {
        server: &'a Server,
        prefix: &'a [isupport::PrefixMap],
        channel: Option<&'a target::Channel>,
        user: &'a User,
        current_user: Option<&'a User>,
    },
    Url(&'a String),
    Timestamp(&'a DateTime<Utc>),
    NotSentMessage(&'a DateTime<Utc>, &'a message::Hash),
}

#[derive(Debug, Clone, Copy)]
pub enum Entry {
    // user context
    Whois,
    Whowas,
    Query,
    ToggleAccessLevelOp,
    ToggleAccessLevelVoice,
    SendFile,
    UserInfo,
    HorizontalRule,
    CtcpRequestTime,
    CtcpRequestVersion,
    // url context
    CopyUrl,
    OpenUrl,
    // timestamp context
    Timestamp,
    // not sent message context
    DeleteMessage,
    ResendMessage,
}

impl Entry {
    pub fn not_sent_message_list(can_resend: bool) -> Vec<Self> {
        if can_resend {
            vec![Entry::DeleteMessage, Entry::ResendMessage]
        } else {
            vec![Entry::DeleteMessage]
        }
    }

    pub fn timestamp_list() -> Vec<Self> {
        vec![Entry::Timestamp]
    }

    pub fn url_list() -> Vec<Self> {
        vec![Entry::CopyUrl, Entry::OpenUrl]
    }

    pub fn user_list(
        is_channel: bool,
        user_in_channel: Option<&User>,
        our_user: Option<&User>,
        file_transfer_enabled: bool,
    ) -> Vec<Self> {
        let user_is_online = user_in_channel.is_some();

        if is_channel {
            if !user_is_online {
                vec![Entry::UserInfo, Entry::HorizontalRule, Entry::Whowas]
            } else if our_user.is_some_and(|u| {
                u.has_access_level(data::user::AccessLevel::Oper)
            }) {
                let mut list = vec![
                    Entry::UserInfo,
                    Entry::HorizontalRule,
                    Entry::Whois,
                    Entry::Query,
                ];

                if file_transfer_enabled {
                    list.push(Entry::SendFile);
                }

                list.extend(vec![
                    Entry::HorizontalRule,
                    Entry::ToggleAccessLevelOp,
                    Entry::ToggleAccessLevelVoice,
                    Entry::HorizontalRule,
                    Entry::CtcpRequestVersion,
                    Entry::CtcpRequestTime,
                ]);

                list
            } else {
                let mut list = vec![
                    Entry::UserInfo,
                    Entry::HorizontalRule,
                    Entry::Whois,
                    Entry::Query,
                ];

                if file_transfer_enabled {
                    list.push(Entry::SendFile);
                }

                list.extend(vec![
                    Entry::HorizontalRule,
                    Entry::CtcpRequestVersion,
                    Entry::CtcpRequestTime,
                ]);

                list
            }
        } else {
            let mut list = vec![if user_is_online {
                Entry::Whois
            } else {
                Entry::Whowas
            }];

            if file_transfer_enabled {
                list.push(Entry::SendFile);
            }

            list
        }
    }

    pub fn view<'a>(
        self,
        context: Option<Context<'_>>,
        length: Length,
        config: &Config,
        theme: &Theme,
    ) -> Element<'a, Message> {
        context.map_or(row![].into(), |context| match (self, context) {
            (Entry::Whois, Context::User { server, user, .. }) => {
                let message =
                    Message::Whois(server.clone(), user.nickname().to_owned());

                menu_button(
                    "Whois".to_string(),
                    Some(message),
                    length,
                    theme,
                    config,
                )
            }
            (Entry::Whowas, Context::User { server, user, .. }) => {
                let message =
                    Message::Whowas(server.clone(), user.nickname().to_owned());

                menu_button(
                    "Whowas".to_string(),
                    Some(message),
                    length,
                    theme,
                    config,
                )
            }
            (Entry::Query, Context::User { server, user, .. }) => {
                let message = Message::Query(
                    server.clone(),
                    target::Query::from(user.clone()),
                    config.actions.buffer.message_user,
                );

                menu_button(
                    "Message".to_string(),
                    Some(message),
                    length,
                    theme,
                    config,
                )
            }
            (
                Entry::ToggleAccessLevelOp,
                Context::User {
                    server,
                    prefix,
                    channel,
                    user,
                    ..
                },
            ) => {
                let operator_mode = prefix.iter().find_map(|prefix_map| {
                    (prefix_map.prefix == '@').then_some(prefix_map.mode)
                });

                let (label, message) =
                    if let (Some(channel), Some(operator_mode)) =
                        (channel, operator_mode)
                    {
                        let is_op = user
                            .has_access_level(data::user::AccessLevel::Oper);
                        let prefix = if is_op { "-" } else { "+" };
                        let action = format!("{prefix}{operator_mode}");

                        (
                            format!(
                                "{} Op ({action})",
                                if is_op { "Take" } else { "Give" }
                            ),
                            Some(Message::ToggleAccessLevel(
                                server.clone(),
                                channel.clone(),
                                user.nickname().to_owned(),
                                action,
                            )),
                        )
                    } else {
                        (String::new(), None)
                    };

                menu_button(label, message, length, theme, config)
            }
            (
                Entry::ToggleAccessLevelVoice,
                Context::User {
                    server,
                    prefix,
                    channel,
                    user,
                    ..
                },
            ) => {
                let voice_mode = prefix.iter().find_map(|prefix_map| {
                    (prefix_map.prefix == '+').then_some(prefix_map.mode)
                });

                let (label, message) =
                    if let (Some(channel), Some(voice_mode)) =
                        (channel, voice_mode)
                    {
                        let has_voice = user
                            .has_access_level(data::user::AccessLevel::Voice);
                        let prefix = if has_voice { "-" } else { "+" };
                        let action = format!("{prefix}{voice_mode}");

                        (
                            format!(
                                "{} Voice ({action})",
                                if has_voice { "Take" } else { "Give" }
                            ),
                            Some(Message::ToggleAccessLevel(
                                server.clone(),
                                channel.clone(),
                                user.nickname().to_owned(),
                                action,
                            )),
                        )
                    } else {
                        (String::new(), None)
                    };

                menu_button(label, message, length, theme, config)
            }
            (Entry::SendFile, Context::User { server, user, .. }) => {
                let message = Message::SendFile(server.clone(), user.clone());

                menu_button(
                    "Send File".to_string(),
                    Some(message),
                    length,
                    theme,
                    config,
                )
            }
            (
                Entry::UserInfo,
                Context::User {
                    user, current_user, ..
                },
            ) => user_info(
                current_user,
                user.nickname().to_owned(),
                length,
                config,
                theme,
            ),
            (Entry::HorizontalRule, _) => match length {
                Length::Fill => {
                    container(rule::horizontal(1)).padding([0, 6]).into()
                }
                _ => Space::new().width(length).height(1).into(),
            },
            (Entry::CtcpRequestTime, Context::User { server, user, .. }) => {
                let message = Message::CtcpRequest(
                    ctcp::Command::Time,
                    server.clone(),
                    user.nickname().to_owned(),
                    None,
                );

                menu_button(
                    "Local Time (TIME)".to_string(),
                    Some(message),
                    length,
                    theme,
                    config,
                )
            }
            (Entry::CtcpRequestVersion, Context::User { server, user, .. }) => {
                let message = Message::CtcpRequest(
                    ctcp::Command::Version,
                    server.clone(),
                    user.nickname().to_owned(),
                    None,
                );

                menu_button(
                    "Client (VERSION)".to_string(),
                    Some(message),
                    length,
                    theme,
                    config,
                )
            }
            (Entry::CopyUrl, Context::Url(url)) => {
                let message = Message::CopyUrl(url.clone());

                menu_button(
                    "Copy URL".to_string(),
                    Some(message),
                    length,
                    theme,
                    config,
                )
            }
            (Entry::OpenUrl, Context::Url(url)) => {
                let message = Message::OpenUrl(url.clone());

                menu_button(
                    "Open URL".to_string(),
                    Some(message),
                    length,
                    theme,
                    config,
                )
            }
            (Entry::Timestamp, Context::Timestamp(date_time)) => {
                let message = Message::CopyTimestamp(
                    *date_time,
                    config.buffer.timestamp.copy_format.clone(),
                );

                menu_button(
                    format!(
                        "{}",
                        date_time.with_timezone(&Local).format(
                            &config.buffer.timestamp.context_menu_format
                        )
                    ),
                    Some(message),
                    length,
                    theme,
                    config,
                )
            }
            (
                Entry::DeleteMessage,
                Context::NotSentMessage(server_time, hash),
            ) => {
                let message = Message::DeleteMessage(*server_time, *hash);

                menu_button(
                    "Delete Message".to_string(),
                    Some(message),
                    length,
                    theme,
                    config,
                )
            }
            (
                Entry::ResendMessage,
                Context::NotSentMessage(server_time, hash),
            ) => {
                let message = Message::ResendMessage(*server_time, *hash);

                menu_button(
                    "Re-send Message".to_string(),
                    Some(message),
                    length,
                    theme,
                    config,
                )
            }
            _ => row![].into(),
        })
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    Whois(Server, Nick),
    Whowas(Server, Nick),
    Query(Server, target::Query, BufferAction),
    ToggleAccessLevel(Server, target::Channel, Nick, String),
    SendFile(Server, User),
    InsertNickname(Nick),
    CtcpRequest(ctcp::Command, Server, Nick, Option<String>),
    CopyUrl(String),
    OpenUrl(String),
    CopyTimestamp(DateTime<Utc>, Option<String>),
    #[allow(clippy::enum_variant_names)]
    DeleteMessage(DateTime<Utc>, message::Hash),
    #[allow(clippy::enum_variant_names)]
    ResendMessage(DateTime<Utc>, message::Hash),
}

#[derive(Debug, Clone)]
pub enum Event {
    SendWhois(Server, Nick),
    SendWhowas(Server, Nick),
    OpenQuery(Server, target::Query, BufferAction),
    ToggleAccessLevel(Server, target::Channel, Nick, String),
    SendFile(Server, User),
    InsertNickname(Nick),
    CtcpRequest(ctcp::Command, Server, Nick, Option<String>),
    CopyUrl(String),
    OpenUrl(String),
    CopyTimestamp(DateTime<Utc>, Option<String>),
    DeleteMessage(DateTime<Utc>, message::Hash),
    ResendMessage(DateTime<Utc>, message::Hash),
}

pub fn update(message: Message) -> Event {
    match message {
        Message::Whois(server, nick) => Event::SendWhois(server, nick),
        Message::Whowas(server, nick) => Event::SendWhowas(server, nick),
        Message::Query(server, nick, buffer_action) => {
            Event::OpenQuery(server, nick, buffer_action)
        }
        Message::ToggleAccessLevel(server, target, nick, mode) => {
            Event::ToggleAccessLevel(server, target, nick, mode)
        }
        Message::SendFile(server, user) => Event::SendFile(server, user),
        Message::InsertNickname(nick) => Event::InsertNickname(nick),
        Message::CtcpRequest(command, server, nick, params) => {
            Event::CtcpRequest(command, server, nick, params)
        }
        Message::CopyUrl(url) => Event::CopyUrl(url),
        Message::OpenUrl(url) => Event::OpenUrl(url),
        Message::CopyTimestamp(date_time, format) => {
            Event::CopyTimestamp(date_time, format)
        }
        Message::DeleteMessage(sesrver_time, hash) => {
            Event::DeleteMessage(sesrver_time, hash)
        }
        Message::ResendMessage(sesrver_time, hash) => {
            Event::ResendMessage(sesrver_time, hash)
        }
    }
}

pub fn user<'a>(
    content: impl Into<Element<'a, Message>>,
    server: &'a Server,
    prefix: &'a [isupport::PrefixMap],
    channel: Option<&'a target::Channel>,
    user: &'a User,
    current_user: Option<&'a User>,
    our_user: Option<&'a User>,
    config: &'a Config,
    theme: &'a Theme,
    click: &'a config::buffer::NicknameClickAction,
) -> Element<'a, Message> {
    let entries = Entry::user_list(
        channel.is_some(),
        current_user,
        our_user,
        config.file_transfer.enabled,
    );

    let message = match click {
        data::config::buffer::NicknameClickAction::OpenQuery => Message::Query(
            server.clone(),
            target::Query::from(user),
            config.actions.buffer.click_username,
        ),
        data::config::buffer::NicknameClickAction::InsertNickname => {
            Message::InsertNickname(user.nickname().to_owned())
        }
    };

    let base = widget::button::transparent_button(content, message);

    context_menu(
        context_menu::MouseButton::default(),
        context_menu::Anchor::Cursor,
        context_menu::ToggleBehavior::KeepOpen,
        base,
        entries,
        move |entry, length| {
            entry.view(
                Some(Context::User {
                    server,
                    prefix,
                    channel,
                    user,
                    current_user,
                }),
                length,
                config,
                theme,
            )
        },
    )
    .into()
}

pub fn timestamp<'a>(
    content: impl Into<Element<'a, Message>>,
    date_time: &'a DateTime<Utc>,
    config: &'a Config,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let entries = Entry::timestamp_list();

    context_menu(
        context_menu::MouseButton::default(),
        context_menu::Anchor::Cursor,
        context_menu::ToggleBehavior::KeepOpen,
        content,
        entries,
        move |entry, length| {
            entry.view(
                Some(Context::Timestamp(date_time)),
                length,
                config,
                theme,
            )
        },
    )
    .into()
}

pub fn not_sent_message<'a>(
    content: impl Into<Element<'a, Message>>,
    server_time: &'a DateTime<Utc>,
    hash: &'a message::Hash,
    can_resend: bool,
    config: &'a Config,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let entries = Entry::not_sent_message_list(can_resend);

    context_menu(
        context_menu::MouseButton::Left,
        context_menu::Anchor::Cursor,
        context_menu::ToggleBehavior::KeepOpen,
        content,
        entries,
        move |entry, length| {
            entry.view(
                Some(Context::NotSentMessage(server_time, hash)),
                length,
                config,
                theme,
            )
        },
    )
    .into()
}

fn menu_button(
    content: String,
    message: Option<Message>,
    length: Length,
    theme: &Theme,
    config: &Config,
) -> Element<'static, Message> {
    button(
        text(content)
            .style(theme::text::primary)
            .font_maybe(theme::font_style::primary(theme).map(font::get)),
    )
    .padding(config.context_menu.padding.entry)
    .width(length)
    .on_press_maybe(message)
    .into()
}

fn right_justified_padding(config: &Config) -> Padding {
    let padding = config.context_menu.padding.entry;
    Padding::from(padding)
        .right(f32::from(padding[1]) + double_pass::horizontal_expansion())
}

fn user_info<'a>(
    current_user: Option<&User>,
    nickname: Nick,
    length: Length,
    config: &Config,
    theme: &Theme,
) -> Element<'a, Message> {
    let state = match current_user {
        Some(user) => {
            if user.is_away() {
                Some(
                    text("(Away)")
                        .style(theme::text::secondary)
                        .font_maybe(
                            theme::font_style::secondary(theme).map(font::get),
                        )
                        .width(length),
                )
            } else {
                None
            }
        }
        None => Some(
            text("(Offline)")
                .style(theme::text::secondary)
                .font_maybe(theme::font_style::secondary(theme).map(font::get))
                .width(length),
        ),
    };

    // Dimmed if away or offline.
    let is_user_away = config
        .buffer
        .nickname
        .away
        .is_away(current_user.is_none_or(User::is_away));
    let is_user_offline = config
        .buffer
        .nickname
        .offline
        .is_offline(current_user.is_none());
    let seed = match config.buffer.nickname.color {
        data::buffer::Color::Solid => None,
        data::buffer::Color::Unique => Some(nickname.seed()),
    };

    let style =
        theme::text::nickname(theme, seed, is_user_away, is_user_offline);

    let nickname = text(nickname.to_string()).style(move |_| style).font_maybe(
        theme::font_style::nickname(theme, is_user_offline).map(font::get),
    );

    column![
        container(row![nickname, state].width(length).spacing(4))
            .padding(right_justified_padding(config))
    ]
    .into()
}
