use data::user::Nick;
use data::{Buffer, User};
use iced::widget::{button, column, container, horizontal_rule, row, text};
use iced::Length;

use crate::theme;
use crate::widget::{context_menu, Element};

#[derive(Debug, Clone, Copy)]
enum Entry {
    Whois,
    Query,
    ToggleAccessLevelOp,
    ToggleAccessLevelVoice,
    SendFile,
    UserInfo,
}

impl Entry {
    fn list(buffer: &Buffer, our_user: Option<&User>) -> Vec<Self> {
        match buffer {
            Buffer::Channel(_, _) => {
                if our_user.is_some_and(|u| u.has_access_level(data::user::AccessLevel::Oper)) {
                    vec![
                        Entry::UserInfo,
                        Entry::Whois,
                        Entry::Query,
                        Entry::ToggleAccessLevelOp,
                        Entry::ToggleAccessLevelVoice,
                        Entry::SendFile,
                    ]
                } else {
                    vec![Entry::UserInfo, Entry::Whois, Entry::Query, Entry::SendFile]
                }
            }
            Buffer::Server(_) | Buffer::Query(_, _) => vec![Entry::Whois, Entry::SendFile],
        }
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    Whois(Nick),
    Query(Nick),
    SingleClick(Nick),
    ToggleAccessLevel(Nick, String),
    SendFile(Nick),
}

#[derive(Debug, Clone)]
pub enum Event {
    SendWhois(Nick),
    OpenQuery(Nick),
    SingleClick(Nick),
    ToggleAccessLevel(Nick, String),
    SendFile(Nick),
}

pub fn update(message: Message) -> Event {
    match message {
        Message::Whois(nick) => Event::SendWhois(nick),
        Message::Query(nick) => Event::OpenQuery(nick),
        Message::SingleClick(nick) => Event::SingleClick(nick),
        Message::ToggleAccessLevel(nick, mode) => Event::ToggleAccessLevel(nick, mode),
        Message::SendFile(nick) => Event::SendFile(nick),
    }
}

pub fn view<'a>(
    content: impl Into<Element<'a, Message>>,
    user: &'a User,
    current_user: Option<&'a User>,
    buffer: Buffer,
    our_user: Option<&'a User>,
) -> Element<'a, Message> {
    let entries = Entry::list(&buffer, our_user);

    let content = button(content)
        .padding(0)
        .style(theme::button::bare)
        .on_press(Message::SingleClick(user.nickname().to_owned()));

    context_menu(content, entries, move |entry, length| {
        let nickname = user.nickname().to_owned();

        let (content, message) = match entry {
            Entry::Whois => (button_text("Whois"), Some(Message::Whois(nickname))),
            Entry::Query => (button_text("Message"), Some(Message::Query(nickname))),
            Entry::ToggleAccessLevelOp => {
                if user.has_access_level(data::user::AccessLevel::Oper) {
                    (
                        button_text("Take Op (-o)"),
                        Some(Message::ToggleAccessLevel(nickname, "-o".to_owned())),
                    )
                } else {
                    (
                        button_text("Give Op (+o)"),
                        Some(Message::ToggleAccessLevel(nickname, "+o".to_owned())),
                    )
                }
            }
            Entry::ToggleAccessLevelVoice => {
                if user.has_access_level(data::user::AccessLevel::Voice) {
                    (
                        button_text("Take Voice (-v)"),
                        Some(Message::ToggleAccessLevel(nickname, "-v".to_owned())),
                    )
                } else {
                    (
                        button_text("Give Voice (+v)"),
                        Some(Message::ToggleAccessLevel(nickname, "+v".to_owned())),
                    )
                }
            }
            Entry::SendFile => (button_text("Send File"), Some(Message::SendFile(nickname))),
            Entry::UserInfo => (user_info(current_user, length), None),
        };

        if let Some(message) = message {
            button(content)
                .padding(5)
                .width(length)
                .style(theme::button::context)
                .on_press(message)
                .into()
        } else {
            column![]
                .push(container(content).padding(5).width(length))
                .push(
                    row![]
                        .push(horizontal_rule(1))
                        .padding([0, 5])
                        .width(length),
                )
                .into()
        }
    })
}

fn button_text(content: &str) -> Element<'_, Message> {
    text(content).style(theme::text::primary).into()
}

fn symbol_padding() -> [u16; 4] {
    [0, 1, 0, 0]
}

fn user_info(current_user: Option<&User>, length: Length) -> Element<'_, Message> {
    if let Some(current_user) = current_user {
        let user_hostname = current_user.hostname().map(|hostname| {
            row![].push(text(hostname).style(theme::text::transparent).width(length))
        });

        let user_status = if current_user.is_away() {
            row![]
                .push(text("Away").style(theme::text::transparent).width(length))
                .push(
                    text("⬤")
                        .style(theme::text::info)
                        .shaping(text::Shaping::Advanced),
                )
                .padding(symbol_padding())
        } else {
            row![]
                .push(text("Online").style(theme::text::transparent).width(length))
                .push(
                    text("⬤")
                        .style(theme::text::success)
                        .shaping(text::Shaping::Advanced),
                )
                .padding(symbol_padding())
        };

        let user_access_levels = column![]
            .push_maybe(
                current_user
                    .has_access_level(data::user::AccessLevel::Owner)
                    .then(move || {
                        row![]
                            .push(text("Owner").style(theme::text::transparent).width(length))
                            .push(text("~").style(theme::text::transparent))
                            .padding(symbol_padding())
                    }),
            )
            .push_maybe(
                current_user
                    .has_access_level(data::user::AccessLevel::Admin)
                    .then(move || {
                        row![]
                            .push(
                                text("Administrator")
                                    .style(theme::text::transparent)
                                    .width(length),
                            )
                            .push(text("&").style(theme::text::transparent))
                            .padding(symbol_padding())
                    }),
            )
            .push_maybe(
                current_user
                    .has_access_level(data::user::AccessLevel::Oper)
                    .then(move || {
                        row![]
                            .push(
                                text("Operator")
                                    .style(theme::text::transparent)
                                    .width(length),
                            )
                            .push(text("@").style(theme::text::transparent))
                            .padding(symbol_padding())
                    }),
            )
            .push_maybe(
                current_user
                    .has_access_level(data::user::AccessLevel::HalfOp)
                    .then(move || {
                        row![]
                            .push(
                                text("Half-Operator")
                                    .style(theme::text::transparent)
                                    .width(length),
                            )
                            .push(text("%").style(theme::text::transparent))
                            .padding(symbol_padding())
                    }),
            )
            .push_maybe(
                current_user
                    .has_access_level(data::user::AccessLevel::Voice)
                    .then(move || {
                        row![]
                            .push(text("Voiced").style(theme::text::transparent).width(length))
                            .push(text("+").style(theme::text::transparent))
                            .padding(symbol_padding())
                    }),
            );

        column![]
            .push_maybe(user_hostname)
            .push(user_access_levels)
            .push(user_status)
            .into()
    } else {
        row![]
            .push(
                text("Offline")
                    .style(theme::text::transparent)
                    .width(length),
            )
            .push(
                text("⬤")
                    .style(theme::text::error)
                    .shaping(text::Shaping::Advanced),
            )
            .padding(symbol_padding())
            .into()
    }
}
