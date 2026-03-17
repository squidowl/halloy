use std::collections::HashMap;

use irc::proto;
use irc::proto::format;
use nom::character::complete::char;
use nom::combinator::{cut, map, rest, verify};
use nom::multi::{many_m_n, many0_count, many1_count};
use nom::{Finish, IResult, Parser};

use crate::buffer::{self};
use crate::capabilities::{MultilineBatchKind, MultilineLimits};
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
    code_fence: Option<&CodeFence>,
    our_nickname: Option<NickRef>,
    in_channel: Option<bool>,
    is_connected: bool,
    isupport: &HashMap<isupport::Kind, isupport::Parameter>,
    multiline_limits: Option<&MultilineLimits>,
    relay_bytes: usize,
    config: &Config,
) -> Result<Parsed, Error> {
    let content = if let Some(open_code_fence) = code_fence {
        if let Some(close_code_fence) = parse_code_fence(input)
            .finish()
            .ok()
            .map(|(_, code_fence)| code_fence)
            && close_code_fence.backticks >= open_code_fence.backticks
            && close_code_fence.info.is_none()
        {
            return Ok(Parsed::CodeFence(close_code_fence));
        }

        Content::Text(format!(
            "\u{11}{}\u{11}",
            remove_indent(input, open_code_fence)
                .finish()
                .ok()
                .map_or(input, |(_, unindented)| unindented)
        ))
    } else {
        match auto_format {
            AutoFormat::Disabled => (),
            AutoFormat::Markdown | AutoFormat::All => {
                if let Some(open_code_fence) = parse_code_fence(input)
                    .finish()
                    .ok()
                    .map(|(_, code_fence)| code_fence)
                {
                    return Ok(Parsed::CodeFence(open_code_fence));
                }
            }
        }

        match command::parse(
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
                    command::Internal::Reconnect
                        | command::Internal::Connect(_)
                ) {
                    return Ok(Parsed::Internal(command));
                } else {
                    return Err(Error::Command(command::Error::Disconnected));
                }
            }
            Ok(Command::Irc(command::Irc::Msg(targets, text))) => {
                let text = match auto_format {
                    AutoFormat::Disabled => text,
                    AutoFormat::Markdown => formatting::encode(&text, true),
                    AutoFormat::All => formatting::encode(&text, false),
                };

                Content::Command(command::Irc::Msg(targets, text))
            }
            Ok(Command::Irc(command::Irc::Me(target, text))) => {
                let text = match auto_format {
                    AutoFormat::Disabled => text,
                    AutoFormat::Markdown => formatting::encode(&text, true),
                    AutoFormat::All => formatting::encode(&text, false),
                };

                Content::Command(command::Irc::Me(target, text))
            }
            Ok(Command::Irc(command::Irc::Notice(targets, text))) => {
                let text = match auto_format {
                    AutoFormat::Disabled => text,
                    AutoFormat::Markdown => formatting::encode(&text, true),
                    AutoFormat::All => formatting::encode(&text, false),
                };

                Content::Command(command::Irc::Notice(targets, text))
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
        }
    };

    let parsed = Parsed::Input(Input { buffer, content });

    if let Some(multiline_limits) = multiline_limits
        && let Some((text, _)) = parsed
            .multiline_content(isupport::get_casemapping_or_default(isupport))
    {
        if text.len() > multiline_limits.max_bytes {
            return Err(Error::ExceedsByteLimit {
                bytes: text.len(),
                bytes_limit: multiline_limits.max_bytes,
            });
        }
    } else if let Parsed::Input(Input { buffer, content }) = &parsed
        && let Some(message_bytes) = content
            .proto(buffer)
            .map(|message| format::message(message).len())
    {
        let message_bytes = match &content {
            Content::Text(_)
            | Content::Command(command::Irc::Msg(_, _))
            | Content::Command(command::Irc::Me(_, _))
            | Content::Command(command::Irc::Notice(_, _)) => {
                message_bytes + relay_bytes
            }
            Content::Command(_) => message_bytes,
        };

        if message_bytes > format::BYTE_LIMIT {
            return Err(Error::ExceedsByteLimit {
                bytes: message_bytes,
                bytes_limit: format::BYTE_LIMIT,
            });
        }
    }

    if !is_connected {
        return Err(Error::Command(command::Error::Disconnected));
    } else if in_channel.is_some_and(|in_channel| !in_channel) {
        return Err(Error::Command(command::Error::NotInChannel));
    }

    Ok(parsed)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Parsed {
    Input(Input),
    Internal(command::Internal),
    CodeFence(CodeFence),
}

