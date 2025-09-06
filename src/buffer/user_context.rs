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
    Whois,
    Query,
    ToggleAccessLevelOp,
    ToggleAccessLevelVoice,
    SendFile,
    UserInfo,
    HorizontalRule,
    CtcpRequestTime,
    CtcpRequestVersion,
}

impl Entry {
    pub fn list(is_channel: bool, our_user: Option<&User>) -> Vec<Self> {
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
        user: &User,
        current_user: Option<&User>,
        length: Length,
        config: &Config,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let nickname = user.nickname().to_owned();

        match self {
            Entry::Whois => menu_button(
                "Whois".to_string(),
                Message::Whois(server.clone(), nickname),
                length,
                theme,
            ),
            Entry::Query => menu_button(
                "Message".to_string(),
                Message::Query(
                    server.clone(),
                    target::Query::from(user),
                    config.actions.buffer.message_user,
                ),
                length,
                theme,
            ),
            Entry::ToggleAccessLevelOp => {
                if let (Some(channel), Some(operator_mode)) = (
                    channel,
                    prefix.iter().find_map(|prefix_map| {
                        (prefix_map.prefix == '@').then_some(prefix_map.mode)
                    }),
                ) {
                    if user.has_access_level(data::user::AccessLevel::Oper) {
                        menu_button(
                            format!("Take Op (-{operator_mode})"),
                            Message::ToggleAccessLevel(
                                server.clone(),
                                channel.clone(),
                                nickname,
                                format!("-{operator_mode}"),
                            ),
                            length,
                            theme,
                        )
                    } else {
                        menu_button(
                            format!("Give Op (+{operator_mode})"),
                            Message::ToggleAccessLevel(
                                server.clone(),
                                channel.clone(),
                                nickname,
                                format!("+{operator_mode}"),
                            ),
                            length,
                            theme,
                        )
                    }
                } else {
                    row![].into()
                }
            }
            Entry::ToggleAccessLevelVoice => {
                if let (Some(channel), Some(voice_mode)) = (
                    channel,
                    prefix.iter().find_map(|prefix_map| {
                        (prefix_map.prefix == '+').then_some(prefix_map.mode)
                    }),
                ) {
                    if user.has_access_level(data::user::AccessLevel::Voice) {
                        menu_button(
                            format!("Take Voice (-{voice_mode})"),
                            Message::ToggleAccessLevel(
                                server.clone(),
                                channel.clone(),
                                nickname,
                                format!("-{voice_mode}"),
                            ),
                            length,
                            theme,
                        )
                    } else {
                        menu_button(
                            format!("Give Voice (+{voice_mode})"),
                            Message::ToggleAccessLevel(
                                server.clone(),
                                channel.clone(),
                                nickname,
                                format!("+{voice_mode}"),
                            ),
                            length,
                            theme,
                        )
                    }
                } else {
                    row![].into()
                }
            }
            Entry::SendFile => menu_button(
                "Send File".to_string(),
                Message::SendFile(server.clone(), user.clone()),
                length,
                theme,
            ),
            Entry::UserInfo => {
                user_info(current_user, nickname, length, config, theme)
            }
            Entry::HorizontalRule => match length {
                Length::Fill => {
                    container(horizontal_rule(1)).padding([0, 6]).into()
                }
                _ => Space::new(length, 1).into(),
            },
            Entry::CtcpRequestTime => menu_button(
                "Local Time (TIME)".to_string(),
                Message::CtcpRequest(
                    ctcp::Command::Time,
                    server.clone(),
                    nickname,
                    None,
                ),
                length,
                theme,
            ),
            Entry::CtcpRequestVersion => menu_button(
                "Client (VERSION)".to_string(),
                Message::CtcpRequest(
                    ctcp::Command::Version,
                    server.clone(),
                    nickname,
                    None,
                ),
                length,
                theme,
            ),
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
}

#[derive(Debug, Clone)]
pub enum Event {
    SendWhois(Server, Nick),
    OpenQuery(Server, target::Query, BufferAction),
    ToggleAccessLevel(Server, target::Channel, Nick, String),
    SendFile(Server, User),
    InsertNickname(Nick),
    CtcpRequest(ctcp::Command, Server, Nick, Option<String>),
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
    }
}

pub fn view<'a>(
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
    let entries = Entry::list(channel.is_some(), our_user);

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
                user,
                current_user,
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
    message: Message,
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
    .on_press(message)
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
        data::buffer::Color::Unique => Some(nickname.to_string()),
    };

    let style = theme::text::nickname(
        theme,
        seed.clone(),
        is_user_away,
        is_user_offline,
    );

    let nickname = text(nickname.to_string()).style(move |_| style).font_maybe(
        theme::font_style::nickname(theme, is_user_offline).map(font::get),
    );

    column![
        container(row![nickname, state].width(length).spacing(4))
            .padding(right_justified_padding())
    ]
    .into()
}
