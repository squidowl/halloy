use data::user::Nick;
use data::{Buffer, User};
use iced::widget::{button, container, horizontal_rule, row, text, Space};
use iced::{padding, Length, Padding};

use crate::widget::{context_menu, double_pass, Element};
use crate::{icon, theme};

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
    pub fn list(buffer: &Buffer, our_user: Option<&User>) -> Vec<Self> {
        match buffer {
            Buffer::Channel(_, _) => {
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
            }
            Buffer::Server(_) | Buffer::Query(_, _) => vec![Entry::Whois, Entry::SendFile],
        }
    }

    pub fn view<'a>(
        self,
        user: &User,
        current_user: Option<&User>,
        length: Length,
    ) -> Element<'a, Message> {
        let nickname = user.nickname().to_owned();

        match self {
            Entry::Whois => menu_button("Whois", Message::Whois(nickname), length),
            Entry::Query => menu_button("Message", Message::Query(nickname), length),
            Entry::ToggleAccessLevelOp => {
                if user.has_access_level(data::user::AccessLevel::Oper) {
                    menu_button(
                        "Take Op (-o)",
                        Message::ToggleAccessLevel(nickname, "-o".to_owned()),
                        length,
                    )
                } else {
                    menu_button(
                        "Give Op (+o)",
                        Message::ToggleAccessLevel(nickname, "+o".to_owned()),
                        length,
                    )
                }
            }
            Entry::ToggleAccessLevelVoice => {
                if user.has_access_level(data::user::AccessLevel::Voice) {
                    menu_button(
                        "Take Voice (-v)",
                        Message::ToggleAccessLevel(nickname, "-v".to_owned()),
                        length,
                    )
                } else {
                    menu_button(
                        "Give Voice (+v)",
                        Message::ToggleAccessLevel(nickname, "+v".to_owned()),
                        length,
                    )
                }
            }
            Entry::SendFile => menu_button("Send File", Message::SendFile(nickname), length),
            Entry::UserInfo => user_info(current_user, length),
            Entry::HorizontalRule => match length {
                Length::Fill => container(horizontal_rule(1)).padding([0, 6]).into(),
                _ => Space::new(length, 1).into(),
            },
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

pub fn update(message: Message) -> Option<Event> {
    match message {
        Message::Whois(nick) => Some(Event::SendWhois(nick)),
        Message::Query(nick) => Some(Event::OpenQuery(nick)),
        Message::SingleClick(nick) => Some(Event::SingleClick(nick)),
        Message::ToggleAccessLevel(nick, mode) => Some(Event::ToggleAccessLevel(nick, mode)),
        Message::SendFile(nick) => Some(Event::SendFile(nick)),
    }
}

pub fn view<'a>(
    content: impl Into<Element<'a, Message>>,
    user: &'a User,
    current_user: Option<&'a User>,
    buffer: &'a Buffer,
    our_user: Option<&'a User>,
) -> Element<'a, Message> {
    let entries = Entry::list(buffer, our_user);

    let content = button(content)
        .padding(0)
        .style(theme::button::bare)
        .on_press(Message::SingleClick(user.nickname().to_owned()));

    context_menu(content, entries, move |entry, length| {
        entry.view(user, current_user, length)
    })
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

fn user_info<'a>(current_user: Option<&User>, length: Length) -> Element<'a, Message> {
    if let Some(current_user) = current_user {
        if current_user.is_away() {
            row![]
                .push(text("Away").style(theme::text::secondary).width(length))
                .push(
                    icon::dot()
                        .size(6)
                        .style(theme::text::tertiary)
                        .shaping(text::Shaping::Advanced),
                )
                .padding(right_justified_padding())
                .align_y(iced::Alignment::Center)
                .into()
        } else {
            row![]
                .push(text("Online").style(theme::text::secondary).width(length))
                .push(
                    icon::dot()
                        .size(6)
                        .style(theme::text::success)
                        .shaping(text::Shaping::Advanced),
                )
                .padding(right_justified_padding())
                .align_y(iced::Alignment::Center)
                .into()
        }
    } else {
        row![]
            .push(text("Offline").style(theme::text::secondary).width(length))
            .push(
                icon::dot()
                    .size(6)
                    .style(theme::text::error)
                    .shaping(text::Shaping::Advanced),
            )
            .padding(right_justified_padding())
            .align_y(iced::Alignment::Center)
            .into()
    }
}
