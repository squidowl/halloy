use chrono::Utc;
use irc::proto::ChannelExt;

use crate::time::Posix;
use crate::user::NickRef;
use crate::{command, message, Buffer, Command, Message, Server, User};

pub fn parse(buffer: Buffer, input: &str) -> Result<Input, command::Error> {
    let content = match command::parse(input, &buffer) {
        Ok(command) => Content::Command(command),
        Err(command::Error::MissingSlash) => Content::Text(input.to_string()),
        Err(error) => return Err(error),
    };

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
        let command = self.content.command(&self.buffer)?;

        let source = |target: String, sender| {
            if target.is_channel_name() {
                Some(message::Source::Channel(target, sender))
            } else if let Ok(user) = User::try_from(target) {
                Some(message::Source::Query(user.nickname().to_owned(), sender))
            } else {
                None
            }
        };

        match command {
            Command::Msg(target, text) => Some(Message {
                received_at: Posix::now(),
                server_time: Utc::now(),
                direction: message::Direction::Sent,
                source: source(
                    target,
                    message::Sender::User(User::new(our_nick.to_owned(), None, None)),
                )?,
                text,
            }),
            Command::Me(target, action) => Some(Message {
                received_at: Posix::now(),
                server_time: Utc::now(),
                direction: message::Direction::Sent,
                source: source(target, message::Sender::Action)?,
                text: message::action_text(our_nick, &action),
            }),
            _ => None,
        }
    }

    pub fn encoded(&self) -> Option<message::Encoded> {
        use irc::proto;

        let command = self
            .content
            .command(&self.buffer)
            .and_then(|command| proto::Command::try_from(command).ok())?;

        Some(message::Encoded::from(proto::Message::from(command)))
    }

    pub fn raw(&self) -> Option<&str> {
        self.raw.as_deref()
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
}
