use chrono::Utc;
use irc::proto;
use irc::proto::format;

use crate::time::Posix;
use crate::user::NickRef;
use crate::{command, message, Buffer, Command, Message, Server, User};

pub fn parse(buffer: Buffer, input: &str) -> Result<Input, Error> {
    let content = match command::parse(input, Some(&buffer)) {
        Ok(command) => Content::Command(command),
        Err(command::Error::MissingSlash) => Content::Text(input.to_string()),
        Err(error) => return Err(Error::Command(error)),
    };

    if content
        .proto(&buffer)
        .map(exceeds_byte_limit)
        .unwrap_or_default()
    {
        return Err(Error::ExceedsByteLimit);
    }

    Ok(Input {
        buffer,
        content,
        raw: Some(input.to_string()),
    })
}

#[derive(Debug, Clone)]
pub struct Input {
    buffer: Buffer,
    content: Content,
    raw: Option<String>,
}

impl Input {
    pub fn command(buffer: Buffer, command: Command) -> Self {
        Self {
            buffer,
            content: Content::Command(command),
            raw: None,
        }
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn server(&self) -> &Server {
        self.buffer.server()
    }

    pub fn message(&self, our_nick: NickRef) -> Option<Message> {
        let to_target = |target: String, source| {
            if proto::is_channel(&target) {
                Some(message::Target::Channel {
                    channel: target,
                    source,
                })
            } else if let Ok(user) = User::try_from(target) {
                Some(message::Target::Query {
                    nick: user.nickname().to_owned(),
                    source,
                })
            } else {
                None
            }
        };

        let command = self.content.command(&self.buffer)?;

        match command {
            Command::Msg(target, text) => Some(Message {
                received_at: Posix::now(),
                server_time: Utc::now(),
                direction: message::Direction::Sent,
                target: to_target(
                    target,
                    message::Source::User(User::from(our_nick.to_owned())),
                )?,
                text,
            }),
            Command::Me(target, action) => Some(Message {
                received_at: Posix::now(),
                server_time: Utc::now(),
                direction: message::Direction::Sent,
                target: to_target(target, message::Source::Action)?,
                text: message::action_text(our_nick, &action),
            }),
            _ => None,
        }
    }

    pub fn encoded(&self) -> Option<message::Encoded> {
        self.content.proto(&self.buffer).map(message::Encoded::from)
    }

    pub fn raw(&self) -> Option<&str> {
        self.raw.as_deref()
    }
}

#[derive(Debug, Clone)]
pub struct InputDraft {
    buffer: Buffer,
    text: String,
}

impl InputDraft {
    pub fn new(buffer: Buffer, text: String) -> Self {
        InputDraft { buffer, text }
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn text(&self) -> &str {
        self.text.as_ref()
    }
}

#[derive(Debug, Clone)]
enum Content {
    Text(String),
    Command(Command),
}

impl Content {
    fn command(&self, buffer: &Buffer) -> Option<Command> {
        match self {
            Self::Text(text) => {
                let target = buffer.target()?;
                Some(Command::Msg(target, text.clone()))
            }
            Self::Command(command) => Some(command.clone()),
        }
    }

    fn proto(&self, buffer: &Buffer) -> Option<proto::Message> {
        self.command(buffer)
            .and_then(|command| proto::Command::try_from(command).ok())
            .map(proto::Message::from)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(
        "message exceeds maximum encoded length of {} bytes",
        format::BYTE_LIMIT
    )]
    ExceedsByteLimit,
    #[error(transparent)]
    Command(#[from] command::Error),
}

fn exceeds_byte_limit(message: proto::Message) -> bool {
    format::message(message).len() > format::BYTE_LIMIT
}
