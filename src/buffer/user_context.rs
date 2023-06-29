use data::User;
use iced::widget::{button, text};

use crate::theme;
use crate::widget::{context_menu, Element};

#[derive(Debug, Clone, Copy)]
enum Entry {
    Whois,
    Query,
    Op,
    Voice,
}

impl Entry {
    fn list(channel: &Option<String>) -> Vec<Self> {
        match channel {
            Some(_) => vec![Entry::Whois, Entry::Query, Entry::Op, Entry::Voice],
            None => vec![Entry::Whois],
        }
    }
}

#[derive(Clone, Debug)]
pub enum Mode {
    Plus(UserMode),
    Minus(UserMode),
}

impl Mode {
    pub fn raw(&self) -> String {
        match self {
            Mode::Plus(user_mode) => format!("+{}", user_mode.raw()),
            Mode::Minus(user_mode) => format!("-{}", user_mode.raw()),
        }
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Mode::Plus(user_mode) => format!("Give {} (+{})", user_mode, user_mode.raw()),
                Mode::Minus(user_mode) => format!("Take {} (-{})", user_mode, user_mode.raw()),
            }
        )
    }
}

#[derive(Clone, Debug)]
pub enum UserMode {
    Op,
    Voice,
}

impl UserMode {
    fn raw(&self) -> &str {
        match self {
            UserMode::Op => "o",
            UserMode::Voice => "v",
        }
    }
}

impl std::fmt::Display for UserMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                UserMode::Op => "Op",
                UserMode::Voice => "Voice",
            }
        )
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    Whois(User),
    Query(User),
    Mode(User, Option<String>, Mode),
}

#[derive(Debug, Clone)]
pub enum Event {
    SendWhois(User),
    OpenQuery(User),
    SendMode(User, Option<String>, Mode),
}

pub fn update(message: Message) -> Event {
    match message {
        Message::Whois(user) => Event::SendWhois(user),
        Message::Query(user) => Event::OpenQuery(user),
        Message::Mode(user, channel, mode) => Event::SendMode(user, channel, mode),
    }
}

pub fn view<'a>(
    content: impl Into<Element<'a, Message>>,
    user: User,
    channel: Option<String>,
) -> Element<'a, Message> {
    let entries = Entry::list(&channel);

    context_menu(content, entries, move |entry| {
        let (content, message) = match entry {
            Entry::Whois => ("Whois".into(), Message::Whois(user.clone())),
            Entry::Query => ("Message".into(), Message::Query(user.clone())),
            Entry::Op => {
                let op = UserMode::Op;
                let mode = if user.has_op() {
                    Mode::Minus(op)
                } else {
                    Mode::Plus(op)
                };

                (
                    mode.to_string(),
                    Message::Mode(user.clone(), channel.clone(), mode),
                )
            }
            Entry::Voice => {
                let voice = UserMode::Voice;
                let mode = if user.has_voice() {
                    Mode::Minus(voice)
                } else {
                    Mode::Plus(voice)
                };

                (
                    mode.to_string(),
                    Message::Mode(user.clone(), channel.clone(), mode),
                )
            }
        };

        button(text(content).style(theme::Text::Primary))
            // Based off longest entry text
            .width(110)
            .style(theme::Button::Context)
            .on_press(message)
            .into()
    })
}
