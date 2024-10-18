use std::collections::HashMap;

use irc::proto;
use irc::proto::format;

use crate::buffer::{self, AutoFormat};
use crate::message::formatting;
use crate::{command, message, Command, Message, Server, User};

const INPUT_HISTORY_LENGTH: usize = 100;

pub fn parse(
    buffer: buffer::Upstream,
    auto_format: AutoFormat,
    input: &str,
) -> Result<Input, Error> {
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
    pub buffer: buffer::Upstream,
    content: Content,
    raw: Option<String>,
}

impl Input {
    pub fn command(buffer: buffer::Upstream, command: Command) -> Self {
        Self {
            buffer,
            content: Content::Command(command),
            raw: None,
        }
    }

    pub fn server(&self) -> &Server {
        self.buffer.server()
    }

    pub fn messages(&self, user: User, channel_users: &[User], chantypes: &[char]) -> Option<Vec<Message>> {
        let to_target = |target: &str, source| {
            if let Some((prefix, channel)) = proto::parse_channel_from_target(target, chantypes) {
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
                    .map(|target| {
                        Message::sent(
                            target,
                            message::parse_fragments(text.clone(), channel_users),
                        )
                    })
                    .collect(),
            ),
            Command::Me(target, action) => Some(vec![Message::sent(
                to_target(&target, message::Source::Action)?,
                message::action_text(user.nickname(), Some(&action)),
            )]),
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
    fn command(&self, buffer: &buffer::Upstream) -> Option<Command> {
        match self {
            Self::Text(text) => {
                let target = buffer.target()?;
                Some(Command::Msg(target, text.clone()))
            }
            Self::Command(command) => Some(command.clone()),
        }
    }

    fn proto(&self, buffer: &buffer::Upstream) -> Option<proto::Message> {
        self.command(buffer)
            .and_then(|command| proto::Command::try_from(command).ok())
            .map(proto::Message::from)
    }
}

#[derive(Debug, Clone)]
pub struct Draft {
    pub buffer: buffer::Upstream,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct Storage {
    sent: HashMap<buffer::Upstream, Vec<String>>,
    draft: HashMap<buffer::Upstream, String>,
}

impl Storage {
    pub fn get<'a>(&'a self, buffer: &buffer::Upstream) -> Cache<'a> {
        Cache {
            history: self.sent.get(buffer).map(Vec::as_slice).unwrap_or_default(),
            draft: self
                .draft
                .get(buffer)
                .map(AsRef::as_ref)
                .unwrap_or_default(),
        }
    }

    pub fn record(&mut self, buffer: &buffer::Upstream, text: String) {
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
