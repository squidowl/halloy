use data::user::Nick;
use data::{Buffer, User};
use iced::widget::{button, text};

use crate::theme;
use crate::widget::{context_menu, Element};

#[derive(Debug, Clone, Copy)]
enum Entry {
    Whois,
    Query,
    ToggleAccessLevelOp,
    ToggleAccessLevelVoice,
}

impl Entry {
    fn list(buffer: &Buffer) -> Vec<Self> {
        match buffer {
            Buffer::Channel(_, _) => vec![
                Entry::Whois,
                Entry::Query,
                Entry::ToggleAccessLevelOp,
                Entry::ToggleAccessLevelVoice,
            ],
            Buffer::Server(_) | Buffer::Query(_, _) => vec![Entry::Whois],
        }
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    Whois(Nick),
    Query(Nick),
    SingleClick(Nick),
    ToggleAccessLevel(Nick, String),
}

#[derive(Debug, Clone)]
pub enum Event {
    SendWhois(Nick),
    OpenQuery(Nick),
    SingleClick(Nick),
    ToggleAccessLevel(Nick, String),
}

pub fn update(message: Message) -> Event {
    match message {
        Message::Whois(nick) => Event::SendWhois(nick),
        Message::Query(nick) => Event::OpenQuery(nick),
        Message::SingleClick(nick) => Event::SingleClick(nick),
        Message::ToggleAccessLevel(nick, mode) => Event::ToggleAccessLevel(nick, mode),
    }
}

pub fn view<'a>(
    content: impl Into<Element<'a, Message>>,
    user: &'a User,
    buffer: Buffer,
) -> Element<'a, Message> {
    let entries = Entry::list(&buffer);

    let content = button(content)
        .padding(0)
        .style(theme::button::bare)
        .on_press(Message::SingleClick(user.nickname().to_owned()));

    context_menu(content, entries, move |entry, length| {
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
        };

        button(text(content).style(theme::text::primary))
            .padding(5)
            .width(length)
            .style(theme::button::context)
            .on_press(message)
            .into()
    })
}
