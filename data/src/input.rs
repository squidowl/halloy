use std::collections::HashMap;

use chrono::Utc;
use irc::proto;
use irc::proto::format;

use crate::buffer::AutoFormat;
use crate::message::formatting;
use crate::time::Posix;
use crate::{command, message, Buffer, Command, Message, Server, User};

const INPUT_HISTORY_LENGTH: usize = 100;

pub fn parse(buffer: Buffer, auto_format: AutoFormat, input: &str) -> Result<Input, Error> {
    let content = match command::parse(input, Some(&buffer)) {
        Ok(command) => Content::Command(command),
        Err(command::Error::MissingSlash) => {
            let text = match auto_format {
                AutoFormat::Disabled => input.to_string(),
                AutoFormat::Markdown => formatting::encode(input, true),
                AutoFormat::All => formatting::encode(input, false),
            };

            Content::Text(text)
        }
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

    pub fn messages(&self, user: User) -> Option<Vec<Message>> {
        let to_target = |target: &str, source| {
            if let Some((prefix, channel)) = proto::parse_channel_from_target(target) {
                Some(message::Target::Channel {
                    channel,
                    source,
                    prefix,
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
            Command::Msg(targets, text) => Some(
                targets
                    .split(',')
                    .filter_map(|target| to_target(target, message::Source::User(user.clone())))
                    .map(|target| Message {
                        received_at: Posix::now(),
                        server_time: Utc::now(),
                        direction: message::Direction::Sent,
                        target,
                        content: message::parse_fragments(text.clone()),
                        id: None,
                    })
                    .collect(),
            ),
            Command::Me(target, action) => Some(vec![Message {
                received_at: Posix::now(),
                server_time: Utc::now(),
                direction: message::Direction::Sent,
                target: to_target(&target, message::Source::Action)?,
                content: message::action_text(user.nickname(), Some(&action)),
                id: None,
            }]),
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

#[derive(Debug, Clone)]
pub struct Draft {
    pub buffer: Buffer,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct Storage {
    sent: HashMap<Buffer, Vec<String>>,
    draft: HashMap<Buffer, String>,
}

impl Storage {
    pub fn get<'a>(&'a self, buffer: &Buffer) -> Cache<'a> {
        Cache {
            history: self.sent.get(buffer).map(Vec::as_slice).unwrap_or_default(),
            draft: self
                .draft
                .get(buffer)
                .map(AsRef::as_ref)
                .unwrap_or_default(),
        }
    }

    pub fn record(&mut self, buffer: &Buffer, text: String) {
        self.draft.remove(buffer);
        let history = self.sent.entry(buffer.clone()).or_default();
        history.insert(0, text);
        history.truncate(INPUT_HISTORY_LENGTH);
    }

    pub fn store_draft(&mut self, draft: Draft) {
        self.draft.insert(draft.buffer, draft.text);
    }
}

/// Cached values for a buffers input
#[derive(Debug, Clone, Copy)]
pub struct Cache<'a> {
    pub history: &'a [String],
    pub draft: &'a str,
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
