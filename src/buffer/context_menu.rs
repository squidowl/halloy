use data::dashboard::BufferAction;
use data::user::Nick;
use data::{Config, Server, User, config, ctcp, isupport, target};
use iced::widget::{
    Space, button, column, container, horizontal_rule, row, text,
};
use iced::{Length, Padding, padding};

use crate::widget::{Element, context_menu, double_pass};
use crate::{Theme, font, theme, widget};

#[derive(Debug, Clone, Copy)]
pub enum Entry {
    // user context
    Whois,
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
}

impl Entry {
    pub fn url_list() -> Vec<Self> {
        vec![Entry::CopyUrl]
    }

    pub fn user_list(is_channel: bool, our_user: Option<&User>) -> Vec<Self> {
        if is_channel {
            if our_user.is_some_and(|u| {
                u.has_access_level(data::user::AccessLevel::Oper)
            }) {
                vec![
                    Entry::UserInfo,
                    Entry::HorizontalRule,
                    Entry::Whois,
                    Entry::Query,
                    Entry::SendFile,
                    Entry::HorizontalRule,
                    Entry::ToggleAccessLevelOp,
                    Entry::ToggleAccessLevelVoice,
                    Entry::HorizontalRule,
                    Entry::CtcpRequestVersion,
                    Entry::CtcpRequestTime,
                ]
            } else {
                vec![
                    Entry::UserInfo,
                    Entry::HorizontalRule,
                    Entry::Whois,
                    Entry::Query,
                    Entry::SendFile,
                    Entry::HorizontalRule,
                    Entry::CtcpRequestVersion,
                    Entry::CtcpRequestTime,
                ]
            }
        } else {
            vec![Entry::Whois, Entry::SendFile]
        }
    }

    pub fn view<'a>(
        self,
        server: &Server,
        prefix: &[isupport::PrefixMap],
        channel: Option<&target::Channel>,
        user: Option<&User>,
        current_user: Option<&User>,
        url: Option<&String>,
        length: Length,
        config: &Config,
        theme: &Theme,
    ) -> Element<'a, Message> {
        match self {
            Entry::Whois => {
                let message = user.map(|user| {
                    Message::Whois(server.clone(), user.nickname().to_owned())
                });

                menu_button("Whois".to_string(), message, length, theme)
            }
            Entry::Query => {
                let message = user.map(|user| {
                    Message::Query(
                        server.clone(),
                        target::Query::from(user.clone()),
                        config.actions.buffer.message_user,
                    )
                });

                menu_button("Message".to_string(), message, length, theme)
            }
            Entry::ToggleAccessLevelOp => {
                let (channel, operator_mode, user) = (
                    channel,
                    prefix.iter().find_map(|prefix_map| {
                        (prefix_map.prefix == '@').then_some(prefix_map.mode)
                    }),
                    user,
                );

                let (label, message) =
                    if let (Some(channel), Some(operator_mode), Some(user)) =
                        (channel, operator_mode, user)
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

                menu_button(label, message, length, theme)
            }
            Entry::ToggleAccessLevelVoice => {
                let (channel, voice_mode, user) = (
                    channel,
                    prefix.iter().find_map(|prefix_map| {
                        (prefix_map.prefix == '+').then_some(prefix_map.mode)
                    }),
                    user,
                );

                let (label, message) =
                    if let (Some(channel), Some(voice_mode), Some(user)) =
                        (channel, voice_mode, user)
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

                menu_button(label, message, length, theme)
            }
            Entry::SendFile => {
                let message = user.map(|user| {
                    Message::SendFile(server.clone(), user.clone())
                });

                menu_button("Send File".to_string(), message, length, theme)
            }
            Entry::UserInfo => {
                if let Some(user) = user {
                    user_info(
                        current_user,
                        user.nickname().to_owned(),
                        length,
                        config,
                        theme,
                    )
                } else {
                    row![].into()
                }
            }
            Entry::HorizontalRule => match length {
                Length::Fill => {
                    container(horizontal_rule(1)).padding([0, 6]).into()
                }
                _ => Space::new(length, 1).into(),
            },
            Entry::CtcpRequestTime => {
                let message = user.map(|user| {
                    Message::CtcpRequest(
                        ctcp::Command::Time,
                        server.clone(),
                        user.nickname().to_owned(),
                        None,
                    )
                });

                menu_button(
                    "Local Time (TIME)".to_string(),
                    message,
                    length,
                    theme,
                )
            }
            Entry::CtcpRequestVersion => {
                let message = user.map(|user| {
                    Message::CtcpRequest(
                        ctcp::Command::Version,
                        server.clone(),
                        user.nickname().to_owned(),
                        None,
                    )
                });
                menu_button(
                    "Client (VERSION)".to_string(),
                    message,
                    length,
                    theme,
                )
            }
            Entry::CopyUrl => {
                let message = url.map(|url| Message::CopyUrl(url.clone()));

                menu_button("Copy URL".to_string(), message, length, theme)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    Whois(Server, Nick),
    Query(Server, target::Query, BufferAction),
    ToggleAccessLevel(Server, target::Channel, Nick, String),
    SendFile(Server, User),
    InsertNickname(Nick),
    CtcpRequest(ctcp::Command, Server, Nick, Option<String>),
    CopyUrl(String),
}

#[derive(Debug, Clone)]
pub enum Event {
    SendWhois(Server, Nick),
    OpenQuery(Server, target::Query, BufferAction),
    ToggleAccessLevel(Server, target::Channel, Nick, String),
    SendFile(Server, User),
    InsertNickname(Nick),
    CtcpRequest(ctcp::Command, Server, Nick, Option<String>),
    CopyUrl(String),
}

pub fn update(message: Message) -> Event {
    match message {
        Message::Whois(server, nick) => Event::SendWhois(server, nick),
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
    let entries = Entry::user_list(channel.is_some(), our_user);

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
                server,
                prefix,
                channel,
                Some(user),
                current_user,
                None,
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
) -> Element<'static, Message> {
    button(
        text(content)
            .style(theme::text::primary)
            .font_maybe(theme::font_style::primary(theme).map(font::get)),
    )
    .padding(5)
    .width(length)
    .on_press_maybe(message)
    .into()
}

fn right_justified_padding() -> Padding {
    padding::all(5).right(5.0 + double_pass::horizontal_expansion())
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
            .padding(right_justified_padding())
    ]
    .into()
}
