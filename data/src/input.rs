use std::collections::HashMap;

use irc::proto;
use irc::proto::format;

use crate::buffer::{self};
use crate::config::buffer::text_input::AutoFormat;
use crate::message::formatting;
use crate::target::Target;
use crate::user::{ChannelUsers, NickRef};
use crate::{
    Command, Config, Message, Server, User, command, isupport, message,
};

const INPUT_HISTORY_LENGTH: usize = 100;

pub fn parse(
    buffer: buffer::Upstream,
    auto_format: AutoFormat,
    input: &str,
    our_nickname: Option<NickRef>,
    in_channel: Option<bool>,
    is_connected: bool,
    isupport: &HashMap<isupport::Kind, isupport::Parameter>,
    config: &Config,
) -> Result<Parsed, Error> {
    let content = match command::parse(
        input,
        Some(&buffer),
        our_nickname,
        is_connected,
        isupport,
        config,
    ) {
        Ok(Command::Internal(command)) => {
            if is_connected {
                if matches!(command, command::Internal::Reconnect) {
                    return Err(Error::Command(command::Error::Connected));
                } else {
                    return Ok(Parsed::Internal(command));
                }
            } else if matches!(
                command,
                command::Internal::Reconnect | command::Internal::Connect(_)
            ) {
                return Ok(Parsed::Internal(command));
            } else {
                return Err(Error::Command(command::Error::Disconnected));
            }
        }
        Ok(Command::Irc(command)) => Content::Command(command),
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

    if let Some(message_bytes) = content
        .proto(&buffer)
        .map(|message| format::message(message).len())
        && message_bytes > format::BYTE_LIMIT
    {
        return Err(Error::ExceedsByteLimit { message_bytes });
    }

    if !is_connected {
        return Err(Error::Command(command::Error::Disconnected));
    } else if in_channel.is_some_and(|in_channel| !in_channel) {
        return Err(Error::Command(command::Error::NotInChannel));
    }

    Ok(Parsed::Input(Input { buffer, content }))
}

pub enum Parsed {
    Input(Input),
    Internal(command::Internal),
}

#[derive(Debug, Clone)]
pub struct Input {
    pub buffer: buffer::Upstream,
    content: Content,
}

impl Input {
    pub fn from_command(
        buffer: buffer::Upstream,
        command: command::Irc,
    ) -> Self {
        Self {
            buffer,
            content: Content::Command(command),
        }
    }

    pub fn command(&self) -> Option<&command::Irc> {
        match &self.content {
            Content::Text(_) => None,
            Content::Command(command) => Some(command),
        }
    }

    pub fn server(&self) -> &Server {
        self.buffer.server()
    }

    pub fn messages(
        &self,
        user: User,
        channel_users: Option<&ChannelUsers>,
        chantypes: &[char],
        statusmsg: &[char],
        casemapping: isupport::CaseMap,
        supports_echoes: bool,
    ) -> Option<Vec<Message>> {
        self.content.command(&self.buffer).and_then(|command| {
            command.messages(
                user,
                channel_users,
                chantypes,
                statusmsg,
                casemapping,
                supports_echoes,
            )
        })
    }

    pub fn targets(
        &self,
        chantypes: &[char],
        statusmsg: &[char],
        casemapping: isupport::CaseMap,
    ) -> Option<Vec<Target>> {
        let command = self.content.command(&self.buffer)?;

        match command {
            command::Irc::Msg(targets, _)
            | command::Irc::Notice(targets, _) => Some(
                targets
                    .split(',')
                    .map(|target| {
                        Target::parse(target, chantypes, statusmsg, casemapping)
                    })
                    .collect(),
            ),
            command::Irc::Me(target, _) => Some(vec![Target::parse(
                &target,
                chantypes,
                statusmsg,
                casemapping,
            )]),
            _ => None,
        }
    }

    pub fn encoded(&self) -> Option<message::Encoded> {
        self.content.proto(&self.buffer).map(message::Encoded::from)
    }
}

#[derive(Debug, Clone)]
enum Content {
    Text(String),
    Command(command::Irc),
}

impl Content {
    fn command(&self, buffer: &buffer::Upstream) -> Option<command::Irc> {
        match self {
            Self::Text(text) => {
                let target = buffer.target()?;
                Some(command::Irc::Msg(target.to_string(), text.clone()))
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
pub struct RawInput {
    pub buffer: buffer::Upstream,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct Storage {
    sent: HashMap<buffer::Upstream, Vec<String>>,
    draft: HashMap<buffer::Upstream, String>,
    cursor_position: HashMap<buffer::Upstream, (usize, usize)>,
}

impl Storage {
    pub fn get<'a>(&'a self, buffer: &buffer::Upstream) -> Cache<'a> {
        Cache {
            history: self
                .sent
                .get(buffer)
                .map(Vec::as_slice)
                .unwrap_or_default(),
            draft: self
                .draft
                .get(buffer)
                .map(AsRef::as_ref)
                .unwrap_or_default(),
            cursor_position: self.cursor_position.get(buffer),
        }
    }

    pub fn record(&mut self, buffer: &buffer::Upstream, text: String) {
        self.draft.remove(buffer);
        let history = self.sent.entry(buffer.clone()).or_default();
        history.insert(0, text);
        history.truncate(INPUT_HISTORY_LENGTH);
    }

    pub fn store_draft(&mut self, raw_input: RawInput) {
        self.draft.insert(raw_input.buffer, raw_input.text);
    }
}

/// Cached values for a buffers input
#[derive(Debug, Clone, Copy)]
pub struct Cache<'a> {
    pub history: &'a [String],
    pub draft: &'a str,
    pub cursor_position: Option<&'a (usize, usize)>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(
        "message exceeds maximum encoded length ({}/{} bytes)",
        message_bytes,
        format::BYTE_LIMIT
    )]
    ExceedsByteLimit { message_bytes: usize },
    #[error(transparent)]
    Command(#[from] command::Error),
}
