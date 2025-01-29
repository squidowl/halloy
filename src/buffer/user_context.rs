use data::user::Nick;
use data::{isupport, target, Config, Server, User};
use iced::widget::{button, column, container, horizontal_rule, row, text, Space};
use iced::{padding, Length, Padding};

use crate::theme;
use crate::widget::{context_menu, double_pass, Element};

#[derive(Debug, Clone, Copy)]
pub enum Entry {
    Whois,
    Query,
    ToggleAccessLevelOp,
    ToggleAccessLevelVoice,
    SendFile,
    UserInfo,
    HorizontalRule,
}

impl Entry {
    pub fn list(is_channel: bool, our_user: Option<&User>) -> Vec<Self> {
        if is_channel {
            if our_user.is_some_and(|u| u.has_access_level(data::user::AccessLevel::Oper)) {
                vec![
                    Entry::UserInfo,
                    Entry::HorizontalRule,
                    Entry::Whois,
                    Entry::Query,
                    Entry::ToggleAccessLevelOp,
                    Entry::ToggleAccessLevelVoice,
                    Entry::SendFile,
                ]
            } else {
                vec![
                    Entry::UserInfo,
                    Entry::HorizontalRule,
                    Entry::Whois,
                    Entry::Query,
                    Entry::SendFile,
                ]
            }
        } else {
            vec![Entry::Whois, Entry::SendFile]
        }
    }

    pub fn view<'a>(
        self,
        server: &Server,
        casemapping: isupport::CaseMap,
        channel: Option<&target::Channel>,
        user: &User,
        current_user: Option<&User>,
        length: Length,
        config: &Config,
    ) -> Element<'a, Message> {
        let nickname = user.nickname().to_owned();

        match self {
            Entry::Whois => menu_button("Whois", Message::Whois(server.clone(), nickname), length),
            Entry::Query => menu_button(
                "Message",
                Message::Query(server.clone(), target::Query::from_user(user, casemapping)),
                length,
            ),
            Entry::ToggleAccessLevelOp => {
                if let Some(channel) = channel {
                    if user.has_access_level(data::user::AccessLevel::Oper) {
                        menu_button(
                            "Take Op (-o)",
                            Message::ToggleAccessLevel(
                                server.clone(),
                                channel.clone(),
                                nickname,
                                "-o".to_owned(),
                            ),
                            length,
                        )
                    } else {
                        menu_button(
                            "Give Op (+o)",
                            Message::ToggleAccessLevel(
                                server.clone(),
                                channel.clone(),
                                nickname,
                                "+o".to_owned(),
                            ),
                            length,
                        )
                    }
                } else {
                    row![].into()
                }
            }
            Entry::ToggleAccessLevelVoice => {
                if let Some(channel) = channel {
                    if user.has_access_level(data::user::AccessLevel::Voice) {
                        menu_button(
                            "Take Voice (-v)",
                            Message::ToggleAccessLevel(
                                server.clone(),
                                channel.clone(),
                                nickname,
                                "-v".to_owned(),
                            ),
                            length,
                        )
                    } else {
                        menu_button(
                            "Give Voice (+v)",
                            Message::ToggleAccessLevel(
                                server.clone(),
                                channel.clone(),
                                nickname,
                                "+v".to_owned(),
                            ),
                            length,
                        )
                    }
                } else {
                    row![].into()
                }
            }
            Entry::SendFile => menu_button(
                "Send File",
                Message::SendFile(server.clone(), nickname),
                length,
            ),
            Entry::UserInfo => user_info(current_user, nickname, length, config),
            Entry::HorizontalRule => match length {
                Length::Fill => container(horizontal_rule(1)).padding([0, 6]).into(),
                _ => Space::new(length, 1).into(),
            },
        }
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    Whois(Server, Nick),
    Query(Server, target::Query),
    ToggleAccessLevel(Server, target::Channel, Nick, String),
    SendFile(Server, Nick),
    SingleClick(Nick),
}

#[derive(Debug, Clone)]
pub enum Event {
    SendWhois(Server, Nick),
    OpenQuery(Server, target::Query),
    ToggleAccessLevel(Server, target::Channel, Nick, String),
    SendFile(Server, Nick),
    SingleClick(Nick),
}

pub fn update(message: Message) -> Option<Event> {
    match message {
        Message::Whois(server, nick) => Some(Event::SendWhois(server, nick)),
        Message::Query(server, nick) => Some(Event::OpenQuery(server, nick)),
        Message::ToggleAccessLevel(server, target, nick, mode) => {
            Some(Event::ToggleAccessLevel(server, target, nick, mode))
        }
        Message::SendFile(server, nick) => Some(Event::SendFile(server, nick)),
        Message::SingleClick(nick) => Some(Event::SingleClick(nick)),
    }
}

pub fn view<'a>(
    content: impl Into<Element<'a, Message>>,
    server: &'a Server,
    casemapping: isupport::CaseMap,
    channel: Option<&'a target::Channel>,
    user: &'a User,
    current_user: Option<&'a User>,
    our_user: Option<&'a User>,
    config: &'a Config,
) -> Element<'a, Message> {
    let entries = Entry::list(channel.is_some(), our_user);

    let content = button(content)
        .padding(0)
        .style(theme::button::bare)
        .on_press(Message::SingleClick(user.nickname().to_owned()));

    context_menu(
        Default::default(),
        content,
        entries,
        move |entry, length| {
            entry.view(
                server,
                casemapping,
                channel,
                user,
                current_user,
                length,
                config,
            )
        },
    )
    .into()
}

fn menu_button(content: &str, message: Message, length: Length) -> Element<'_, Message> {
    button(text(content).style(theme::text::primary))
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
) -> Element<'a, Message> {
    let state = match current_user {
        Some(user) => {
            if user.is_away() {
                Some(text("Away").style(theme::text::secondary).width(length))
            } else {
                None
            }
        }
        None => Some(text("Offline").style(theme::text::secondary).width(length)),
    };

    // Dimmed if away or offline.
    let user_is_dimmed = current_user.map(|u| u.is_away()).unwrap_or(true);
    let seed = match config.buffer.nickname.color {
        data::buffer::Color::Solid => None,
        data::buffer::Color::Unique => Some(nickname.to_string()),
    };

    column![container(
        text(nickname.to_string())
            .style(move |theme| theme::text::nickname(theme, seed.clone(), user_is_dimmed))
            .width(length)
    )
    .padding(right_justified_padding()),]
    .push_maybe(state.map(|s| container(s).padding(right_justified_padding())))
    .into()
}
