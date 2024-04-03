use data::user::Nick;
use data::{Buffer, User};
use iced::widget::{button, column, container, row, text, tooltip};

use crate::theme;
use crate::widget::{context_menu, Element};

#[derive(Debug, Clone, Copy)]
enum Entry {
    Whois,
    Query,
    ToggleAccessLevelOp,
    ToggleAccessLevelVoice,
    SendFile,
}

impl Entry {
    fn list(buffer: &Buffer, our_user: Option<&User>) -> Vec<Self> {
        match buffer {
            Buffer::Channel(_, _) => {
                if our_user.is_some_and(|u| u.has_access_level(data::user::AccessLevel::Oper)) {
                    vec![
                        Entry::Whois,
                        Entry::Query,
                        Entry::ToggleAccessLevelOp,
                        Entry::ToggleAccessLevelVoice,
                        Entry::SendFile,
                    ]
                } else {
                    vec![Entry::Whois, Entry::Query, Entry::SendFile]
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
    current_user: Option<Option<&'a User>>,
    buffer: Buffer,
    our_user: Option<&'a User>,
) -> Element<'a, Message> {
    let entries = Entry::list(&buffer, our_user);

    let content = button(content)
        .padding(0)
        .style(theme::button::bare)
        .on_press(Message::SingleClick(user.nickname().to_owned()));

    let content = context_menu(content, entries, move |entry, length| {
        let nickname = user.nickname().to_owned();

        let (content, message) = match entry {
            Entry::Whois => ("Whois", Message::Whois(nickname)),
            Entry::Query => ("Message", Message::Query(nickname)),
            Entry::ToggleAccessLevelOp => {
                if user.has_access_level(data::user::AccessLevel::Oper) {
                    (
                        "Take Op (-o)",
                        Message::ToggleAccessLevel(nickname, "-o".to_owned()),
                    )
                } else {
                    (
                        "Give Op (+o)",
                        Message::ToggleAccessLevel(nickname, "+o".to_owned()),
                    )
                }
            }
            Entry::ToggleAccessLevelVoice => {
                if user.has_access_level(data::user::AccessLevel::Voice) {
                    (
                        "Take Voice (-v)",
                        Message::ToggleAccessLevel(nickname, "-v".to_owned()),
                    )
                } else {
                    (
                        "Give Voice (+v)",
                        Message::ToggleAccessLevel(nickname, "+v".to_owned()),
                    )
                }
            }
            Entry::SendFile => ("Send File", Message::SendFile(nickname)),
        };

        button(text(content).style(theme::text::primary))
            .padding(5)
            .width(length)
            .style(theme::button::context)
            .on_press(message)
            .into()
    });

    if let Some(current_user) = current_user {
        let tooltip_content = if let Some(current_user) = current_user {
            let user_hostname = current_user
                .hostname()
                .map(|hostname| row![].push(text(hostname).style(theme::text::transparent)));

            let user_status = if current_user.is_away() {
                row![]
                    .push(
                        text("⬤")
                            .style(theme::text::info)
                            .shaping(text::Shaping::Advanced),
                    )
                    .push(text(" Away").style(theme::text::transparent))
                    .align_items(iced::Alignment::Center)
            } else {
                row![]
                    .push(
                        text("⬤")
                            .style(theme::text::success)
                            .shaping(text::Shaping::Advanced),
                    )
                    .push(text(" Online").style(theme::text::transparent))
                    .align_items(iced::Alignment::Center)
            };

            let user_access_levels = column![]
                .push_maybe(
                    current_user
                        .has_access_level(data::user::AccessLevel::Owner)
                        .then(move || row![].push(text("~ Owner").style(theme::text::transparent))),
                )
                .push_maybe(
                    current_user
                        .has_access_level(data::user::AccessLevel::Admin)
                        .then(move || {
                            row![].push(text("& Administrator").style(theme::text::transparent))
                        }),
                )
                .push_maybe(
                    current_user
                        .has_access_level(data::user::AccessLevel::Oper)
                        .then(move || {
                            row![].push(text("@ Operator").style(theme::text::transparent))
                        }),
                )
                .push_maybe(
                    current_user
                        .has_access_level(data::user::AccessLevel::HalfOp)
                        .then(move || {
                            row![].push(text("% Half-Operator").style(theme::text::transparent))
                        }),
                )
                .push_maybe(
                    current_user
                        .has_access_level(data::user::AccessLevel::Voice)
                        .then(move || {
                            row![].push(text("+ Voiced").style(theme::text::transparent))
                        }),
                );

            container(
                column![]
                    .push_maybe(user_hostname)
                    .push(user_status)
                    .push(user_access_levels),
            )
        } else {
            container(
                row![]
                    .push(
                        text("⬤")
                            .style(theme::text::error)
                            .shaping(text::Shaping::Advanced),
                    )
                    .push(text(" Not in Channel").style(theme::text::transparent))
                    .align_items(iced::Alignment::Center),
            )
        };

        iced::widget::tooltip(
            content,
            tooltip_content.style(theme::container::context).padding(8),
            tooltip::Position::Top,
        )
        .into()
    } else {
        content
    }
}