impl Parsed {
    pub fn code_fence(&self) -> Option<&CodeFence> {
        match &self {
            Parsed::Input(_) | Parsed::Internal(_) => None,
            Parsed::CodeFence(code_fence) => Some(code_fence),
        }
    }

    pub fn multiline_batch_kind(
        &self,
        casemapping: isupport::CaseMap,
    ) -> Option<MultilineBatchKind> {
        self.multiline_content(casemapping).map(|(_, kind)| kind)
    }

    pub fn multiline_content(
        &self,
        casemapping: isupport::CaseMap,
    ) -> Option<(&str, MultilineBatchKind)> {
        match self {
            Parsed::Input(input) => match &input.content {
                Content::Text(text) => {
                    Some((text.as_str(), MultilineBatchKind::PRIVMSG))
                }
                Content::Command(command) => match command {
                    command::Irc::Msg(command_target, text) => {
                        input.buffer.target().and_then(|buffer_target| {
                            (buffer_target.as_normalized_str()
                                == casemapping.normalize(command_target))
                            .then_some((
                                text.as_str(),
                                MultilineBatchKind::PRIVMSG,
                            ))
                        })
                    }
                    command::Irc::Notice(command_target, text) => {
                        input.buffer.target().and_then(|buffer_target| {
                            (buffer_target.as_normalized_str()
                                == casemapping.normalize(command_target))
                            .then_some((
                                text.as_str(),
                                MultilineBatchKind::NOTICE,
                            ))
                        })
                    }
                    _ => None,
                },
            },
            Parsed::Internal(_) => None,
            // Not included in batch, but should not delimit a batch
            Parsed::CodeFence(_) => Some(("", MultilineBatchKind::PRIVMSG)),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
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
            .and_then(|command| proto::Message::try_from(command).ok())
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

#[derive(Debug, Clone, PartialEq)]
pub struct CodeFence {
    indent: usize,
    backticks: usize,
    info: Option<String>,
}

fn parse_code_fence(input: &str) -> IResult<&str, CodeFence> {
    cut(map(
        (parse_indent, parse_backticks, parse_info),
        |(indent, backticks, info)| {
            let info = info.trim();
            CodeFence {
                indent,
                backticks,
                info: (!info.is_empty()).then_some(info.to_string()),
            }
        },
    ))
    .parse(input)
}

fn parse_indent(input: &str) -> IResult<&str, usize> {
    verify(many0_count(char(' ')), |indent: &usize| *indent <= 3).parse(input)
}

fn parse_backticks(input: &str) -> IResult<&str, usize> {
    verify(many1_count(char('`')), |backticks: &usize| *backticks >= 3)
        .parse(input)
}

fn parse_info(input: &str) -> IResult<&str, &str> {
    verify(rest, |info: &str| !info.contains('`')).parse(input)
}

fn remove_indent<'a>(
    input: &'a str,
    code_fence: &CodeFence,
) -> IResult<&'a str, &'a str> {
    map(
        (many_m_n(0, code_fence.indent, char(' ')), rest),
        |(_, unindented)| unindented,
    )
    .parse(input)
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Error {
    #[error(
        "message exceeds maximum encoded length ({}/{} bytes)",
        bytes,
        bytes_limit
    )]
    ExceedsByteLimit { bytes: usize, bytes_limit: usize },
    #[error(transparent)]
    Command(#[from] command::Error),
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::capabilities::MultilineLimits;
    use crate::config::buffer::text_input::AutoFormat;
    use crate::input::{CodeFence, Content, Input, Parsed, parse};
    use crate::user::Nick;
    use crate::{Config, Server, buffer, command, isupport, target};

    #[test]
    fn parsing() {
        let config = Config::default();
        let isupport = HashMap::<isupport::Kind, isupport::Parameter>::new();
        let multiline_limits = MultilineLimits {
            max_bytes: 4096,
            max_lines: Some(24),
        };
        let nick = Nick::from_str(
            "tester",
            isupport::get_casemapping_or_default(&isupport),
        );
        let buffer = buffer::Upstream::Channel(
            Server {
                name: "Libera".into(),
                network: None,
            },
            target::Channel::from_str(
                "##chat",
                isupport::get_chantypes_or_default(&isupport),
                isupport::get_casemapping_or_default(&isupport),
            ),
        );
        let tests = [
            (
                (
                    AutoFormat::Disabled,
                    "``` no autoformat ``` _at_ **all**",
                    None,
                ),
                Ok(Parsed::Input(Input {
                    buffer: buffer.clone(),
                    content: Content::Text(String::from(
                        "``` no autoformat ``` _at_ **all**",
                    )),
                })),
            ),
            (
                (AutoFormat::Markdown, "```toml", None),
                Ok(Parsed::CodeFence(CodeFence {
                    indent: 0,
                    backticks: 3,
                    info: Some(String::from("toml")),
                })),
            ),
            (
                (
                    AutoFormat::Markdown,
                    "```toml",
                    Some(&CodeFence {
                        indent: 0,
                        backticks: 3,
                        info: Some(String::from("toml")),
                    }),
                ),
                Ok(Parsed::Input(Input {
                    buffer: buffer.clone(),
                    content: Content::Text(String::from("\u{11}```toml\u{11}")),
                })),
            ),
            (
                (
                    AutoFormat::Markdown,
                    "  ```",
                    Some(&CodeFence {
                        indent: 0,
                        backticks: 3,
                        info: Some(String::from("toml")),
                    }),
                ),
                Ok(Parsed::CodeFence(CodeFence {
                    indent: 2,
                    backticks: 3,
                    info: None,
                })),
            ),
            (
                (
                    AutoFormat::Markdown,
                    "`````",
                    Some(&CodeFence {
                        indent: 0,
                        backticks: 3,
                        info: Some(String::from("toml")),
                    }),
                ),
                Ok(Parsed::CodeFence(CodeFence {
                    indent: 0,
                    backticks: 5,
                    info: None,
                })),
            ),
            (
                (
                    AutoFormat::Markdown,
                    "    key_bindings = \"emacs\"",
                    Some(&CodeFence {
                        indent: 2,
                        backticks: 3,
                        info: None,
                    }),
                ),
                Ok(Parsed::Input(Input {
                    buffer: buffer.clone(),
                    content: Content::Text(String::from(
                        "\u{11}  key_bindings = \"emacs\"\u{11}",
                    )),
                })),
            ),
            (
                (
                    AutoFormat::Markdown,
                    "  key_bindings = \"emacs\"",
                    Some(&CodeFence {
                        indent: 3,
                        backticks: 3,
                        info: None,
                    }),
                ),
                Ok(Parsed::Input(Input {
                    buffer: buffer.clone(),
                    content: Content::Text(String::from(
                        "\u{11}key_bindings = \"emacs\"\u{11}",
                    )),
                })),
            ),
            (
                (
                    AutoFormat::Markdown,
                    "/me thinks in _italics_ and **bold**",
                    None,
                ),
                Ok(Parsed::Input(Input {
                    buffer: buffer.clone(),
                    content: Content::Command(command::Irc::Me(
                        String::from("##chat"),
                        String::from(
                            "thinks in \u{1d}italics\u{1d} and \u{2}bold\u{2}",
                        ),
                    )),
                })),
            ),
        ];
        for ((auto_format, input, code_fence), expected) in tests {
            let parsed = parse(
                buffer.clone(),
                auto_format,
                input,
                code_fence,
                Some(nick.as_nickref()),
                Some(true),
                true,
                &isupport,
                Some(&multiline_limits),
                128,
                &config,
            );

            assert_eq!(parsed, expected);
        }
    }
}
