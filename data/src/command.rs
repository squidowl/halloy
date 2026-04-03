use std::borrow::Cow;
use std::collections::HashMap;
use std::str::FromStr;

use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, Utc};
use fancy_regex::Regex;
use irc::proto::{self, tags};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::buffer::{self, Upstream};
use crate::capabilities::{Capabilities, Capability};
use crate::config::buffer::text_input::AutoFormat;
use crate::isupport::{self, find_target_limit};
use crate::message::{self, formatting};
use crate::user::{ChannelUsers, NickRef};
use crate::{Config, Message, Target, Url, User, ctcp, target};

pub mod alias;

pub use self::alias::Alias;

#[derive(Debug, Clone)]
pub enum Command {
    Internal(Internal),
    Irc(Irc),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Internal {
    OpenBuffers(Vec<Target>),
    LeaveBuffers(Vec<Target>, Option<String>),
    ClearBuffer,
    /// Part the current channel and join a new one.
    ///
    /// - Channel to join
    /// - Part message
    Hop(Option<String>, Option<String>),
    ChannelDiscovery,
    Delay(u64),
    SysInfo,
    Detach(Vec<target::Channel>),
    Connect(String),
    Reconnect,
    Upload(String),
    Exec(String),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum Irc {
    List(Option<String>, Option<String>),
    Join(String, Option<String>),
    Motd(Option<String>),
    Nick(String),
    Quit(Option<String>),
    Msg(String, String),
    React {
        target: String,
        msgid: message::Id,
        text: Cow<'static, str>,
    },
    Unreact {
        target: String,
        msgid: message::Id,
        text: Cow<'static, str>,
    },
    Me(String, String),
    Whois(Option<String>, String),
    Whowas(String, Option<String>),
    Part(String, Option<String>),
    Topic(String, Option<String>),
    Kick(String, String, Option<String>),
    Mode(String, Option<String>, Option<Vec<String>>),
    Away(Option<String>),
    SetName(String),
    Notice(String, String),
    Typing {
        target: String,
        value: Typing,
    },
    Raw(String),
    Unknown(String, Vec<String>),
    Ctcp(ctcp::Command, String, Option<String>),
    Chathistory(String, Vec<String>),
    Monitor(String, Option<String>),
    Invite(String, String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum Typing {
    Active,
    Done,
}

impl Typing {
    fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Done => "done",
        }
    }
}

impl Irc {
    pub fn messages(
        &self,
        user: User,
        channel_users: Option<&ChannelUsers>,
        chantypes: &[char],
        statusmsg: &[char],
        casemapping: isupport::CaseMap,
        supports_echoes: bool,
    ) -> Option<Vec<Message>> {
        let to_message_target = |target: &str, source| {
            if target == "*" || target.starts_with('$') {
                return message::Target::Server { source };
            }

            let target =
                Target::parse(target, chantypes, statusmsg, casemapping);

            match &target {
                Target::Channel(channel) => message::Target::Channel {
                    channel: channel.clone(),
                    source,
                },

                Target::Query(query) => message::Target::Query {
                    query: query.clone(),
                    source,
                },
            }
        };

        match self {
            Irc::Msg(targets, text) => Some(
                targets
                    .split(',')
                    .map(|target| {
                        let message_target = to_message_target(
                            target,
                            message::Source::User(user.clone()),
                        );

                        Message::sent(
                            message_target,
                            message::parse_fragments_with_users(
                                text.clone(),
                                channel_users,
                                casemapping,
                            ),
                            supports_echoes.then_some(Irc::Msg(
                                target.to_string(),
                                text.clone(),
                            )),
                        )
                    })
                    .collect(),
            ),
            Irc::Notice(targets, text) => Some(
                targets
                    .split(',')
                    .map(|target| {
                        let message_target = to_message_target(
                            target,
                            message::Source::User(user.clone()),
                        );

                        Message::sent(
                            message_target,
                            message::parse_fragments_with_users(
                                text.clone(),
                                channel_users,
                                casemapping,
                            ),
                            supports_echoes.then_some(Irc::Notice(
                                target.to_string(),
                                text.clone(),
                            )),
                        )
                    })
                    .collect(),
            ),
            Irc::Me(target, action) => {
                let message_target = to_message_target(
                    target,
                    message::Source::Action(Some(user.clone())),
                );

                Some(vec![Message::sent(
                    message_target,
                    message::action_text(
                        &user,
                        Some(action),
                        channel_users,
                        casemapping,
                    ),
                    supports_echoes.then_some(self.clone()),
                )])
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Kind {
    Join,
    Motd,
    Nick,
    Quit,
    Msg,
    Me,
    Whois,
    Part,
    Topic,
    Kick,
    Mode,
    Format,
    FormatMe,
    FormatMsg,
    FormatNotice,
    Plain,
    PlainMe,
    PlainMsg,
    PlainNotice,
    Away,
    SetName,
    Ctcp,
    Chathistory,
    Monitor,
    Invite,
    Hop,
    Notice,
    Delay,
    Clear,
    List,
    ClearTopic,
    SysInfo,
    Detach,
    Connect,
    Reconnect,
    Upload,
    Exec,
    Raw,
}

impl FromStr for Kind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "join" | "j" => Ok(Kind::Join),
            "motd" => Ok(Kind::Motd),
            "nick" => Ok(Kind::Nick),
            "quit" | "disconnect" => Ok(Kind::Quit),
            "msg" | "query" => Ok(Kind::Msg),
            "me" | "describe" => Ok(Kind::Me),
            "whois" => Ok(Kind::Whois),
            "part" | "leave" => Ok(Kind::Part),
            "topic" | "t" => Ok(Kind::Topic),
            "kick" => Ok(Kind::Kick),
            "mode" | "m" => Ok(Kind::Mode),
            "format" | "f" => Ok(Kind::Format),
            "format-me" => Ok(Kind::FormatMe),
            "format-msg" => Ok(Kind::FormatMsg),
            "format-notice" => Ok(Kind::FormatNotice),
            "plain" | "p" => Ok(Kind::Plain),
            "plain-me" => Ok(Kind::PlainMe),
            "plain-msg" => Ok(Kind::PlainMsg),
            "plain-notice" => Ok(Kind::PlainNotice),
            "away" => Ok(Kind::Away),
            "setname" => Ok(Kind::SetName),
            "notice" => Ok(Kind::Notice),
            "raw" => Ok(Kind::Raw),
            "ctcp" => Ok(Kind::Ctcp),
            "chathistory" => Ok(Kind::Chathistory),
            "monitor" => Ok(Kind::Monitor),
            "invite" => Ok(Kind::Invite),
            "hop" | "rejoin" => Ok(Kind::Hop),
            "delay" => Ok(Kind::Delay),
            "clear" => Ok(Kind::Clear),
            "list" => Ok(Kind::List),
            "cleartopic" | "ct" => Ok(Kind::ClearTopic),
            "sysinfo" => Ok(Kind::SysInfo),
            "detach" => Ok(Kind::Detach),
            "connect" => Ok(Kind::Connect),
            "reconnect" => Ok(Kind::Reconnect),
            "upload" => Ok(Kind::Upload),
            "exec" => Ok(Kind::Exec),
            _ => Err(()),
        }
    }
}

pub fn parse(
    s: &str,
    buffer: Option<&buffer::Upstream>,
    our_nickname: Option<NickRef>,
    auto_format: AutoFormat,
    is_connected: bool,
    isupport: &HashMap<isupport::Kind, isupport::Parameter>,
    capabilities: &Capabilities,
    supports_detach: bool,
    config: &Config,
) -> Result<Command, Error> {
    let parsed = parse_input(s)?;
    let alias_context = alias::Context::new(buffer, our_nickname);
    let expanded = alias::expand(parsed.0, parsed.2, &alias_context, config)?;

    let (command_name, args, raw_args) = match expanded.as_deref() {
        Some(input) => parse_input(input)?,
        None => parsed,
    };

    parse_command(
        command_name,
        args,
        raw_args,
        buffer,
        our_nickname,
        auto_format,
        is_connected,
        isupport,
        capabilities,
        supports_detach,
        config,
    )
}

fn parse_input(input: &str) -> Result<(&str, Vec<&str>, &str), Error> {
    let (head, rest) = input.split_once('/').ok_or(Error::MissingSlash)?;
    // Don't allow leading whitespace before slash
    if !head.is_empty() {
        return Err(Error::MissingSlash);
    }

    let mut split = rest.split(' ');
    let command = split.next().ok_or(Error::MissingCommand)?;
    let args = split.collect::<Vec<_>>();
    let raw_args = if rest.len() == command.len() {
        ""
    } else {
        &rest[command.len() + 1..]
    };

    Ok((command, args, raw_args))
}

fn parse_command(
    cmd: &str,
    args: Vec<&str>,
    raw: &str,
    buffer: Option<&buffer::Upstream>,
    our_nickname: Option<NickRef>,
    auto_format: AutoFormat,
    is_connected: bool,
    isupport: &HashMap<isupport::Kind, isupport::Parameter>,
    capabilities: &Capabilities,
    supports_detach: bool,
    config: &Config,
) -> Result<Command, Error> {
    let unknown = || {
        Command::Irc(Irc::Unknown(
            cmd.to_string(),
            args.iter().map(ToString::to_string).collect(),
        ))
    };

    match cmd.parse::<Kind>() {
        Ok(kind) => match kind {
            Kind::Join => {
                validated::<0, 2, false>(args, |[], [chanlist, chankeys]| {
                    let (chanlist, chankeys) = if let Some(chanlist) = chanlist
                    {
                        let chantypes =
                            isupport::get_chantypes_or_default(isupport);

                        if !chanlist.contains(',')
                            && !proto::is_channel(&chanlist, chantypes)
                            && chankeys.is_none()
                            && let Some(channel) = buffer
                                .and_then(Upstream::target)
                                .and_then(Target::to_channel)
                        {
                            (channel.to_string(), Some(chanlist))
                        } else {
                            (chanlist, chankeys)
                        }
                    } else {
                        let Some(channel) = buffer
                            .and_then(Upstream::target)
                            .and_then(Target::to_channel)
                        else {
                            // If not in a channel then the chanlist argument is
                            // required
                            return Err(Error::IncorrectArgCount {
                                min: 1,
                                max: 2,
                                actual: 0,
                            });
                        };

                        (channel.to_string(), None)
                    };

                    let chan_limits =
                        if let Some(isupport::Parameter::CHANLIMIT(limits)) =
                            isupport.get(&isupport::Kind::CHANLIMIT)
                        {
                            Some(limits)
                        } else {
                            None
                        };

                    let channel_len =
                        if let Some(isupport::Parameter::CHANNELLEN(max_len)) =
                            isupport.get(&isupport::Kind::CHANNELLEN)
                        {
                            Some(*max_len as usize)
                        } else {
                            None
                        };

                    if chan_limits.is_some() || channel_len.is_some() {
                        let channels = chanlist.split(',').collect::<Vec<_>>();

                        if let Some(chan_limits) = chan_limits {
                            for chan_limit in chan_limits {
                                if let Some(limit) = chan_limit.limit {
                                    let limit = limit as usize;

                                    if channels
                                        .iter()
                                        .filter(|channel| {
                                            channel
                                                .starts_with(chan_limit.prefix)
                                        })
                                        .count()
                                        > limit
                                    {
                                        return Err(Error::TooManyTargets {
                                            name: "channels",
                                            number: channels.len(),
                                            max_number: limit,
                                        });
                                    }
                                }
                            }
                        }

                        if let Some(max_len) = channel_len
                            && let Some(channel) = channels
                                .into_iter()
                                .find(|channel| channel.len() > max_len)
                        {
                            return Err(Error::ArgTooLong {
                                name: "channel in channels",
                                len: channel.len(),
                                max_len,
                            });
                        }
                    }

                    if let Some(ref chankeys) = chankeys
                        && let Some(isupport::Parameter::KEYLEN(max_len)) =
                            isupport.get(&isupport::Kind::KEYLEN)
                    {
                        let max_len = *max_len as usize;

                        let keys = chankeys.split(',').collect::<Vec<_>>();

                        if let Some(key) =
                            keys.into_iter().find(|key| key.len() > max_len)
                        {
                            return Err(Error::ArgTooLong {
                                name: "key in chankeys",
                                len: key.len(),
                                max_len,
                            });
                        }
                    }

                    Ok(Command::Irc(Irc::Join(chanlist, chankeys)))
                })
            }
            Kind::Motd => validated::<0, 1, false>(args, |_, [target]| {
                Ok(Command::Irc(Irc::Motd(target)))
            }),
            Kind::Nick => validated::<1, 0, false>(args, |[nick], _| {
                if let Some(isupport::Parameter::NICKLEN(max_len)) =
                    isupport.get(&isupport::Kind::NICKLEN)
                {
                    let max_len = *max_len as usize;

                    if nick.len() > max_len {
                        return Err(Error::ArgTooLong {
                            name: "nickname",
                            len: nick.len(),
                            max_len,
                        });
                    }
                }

                Ok(Command::Irc(Irc::Nick(nick)))
            }),
            Kind::Quit => validated::<0, 1, true>(args, |_, [comment]| {
                Ok(Command::Irc(Irc::Quit(comment.or_else(|| {
                    config.buffer.commands.quit.default_reason.clone()
                }))))
            }),
            Kind::Msg | Kind::FormatMsg | Kind::PlainMsg => {
                validated::<1, 1, true>(args, |[targets], [msg]| {
                    let target_limit = find_target_limit(isupport, "PRIVMSG")
                        .map(|limit| limit as usize);

                    if let Some(target_limit) = target_limit {
                        let targets = targets.split(',').collect::<Vec<_>>();

                        if targets.len() > target_limit {
                            return Err(Error::TooManyTargets {
                                name: "targets",
                                number: targets.len(),
                                max_number: target_limit,
                            });
                        }
                    }

                    if let Some(msg) = msg {
                        let msg = match kind {
                            Kind::FormatMsg => formatting::encode(&msg, false),
                            Kind::PlainMsg => msg,
                            _ => match auto_format {
                                AutoFormat::Disabled => msg,
                                AutoFormat::Markdown => {
                                    formatting::encode(&msg, true)
                                }
                                AutoFormat::All => {
                                    formatting::encode(&msg, false)
                                }
                            },
                        };

                        Ok(Command::Irc(Irc::Msg(targets, msg)))
                    } else {
                        let casemapping =
                            isupport::get_casemapping_or_default(isupport);
                        let chantypes =
                            isupport::get_chantypes_or_default(isupport);
                        let statusmsg =
                            isupport::get_statusmsg_or_default(isupport);

                        Ok(Command::Internal(Internal::OpenBuffers(
                            targets
                                .split(",")
                                .map(|target| {
                                    Target::parse(
                                        target,
                                        chantypes,
                                        statusmsg,
                                        casemapping,
                                    )
                                })
                                .collect(),
                        )))
                    }
                })
            }
            Kind::Me => {
                if let Some(target) = buffer.and_then(Upstream::target) {
                    validated::<1, 0, true>(args, |[text], _| {
                        let text = match auto_format {
                            AutoFormat::Disabled => text,
                            AutoFormat::Markdown => {
                                formatting::encode(&text, true)
                            }
                            AutoFormat::All => formatting::encode(&text, false),
                        };

                        Ok(Command::Irc(Irc::Me(target.to_string(), text)))
                    })
                } else {
                    Ok(unknown())
                }
            }
            Kind::Whois => {
                validated::<1, 1, false>(args, |[target], [nickname]| {
                    let target_limit = find_target_limit(isupport, "WHOIS")
                        .map(|limit| limit as usize);

                    // If both `target` and `nickname` is specified `target` should be a server.
                    // Otherwise we use `target` as nick (when `nick` is `None`).
                    let server = nickname.as_ref().map(|_| target.clone());
                    let nickname = match nickname {
                        Some(nickname) => nickname,
                        None => target,
                    };

                    if let Some(target_limit) = target_limit {
                        let nicks = nickname.split(',').collect::<Vec<_>>();

                        if nicks.len() > target_limit {
                            return Err(Error::TooManyTargets {
                                name: "nicks",
                                number: nicks.len(),
                                max_number: target_limit,
                            });
                        }
                    }

                    Ok(Command::Irc(Irc::Whois(server, nickname)))
                })
            }
            Kind::Part => {
                validated::<0, 2, true>(args, |_, [target_list, reason]| {
                    let targets = if let Some(target_list) = target_list {
                        let casemapping =
                            isupport::get_casemapping_or_default(isupport);
                        let chantypes =
                            isupport::get_chantypes_or_default(isupport);
                        let statusmsg =
                            isupport::get_statusmsg_or_default(isupport);

                        target_list
                            .split(',')
                            .map(|target| {
                                Target::parse(
                                    target,
                                    chantypes,
                                    statusmsg,
                                    casemapping,
                                )
                            })
                            .collect::<Vec<_>>()
                    } else {
                        let Some(target) = buffer.and_then(Upstream::target)
                        else {
                            // If not in a query or channel then a target is
                            // required
                            return Err(Error::IncorrectArgCount {
                                min: 1,
                                max: 2,
                                actual: 0,
                            });
                        };

                        vec![target]
                    };

                    if let Some(isupport::Parameter::CHANNELLEN(max_len)) =
                        isupport.get(&isupport::Kind::CHANNELLEN)
                    {
                        let max_len = *max_len as usize;

                        if let Some(target) = targets.iter().find(|target| {
                            target.as_channel().is_some_and(|channel| {
                                channel.as_str().len() > max_len
                            })
                        }) {
                            return Err(Error::ArgTooLong {
                                name: "channel in targets",
                                len: target.as_str().len(),
                                max_len,
                            });
                        }
                    }

                    Ok(Command::Internal(Internal::LeaveBuffers(
                        targets,
                        reason.or_else(|| {
                            config.buffer.commands.part.default_reason.clone()
                        }),
                    )))
                })
            }
            Kind::Topic => {
                validated::<0, 2, true>(args.clone(), |_, [channel, topic]| {
                    let (channel, topic) = if let Some(channel) = channel {
                        let chantypes =
                            isupport::get_chantypes_or_default(isupport);

                        if !proto::is_channel(&channel, chantypes) {
                            // Re-create topic from args in order to preserve
                            // whitespace
                            let topic = get_combined_arg(&args, 1);

                            let Some(channel) = buffer
                                .and_then(Upstream::target)
                                .and_then(Target::to_channel)
                            else {
                                return Err(Error::InvalidChannelName {
                                    requirements: fmt_channel_name_requirements(
                                        chantypes,
                                    ),
                                });
                            };

                            (channel.to_string(), topic)
                        } else {
                            (channel, topic)
                        }
                    } else {
                        let Some(channel) = buffer
                            .and_then(Upstream::target)
                            .and_then(Target::to_channel)
                        else {
                            // If not in a channel then a channel argument is
                            // required
                            return Err(Error::IncorrectArgCount {
                                min: 1,
                                max: 2,
                                actual: 0,
                            });
                        };

                        (channel.to_string(), None)
                    };

                    if let Some(ref topic) = topic
                        && let Some(isupport::Parameter::TOPICLEN(max_len)) =
                            isupport.get(&isupport::Kind::TOPICLEN)
                    {
                        let max_len = *max_len as usize;

                        if topic.len() > max_len {
                            return Err(Error::ArgTooLong {
                                name: "topic",
                                len: topic.len(),
                                max_len,
                            });
                        }
                    }

                    Ok(Command::Irc(Irc::Topic(channel, topic)))
                })
            }
            Kind::Kick => {
                validated::<1, 2, true>(
                    args.clone(),
                    |[channel], [users, comment]| {
                        let chantypes =
                            isupport::get_chantypes_or_default(isupport);

                        let (channel, users, comment) =
                            if !proto::is_channel(&channel, chantypes) {
                                let users = channel;

                                let Some(channel) = buffer
                                    .and_then(Upstream::target)
                                    .and_then(Target::to_channel)
                                else {
                                    // If not in a channel then a channel argument is
                                    // required
                                    return Err(Error::IncorrectArgCount {
                                        min: 2,
                                        max: 3,
                                        actual: 0,
                                    });
                                };

                                // Re-create comment from args in order to preserve
                                // whitespace
                                let comment = get_combined_arg(&args, 2);

                                (channel.to_string(), users, comment)
                            } else {
                                let Some(users) = users else {
                                    // If channel is not skipped then users is
                                    // required
                                    return Err(Error::IncorrectArgCount {
                                        min: 2,
                                        max: 3,
                                        actual: 0,
                                    });
                                };

                                (channel, users, comment)
                            };

                        let target_limit = find_target_limit(isupport, "KICK")
                            .map(|limit| limit as usize);

                        if let Some(target_limit) = target_limit {
                            let users = users.split(',').collect::<Vec<_>>();

                            if users.len() > target_limit {
                                return Err(Error::TooManyTargets {
                                    name: "users",
                                    number: users.len(),
                                    max_number: target_limit,
                                });
                            }
                        }

                        if let Some(ref comment) = comment
                            && let Some(isupport::Parameter::KICKLEN(max_len)) =
                                isupport.get(&isupport::Kind::KICKLEN)
                        {
                            let max_len = *max_len as usize;

                            if comment.len() > max_len {
                                return Err(Error::ArgTooLong {
                                    name: "comment",
                                    len: comment.len(),
                                    max_len,
                                });
                            }
                        }

                        Ok(Command::Irc(Irc::Kick(channel, users, comment)))
                    },
                )
            }
            Kind::Mode => validated::<0, 3, true>(
                args,
                |_, [target, mode_string, mode_arguments]| {
                    let (target, mode_string, mode_arguments) =
                        if let Some(target) = target {
                            if target.starts_with(['+', '-']) {
                                let mode_arguments =
                                    if let Some(ref mode_string) = mode_string
                                        && let Some(mode_arguments) =
                                            mode_arguments
                                    {
                                        Some(format!(
                                            "{mode_string} {mode_arguments}"
                                        ))
                                    } else {
                                        mode_string
                                    };

                                let mode_string = target;

                                (None, Some(mode_string), mode_arguments)
                            } else {
                                (Some(target), mode_string, mode_arguments)
                            }
                        } else {
                            (None, mode_string, mode_arguments)
                        };

                    let target = target.unwrap_or(
                        buffer
                            .and_then(Upstream::target)
                            .map(|buffer_target| buffer_target.to_string())
                            .unwrap_or(
                                our_nickname
                                    .ok_or(Error::IncorrectArgCount {
                                        min: 1,
                                        max: 2,
                                        actual: 0,
                                    })?
                                    .to_string(),
                            ),
                    );

                    let mode_limit =
                        isupport::get_mode_limit_or_default(isupport);

                    if let Some(mode_string) = mode_string {
                        if mode_string == "+" || mode_string == "-" {
                            Err(Error::NoModeString)
                        } else {
                            let mode_string_regex = if proto::is_channel(
                                &target,
                                isupport::get_chantypes_or_default(isupport),
                            ) {
                                let chanmodes =
                                    isupport::get_chanmodes_or_default(
                                        isupport,
                                    );
                                let prefix =
                                    isupport::get_prefix_or_default(isupport);

                                let mut channel_modes_regex =
                                    String::from(r"^((\+|\-)[");
                                for chanmode in chanmodes {
                                    channel_modes_regex +=
                                        chanmode.modes.as_ref();
                                }
                                for prefix_map in prefix {
                                    channel_modes_regex.push(prefix_map.mode);
                                }
                                channel_modes_regex += r"]";
                                if let Some(mode_limit) = mode_limit {
                                    channel_modes_regex += r"{1,";
                                    channel_modes_regex +=
                                        &format!("{mode_limit}");
                                    channel_modes_regex += r"}";
                                } else {
                                    channel_modes_regex += r"+";
                                }
                                channel_modes_regex += r")+$";

                                Regex::new(&channel_modes_regex).unwrap_or(
                                    Regex::new(r"^((\+|\-)[A-Za-z]+)+$")
                                        .unwrap(),
                                )
                            } else {
                                // User modes from RPL_MYINFO is unreliable,
                                // so use the most permissive regex instead of
                                // crafting a regex for the server

                                let mut user_modes_regex =
                                    String::from(r"^((\+|\-)[A-Za-z]");
                                if let Some(mode_limit) = mode_limit {
                                    user_modes_regex += r"{1,";
                                    user_modes_regex +=
                                        &format!("{mode_limit}");
                                    user_modes_regex += r"}";
                                } else {
                                    user_modes_regex += r"+";
                                }
                                user_modes_regex += r")+$";
                                Regex::new(&user_modes_regex).unwrap_or(
                                    Regex::new(r"^((\+|\-)[A-Za-z]+)+$")
                                        .unwrap(),
                                )
                            };

                            if !mode_string_regex
                                .is_match(&mode_string)
                                .unwrap_or_default()
                            {
                                Err(Error::InvalidModeString)
                            } else {
                                let mode_arguments =
                                    mode_arguments.map(|mode_arguments| {
                                        mode_arguments
                                            .split_ascii_whitespace()
                                            .map(String::from)
                                            .collect()
                                    });

                                Ok(Command::Irc(Irc::Mode(
                                    target.to_string(),
                                    Some(mode_string.to_string()),
                                    mode_arguments,
                                )))
                            }
                        }
                    } else {
                        Ok(Command::Irc(Irc::Mode(
                            target.to_string(),
                            None,
                            None,
                        )))
                    }
                },
            ),
            Kind::Away => validated::<0, 1, true>(args, |_, [comment]| {
                if let Some(ref comment) = comment
                    && let Some(isupport::Parameter::AWAYLEN(max_len)) =
                        isupport.get(&isupport::Kind::AWAYLEN)
                {
                    let max_len = *max_len as usize;

                    if comment.len() > max_len {
                        return Err(Error::ArgTooLong {
                            name: "reason",
                            len: comment.len(),
                            max_len,
                        });
                    }
                }

                Ok(Command::Irc(Irc::Away(comment)))
            }),
            Kind::SetName => {
                if let Some(isupport::Parameter::NAMELEN(max_len)) =
                    isupport.get(&isupport::Kind::NAMELEN)
                {
                    validated::<1, 0, true>(args, |[realname], _| {
                        let max_len = *max_len as usize;

                        if realname.len() > max_len {
                            return Err(Error::ArgTooLong {
                                name: "realname",
                                len: realname.len(),
                                max_len,
                            });
                        }

                        Ok(Command::Irc(Irc::SetName(realname)))
                    })
                } else {
                    Err(Error::CommandNotAvailable {
                        command: "setname",
                        context: buffer.map_or(String::new(), |buffer| {
                            format!(" on {}", buffer.server())
                        }),
                    })
                }
            }
            Kind::Notice | Kind::FormatNotice | Kind::PlainNotice => {
                validated::<1, 1, true>(args, |[targets], [msg]| {
                    let target_limit = find_target_limit(isupport, "NOTICE")
                        .map(|limit| limit as usize);

                    if let Some(target_limit) = target_limit {
                        let targets = targets.split(',').collect::<Vec<_>>();

                        if targets.len() > target_limit {
                            return Err(Error::TooManyTargets {
                                name: "targets",
                                number: targets.len(),
                                max_number: target_limit,
                            });
                        }
                    }

                    if let Some(msg) = msg {
                        let msg = match kind {
                            Kind::FormatNotice => {
                                formatting::encode(&msg, false)
                            }
                            Kind::PlainNotice => msg,
                            _ => match auto_format {
                                AutoFormat::Disabled => msg,
                                AutoFormat::Markdown => {
                                    formatting::encode(&msg, true)
                                }
                                AutoFormat::All => {
                                    formatting::encode(&msg, false)
                                }
                            },
                        };

                        Ok(Command::Irc(Irc::Notice(targets, msg)))
                    } else {
                        let casemapping =
                            isupport::get_casemapping_or_default(isupport);
                        let chantypes =
                            isupport::get_chantypes_or_default(isupport);
                        let statusmsg =
                            isupport::get_statusmsg_or_default(isupport);

                        Ok(Command::Internal(Internal::OpenBuffers(
                            targets
                                .split(",")
                                .map(|target| {
                                    Target::parse(
                                        target,
                                        chantypes,
                                        statusmsg,
                                        casemapping,
                                    )
                                })
                                .collect(),
                        )))
                    }
                })
            }
            Kind::Raw => Ok(Command::Irc(Irc::Raw(raw.to_string()))),
            Kind::Format => {
                if let Some(target) = buffer.and_then(Upstream::target) {
                    Ok(Command::Irc(Irc::Msg(
                        target.to_string(),
                        formatting::encode(raw, false),
                    )))
                } else {
                    Ok(unknown())
                }
            }
            Kind::FormatMe => {
                if let Some(target) = buffer.and_then(Upstream::target) {
                    Ok(Command::Irc(Irc::Me(
                        target.to_string(),
                        formatting::encode(raw, false),
                    )))
                } else {
                    Ok(unknown())
                }
            }
            Kind::Plain => {
                if let Some(target) = buffer.and_then(Upstream::target) {
                    Ok(Command::Irc(Irc::Msg(
                        target.to_string(),
                        raw.to_string(),
                    )))
                } else {
                    Ok(unknown())
                }
            }
            Kind::PlainMe => {
                if let Some(target) = buffer.and_then(Upstream::target) {
                    Ok(Command::Irc(Irc::Me(
                        target.to_string(),
                        raw.to_string(),
                    )))
                } else {
                    Ok(unknown())
                }
            }
            Kind::Ctcp => {
                validated::<1, 2, true>(
                    args.clone(),
                    |[target], [command, params]| {
                        let (target, command, params) = if let Some(query) =
                            buffer
                                .and_then(Upstream::target)
                                .and_then(Target::to_query)
                            && matches!(
                                target.to_uppercase().as_str(),
                                "ACTION"
                                    | "CLIENTINFO"
                                    | "USERINFO"
                                    | "PING"
                                    | "SOURCE"
                                    | "TIME"
                                    | "VERSION"
                            ) {
                            // Re-create comment from args in order to preserve
                            // whitespace
                            let params = get_combined_arg(&args, 2);

                            (query.to_string(), target, params)
                        } else {
                            let Some(command) = command else {
                                // If target is not skipped then command is required
                                return Err(Error::IncorrectArgCount {
                                    min: 2,
                                    max: 3,
                                    actual: 0,
                                });
                            };

                            (target, command, params)
                        };

                        Ok(Command::Irc(Irc::Ctcp(
                            ctcp::Command::from(command.as_str()),
                            target,
                            params,
                        )))
                    },
                )
            }
            Kind::Chathistory => {
                if !capabilities.acknowledged(Capability::Chathistory) {
                    return Err(Error::CommandNotAvailable {
                        command: "chathistory",
                        context: buffer.map_or(String::new(), |buffer| {
                            format!(" on {}", buffer.server())
                        }),
                    });
                }

                validated::<1, 4, false>(args, |[subcommand], params| {
                    let maximum_limit = if let Some(
                        isupport::Parameter::CHATHISTORY(maximum_limit),
                    ) =
                        isupport.get(&isupport::Kind::CHATHISTORY)
                    {
                        Some(maximum_limit)
                    } else {
                        None
                    };

                    let subcommand = subcommand.to_uppercase();

                    match subcommand.as_str() {
                        "BEFORE" | "AFTER" | "AROUND" => {
                            if let [
                                Some(target),
                                Some(message_reference),
                                Some(limit),
                                None,
                            ] = params
                            {
                                let Some(message_reference) =
                                    validated_message_reference(
                                        &message_reference,
                                        false,
                                    )
                                else {
                                    return Err(Error::InvalidChathistoryMessageReference);
                                };

                                if let Ok(limit) = limit.parse::<u16>()
                                    && limit > 0
                                {
                                    if let Some(maximum_limit) = maximum_limit
                                        && limit > *maximum_limit
                                    {
                                        return Err(
                                            Error::ChathistoryLimitTooLarge {
                                                maximum_limit: *maximum_limit,
                                            },
                                        );
                                    }
                                } else {
                                    return Err(Error::NotPositiveInteger);
                                }

                                Ok(Command::Irc(Irc::Chathistory(
                                    subcommand,
                                    vec![
                                        target,
                                        message_reference.to_string(),
                                        limit,
                                    ],
                                )))
                            } else {
                                Err(Error::IncorrectArgCount {
                                    min: 4,
                                    max: 4,
                                    actual: params
                                        .into_iter()
                                        .filter(Option::is_some)
                                        .count(),
                                })
                            }
                        }
                        "LATEST" => {
                            if let [
                                Some(target),
                                Some(message_reference),
                                Some(limit),
                                None,
                            ] = params
                            {
                                let Some(message_reference) =
                                    validated_message_reference(
                                        &message_reference,
                                        true,
                                    )
                                else {
                                    return Err(Error::InvalidChathistoryMessageReference);
                                };

                                if let Ok(limit) = limit.parse::<u16>()
                                    && limit > 0
                                {
                                    if let Some(maximum_limit) = maximum_limit
                                        && limit > *maximum_limit
                                    {
                                        return Err(
                                            Error::ChathistoryLimitTooLarge {
                                                maximum_limit: *maximum_limit,
                                            },
                                        );
                                    }
                                } else {
                                    return Err(Error::NotPositiveInteger);
                                }

                                Ok(Command::Irc(Irc::Chathistory(
                                    subcommand,
                                    vec![
                                        target,
                                        message_reference.to_string(),
                                        limit,
                                    ],
                                )))
                            } else {
                                Err(Error::IncorrectArgCount {
                                    min: 4,
                                    max: 4,
                                    actual: params
                                        .into_iter()
                                        .filter(Option::is_some)
                                        .count(),
                                })
                            }
                        }
                        "BETWEEN" => {
                            if let [
                                Some(target),
                                Some(first_message_reference),
                                Some(second_message_reference),
                                Some(limit),
                            ] = params
                            {
                                let Some(first_message_reference) =
                                    validated_message_reference(
                                        &first_message_reference,
                                        false,
                                    )
                                else {
                                    return Err(Error::InvalidChathistoryMessageReference);
                                };

                                let Some(second_message_reference) =
                                    validated_message_reference(
                                        &second_message_reference,
                                        false,
                                    )
                                else {
                                    return Err(Error::InvalidChathistoryMessageReference);
                                };

                                if let Ok(limit) = limit.parse::<u16>()
                                    && limit > 0
                                {
                                    if let Some(maximum_limit) = maximum_limit
                                        && limit > *maximum_limit
                                    {
                                        return Err(
                                            Error::ChathistoryLimitTooLarge {
                                                maximum_limit: *maximum_limit,
                                            },
                                        );
                                    }
                                } else {
                                    return Err(Error::NotPositiveInteger);
                                }

                                Ok(Command::Irc(Irc::Chathistory(
                                    subcommand,
                                    vec![
                                        target,
                                        first_message_reference.to_string(),
                                        second_message_reference.to_string(),
                                        limit,
                                    ],
                                )))
                            } else {
                                Err(Error::IncorrectArgCount {
                                    min: 5,
                                    max: 5,
                                    actual: params
                                        .into_iter()
                                        .filter(Option::is_some)
                                        .count(),
                                })
                            }
                        }
                        "TARGETS" => {
                            if let [
                                Some(first_timestamp),
                                Some(second_timestamp),
                                Some(limit),
                                None,
                            ] = params
                            {
                                let Some(first_timestamp) =
                                    validated_timestamp(&first_timestamp).map(
                                        isupport::MessageReference::Timestamp,
                                    )
                                else {
                                    return Err(
                                        Error::InvalidChathistoryTimestamp,
                                    );
                                };

                                let Some(second_timestamp) =
                                    validated_timestamp(&second_timestamp).map(
                                        isupport::MessageReference::Timestamp,
                                    )
                                else {
                                    return Err(
                                        Error::InvalidChathistoryTimestamp,
                                    );
                                };

                                if let Ok(limit) = limit.parse::<u16>()
                                    && limit > 0
                                {
                                    if let Some(maximum_limit) = maximum_limit
                                        && limit > *maximum_limit
                                    {
                                        return Err(
                                            Error::ChathistoryLimitTooLarge {
                                                maximum_limit: *maximum_limit,
                                            },
                                        );
                                    }
                                } else {
                                    return Err(Error::NotPositiveInteger);
                                }

                                Ok(Command::Irc(Irc::Chathistory(
                                    subcommand,
                                    vec![
                                        first_timestamp.to_string(),
                                        second_timestamp.to_string(),
                                        limit,
                                    ],
                                )))
                            } else {
                                Err(Error::IncorrectArgCount {
                                    min: 4,
                                    max: 4,
                                    actual: params
                                        .into_iter()
                                        .filter(Option::is_some)
                                        .count(),
                                })
                            }
                        }
                        _ => Err(Error::InvalidSubcommand {
                            command: "chathistory",
                            is_partial_valid: [
                                "BEFORE", "AFTER", "AROUND", "LATEST",
                                "BETWEEN", "TARGETS",
                            ]
                            .iter()
                            .any(|valid_subcommand| {
                                valid_subcommand.starts_with(&subcommand)
                            }),
                        }),
                    }
                })
            }
            Kind::Monitor => {
                if let Some(isupport::Parameter::MONITOR(target_limit)) =
                    isupport.get(&isupport::Kind::MONITOR)
                {
                    validated::<1, 1, false>(args, |[subcommand], [targets]| {
                        let target_limit = target_limit
                            .map(|target_limit| target_limit as usize);

                        let subcommand = subcommand.to_uppercase();

                        match subcommand.as_str() {
                            "+" | "-" => {
                                if let Some(targets) = targets {
                                    if let Some(target_limit) = target_limit {
                                        let targets = targets
                                            .split(',')
                                            .collect::<Vec<_>>();

                                        if targets.len() > target_limit {
                                            return Err(
                                                Error::TooManyTargets {
                                                    name: "targets",
                                                    number: targets.len(),
                                                    max_number: target_limit,
                                                },
                                            );
                                        }
                                    }

                                    Ok(Command::Irc(Irc::Monitor(
                                        subcommand,
                                        Some(targets),
                                    )))
                                } else {
                                    Err(Error::IncorrectArgCount {
                                        min: 2,
                                        max: 2,
                                        actual: 1,
                                    })
                                }
                            }
                            "C" | "L" | "S" => {
                                if targets.is_none() {
                                    Ok(Command::Irc(Irc::Monitor(
                                        subcommand, None,
                                    )))
                                } else {
                                    Err(Error::IncorrectArgCount {
                                        min: 1,
                                        max: 1,
                                        actual: 2,
                                    })
                                }
                            }
                            _ => Err(Error::InvalidSubcommand {
                                command: "monitor",
                                is_partial_valid: false,
                            }),
                        }
                    })
                } else {
                    Err(Error::CommandNotAvailable {
                        command: "monitor",
                        context: buffer.map_or(String::new(), |buffer| {
                            format!(" on {}", buffer.server())
                        }),
                    })
                }
            }
            Kind::Invite => {
                validated::<1, 1, true>(args, |[nickname], [channel]| {
                    if let Some(channel) = channel {
                        Ok(Command::Irc(Irc::Invite(nickname, channel)))
                    } else if let Some(channel) = buffer
                        .and_then(Upstream::target)
                        .and_then(Target::to_channel)
                    {
                        Ok(Command::Irc(Irc::Invite(
                            nickname,
                            channel.to_string(),
                        )))
                    } else {
                        // If not in a channel then a channel argument is
                        // required
                        Err(Error::IncorrectArgCount {
                            min: 2,
                            max: 2,
                            actual: 0,
                        })
                    }
                })
            }
            Kind::Hop => {
                validated::<0, 2, true>(args, |_, [channel, message]| {
                    Ok(Command::Internal(Internal::Hop(channel, message)))
                })
            }
            Kind::Clear => validated::<0, 0, false>(args, |_, _| {
                Ok(Command::Internal(Internal::ClearBuffer))
            }),
            Kind::List => validated::<0, 0, false>(args, |_, _| {
                Ok(Command::Internal(Internal::ChannelDiscovery))
            }),
            Kind::SysInfo => validated::<0, 0, false>(args, |_, _| {
                Ok(Command::Internal(Internal::SysInfo))
            }),
            Kind::Detach => {
                if !supports_detach {
                    return Err(Error::CommandNotAvailable {
                        command: "detach",
                        context: buffer.map_or(String::new(), |buffer| {
                            format!(" on {}", buffer.server())
                        }),
                    });
                }

                validated::<0, 1, false>(args, |_, [target_list]| {
                    let channels = if let Some(target_list) = target_list {
                        let casemapping =
                            isupport::get_casemapping_or_default(isupport);
                        let chantypes =
                            isupport::get_chantypes_or_default(isupport);
                        let statusmsg =
                            isupport::get_statusmsg_or_default(isupport);

                        let Ok(channels) = target_list
                            .split(',')
                            .map(|target| {
                                target::Channel::parse(
                                    target,
                                    chantypes,
                                    statusmsg,
                                    casemapping,
                                )
                            })
                            .try_collect()
                        else {
                            return Err(Error::InvalidChannelName {
                                requirements: fmt_channel_name_requirements(
                                    chantypes,
                                ),
                            });
                        };

                        channels
                    } else {
                        let Some(channel) = buffer
                            .and_then(Upstream::target)
                            .and_then(Target::to_channel)
                        else {
                            // If not in a channel then a channel argument is
                            // required
                            return Err(Error::IncorrectArgCount {
                                min: 1,
                                max: 1,
                                actual: 0,
                            });
                        };

                        vec![channel]
                    };

                    if let Some(isupport::Parameter::CHANNELLEN(max_len)) =
                        isupport.get(&isupport::Kind::CHANNELLEN)
                    {
                        let max_len = *max_len as usize;

                        if let Some(channel) = channels
                            .iter()
                            .find(|channel| channel.as_str().len() > max_len)
                        {
                            return Err(Error::ArgTooLong {
                                name: "channel in channels",
                                len: channel.as_str().len(),
                                max_len,
                            });
                        }
                    }

                    Ok(Command::Internal(Internal::Detach(channels)))
                })
            }
            Kind::ClearTopic => {
                validated::<0, 1, false>(args, |_, [channel]| {
                    if let Some(channel) = channel {
                        let chantypes =
                            isupport::get_chantypes_or_default(isupport);

                        if proto::is_channel(&channel, chantypes) {
                            Ok(Command::Irc(Irc::Topic(
                                channel.to_string(),
                                Some(String::new()),
                            )))
                        } else {
                            Err(Error::InvalidChannelName {
                                requirements: fmt_channel_name_requirements(
                                    chantypes,
                                ),
                            })
                        }
                    } else if let Some(channel) = buffer
                        .and_then(Upstream::target)
                        .and_then(Target::to_channel)
                    {
                        Ok(Command::Irc(Irc::Topic(
                            channel.to_string(),
                            Some(String::new()),
                        )))
                    } else {
                        // If not in a channel then a channel argument is
                        // required
                        Err(Error::IncorrectArgCount {
                            min: 1,
                            max: 1,
                            actual: 0,
                        })
                    }
                })
            }
            Kind::Delay => validated::<1, 0, false>(args, |[seconds], _| {
                if let Ok(seconds) = seconds.parse::<u64>()
                    && seconds > 0
                {
                    Ok(Command::Internal(Internal::Delay(seconds)))
                } else {
                    Err(Error::NotPositiveInteger)
                }
            }),
            Kind::Connect => validated::<0, 1, false>(args, |_, [server]| {
                if let Some(server) = server {
                    if let Ok(url) = Url::from_str(&server)
                        && matches!(url, Url::ServerConnect { .. })
                    {
                        Ok(Command::Internal(Internal::Connect(server)))
                    } else {
                        Err(Error::InvalidServerUrl)
                    }
                } else if is_connected {
                    // If not connected then a server is required
                    Err(Error::IncorrectArgCount {
                        min: 1,
                        max: 1,
                        actual: 0,
                    })
                } else {
                    Ok(Command::Internal(Internal::Reconnect))
                }
            }),
            Kind::Reconnect => validated::<0, 0, false>(args, |_, _| {
                Ok(Command::Internal(Internal::Reconnect))
            }),
            Kind::Upload => validated::<1, 0, true>(args, |[path], _| {
                Ok(Command::Internal(Internal::Upload(path)))
            }),
            Kind::Exec => {
                let command = raw.trim();

                if !config.buffer.commands.exec.enabled {
                    Err(Error::ExecDisabled)
                } else if command.is_empty() {
                    Err(Error::IncorrectArgCount {
                        min: 1,
                        max: 1,
                        actual: 0,
                    })
                } else {
                    Ok(Command::Internal(Internal::Exec(command.to_string())))
                }
            }
        },
        Err(()) => Ok(unknown()),
    }
}

// TODO: Expand `validated` so we can better indicate which parameters is optional.
fn validated<const EXACT: usize, const OPT: usize, const TEXT: bool>(
    args: Vec<&str>,
    f: impl FnOnce([String; EXACT], [Option<String>; OPT]) -> Result<Command, Error>,
) -> Result<Command, Error> {
    let max = EXACT + OPT;

    let args: Vec<String> = if TEXT {
        let combined_arg = get_combined_arg(&args, max);

        args.into_iter()
            .filter(|arg| !arg.is_empty())
            .take(max.saturating_sub(1))
            .map(ToString::to_string)
            .chain(combined_arg)
            .collect()
    } else {
        args.into_iter()
            .filter(|arg| !arg.is_empty())
            .map(String::from)
            .collect()
    };

    if args.len() >= EXACT && args.len() <= max {
        let exact = args[0..EXACT].to_vec().try_into().unwrap();
        let opt = args[EXACT..args.len()]
            .iter()
            .map(|s| Some(s.clone()))
            .chain((args.len()..max).map(|_| None))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        (f)(exact, opt)
    } else {
        Err(Error::IncorrectArgCount {
            min: EXACT,
            max,
            actual: args.len(),
        })
    }
}

fn get_combined_arg(
    args: &Vec<&str>,
    combined_arg_number: usize,
) -> Option<String> {
    // Combined arg is always the last arg
    let skip_args_count = if combined_arg_number > 1 {
        args.iter()
            .enumerate()
            .filter_map(|(position, arg)| (!arg.is_empty()).then_some(position))
            .nth(combined_arg_number.saturating_sub(2))
            .map(|position| position.saturating_add(1))
    } else {
        Some(0)
    };

    // Combine everything after the penultimate arg
    let combined_arg = skip_args_count
        .map(|count| args.iter().skip(count).join(" "))
        .unwrap_or_default();

    (!combined_arg.is_empty()).then_some(combined_arg)
}

fn validated_timestamp(timestamp: &str) -> Option<DateTime<Utc>> {
    // Allow no timestamp= prefix if we can parse the remainder as a timestamp.
    let timestamp = timestamp.strip_prefix("timestamp=").unwrap_or(timestamp);

    // Parse full timestamp format
    if let Ok(date_time) = DateTime::parse_from_rfc3339(timestamp)
        .map(|date_time| date_time.to_utc())
    {
        return Some(date_time);
    }

    // Allow omitted offset, by assuming it's the local timezone.  Also, per
    // Chrono crate's documentation, "missing seconds are assumed to be zero".
    if let Some(date_time) =
        NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%dT%H:%M:%S%.f")
            .ok()
            .and_then(|naive_date_time| {
                naive_date_time.and_local_timezone(Local).single()
            })
            .map(|date_time| date_time.to_utc())
    {
        return Some(date_time);
    }

    // Allow omitted seconds, which will be assumed to be zero
    if let Some(date_time) = NaiveDateTime::parse_from_str(
        &format!("{timestamp}:00"),
        "%Y-%m-%dT%H:%M:%S",
    )
    .ok()
    .and_then(|naive_date_time| {
        naive_date_time.and_local_timezone(Local).single()
    })
    .map(|date_time| date_time.to_utc())
    {
        return Some(date_time);
    }

    // Allow omitted minutes, which will be assumed to be zero
    if let Some(date_time) = NaiveDateTime::parse_from_str(
        &format!("{timestamp}:00"),
        "%Y-%m-%dT%H:%M",
    )
    .ok()
    .and_then(|naive_date_time| {
        naive_date_time.and_local_timezone(Local).single()
    })
    .map(|date_time| date_time.to_utc())
    {
        return Some(date_time);
    }

    // Allow omitted hours, which will be assumed to be zero (midnight)
    NaiveDate::parse_from_str(timestamp, "%Y-%m-%d")
        .ok()
        .and_then(|naive_date| naive_date.and_hms_opt(0, 0, 0))
        .and_then(|naive_date_time| {
            naive_date_time.and_local_timezone(Local).single()
        })
        .map(|date_time| date_time.to_utc())
}

fn validated_message_reference(
    message_reference: &str,
    allow_none: bool,
) -> Option<isupport::MessageReference> {
    if let Some(date_time) = validated_timestamp(message_reference) {
        return Some(isupport::MessageReference::Timestamp(date_time));
    }

    if let Some(message_id) = message_reference.strip_prefix("msgid=") {
        return Some(isupport::MessageReference::MessageId(message_id.into()));
    }

    (allow_none && message_reference == "*")
        .then_some(isupport::MessageReference::None)
}

impl TryFrom<Irc> for proto::Command {
    type Error = ();

    fn try_from(command: Irc) -> Result<Self, Self::Error> {
        Ok(match command {
            Irc::Join(chanlist, chankeys) => {
                proto::Command::JOIN(chanlist, chankeys)
            }
            Irc::Motd(target) => proto::Command::MOTD(target),
            Irc::Nick(nick) => proto::Command::NICK(nick),
            Irc::Quit(comment) => proto::Command::QUIT(comment),
            Irc::Msg(target, msg) => proto::Command::PRIVMSG(target, msg),
            Irc::React { target, .. } => proto::Command::TAGMSG(target),
            Irc::Unreact { target, .. } => proto::Command::TAGMSG(target),
            Irc::Me(target, text) => {
                ctcp::query_command(&ctcp::Command::Action, target, Some(text))
            }
            Irc::Whois(channel, user) => proto::Command::WHOIS(channel, user),
            Irc::Whowas(user, count) => proto::Command::WHOWAS(user, count),
            Irc::Part(chanlist, reason) => {
                proto::Command::PART(chanlist, reason)
            }
            Irc::Topic(channel, topic) => proto::Command::TOPIC(channel, topic),
            Irc::Kick(channel, user, comment) => {
                proto::Command::KICK(channel, user, comment)
            }
            Irc::Mode(target, modestring, modearguments) => {
                proto::Command::MODE(target, modestring, modearguments)
            }
            Irc::Away(comment) => proto::Command::AWAY(comment),
            Irc::SetName(realname) => proto::Command::SETNAME(realname),
            Irc::Notice(target, msg) => proto::Command::NOTICE(target, msg),
            Irc::Typing { target, .. } => proto::Command::TAGMSG(target),
            Irc::Raw(raw) => proto::Command::Raw(raw),
            Irc::Unknown(command, args) => proto::Command::new(&command, args),
            Irc::Ctcp(command, target, params) => {
                ctcp::query_command(&command, target, params)
            }
            Irc::Chathistory(subcommand, params) => {
                proto::Command::CHATHISTORY(subcommand, params)
            }
            Irc::List(channels, elistcond) => {
                proto::Command::LIST(channels, elistcond)
            }
            Irc::Monitor(subcommand, targets) => {
                proto::Command::MONITOR(subcommand, targets)
            }
            Irc::Invite(nickname, channel) => {
                proto::Command::INVITE(nickname, channel)
            }
        })
    }
}
impl TryFrom<Irc> for proto::Message {
    type Error = ();

    fn try_from(command: Irc) -> Result<Self, Self::Error> {
        let tags = match &command {
            Irc::React { msgid, text, .. } => tags![
                "+reply" => msgid.to_string(),
                "+draft/reply" => msgid.to_string(),
                "+draft/react" => text.as_ref(),
            ],
            Irc::Unreact { msgid, text, .. } => tags![
                "+reply" => msgid.to_string(),
                "+draft/reply" => msgid.to_string(),
                "+draft/unreact" => text.as_ref(),
            ],
            Irc::Typing { value, .. } => tags!["+typing" => value.as_str()],
            _ => tags![],
        };
        let mut msg = proto::Message::from(proto::Command::try_from(command)?);
        msg.tags.extend(tags);
        Ok(msg)
    }
}
impl TryFrom<Irc> for message::Encoded {
    type Error = ();

    fn try_from(command: Irc) -> Result<Self, Self::Error> {
        Ok(message::Encoded::from(proto::Message::try_from(command)?))
    }
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Error {
    #[error("{}", fmt_incorrect_arg_count(*min, *max, *actual))]
    IncorrectArgCount {
        min: usize,
        max: usize,
        actual: usize,
    },
    #[error("missing slash")]
    MissingSlash,
    #[error("missing command")]
    MissingCommand,
    #[error("no modes in modestring")]
    NoModeString,
    #[error("invalid modestring")]
    InvalidModeString,
    #[error("{name} is too long ({len}/{max_len} characters)")]
    ArgTooLong {
        name: &'static str,
        len: usize,
        max_len: usize,
    },
    #[error("too many {name} ({number}/{max_number} allowed)")]
    TooManyTargets {
        name: &'static str,
        number: usize,
        max_number: usize,
    },
    #[error("must be a number greater than zero")]
    NotPositiveInteger,
    #[error("invalid channel name ({requirements}")]
    InvalidChannelName { requirements: String },
    #[error("invalid server url")]
    InvalidServerUrl,
    #[error("not connected to server")]
    Disconnected,
    #[error("already connected to server")]
    Connected,
    #[error("not in channel")]
    NotInChannel,
    #[error("invalid {command} subcommand")]
    InvalidSubcommand {
        command: &'static str,
        is_partial_valid: bool,
    },
    #[error("invalid timestamp or message id")]
    InvalidChathistoryMessageReference,
    #[error("invalid timestamp")]
    InvalidChathistoryTimestamp,
    #[error("too large (maximum limit: {maximum_limit})")]
    ChathistoryLimitTooLarge { maximum_limit: u16 },
    #[error("exec is not enabled by the user")]
    ExecDisabled,
    #[error("/{command} is not available{context}")]
    CommandNotAvailable {
        command: &'static str,
        context: String,
    },
}

fn fmt_incorrect_arg_count(min: usize, max: usize, actual: usize) -> String {
    let relational = if actual < min { "few" } else { "many" };

    if min == max {
        format!(
            "too {relational} arguments ({actual} provided, {min} expected)"
        )
    } else {
        format!(
            "too {relational} arguments ({actual} provided, {min} to {max} expected)"
        )
    }
}

fn fmt_channel_name_requirements(chantypes: &[char]) -> String {
    let mut requirements = String::from("must start with ");

    for (index, chantype) in chantypes.iter().enumerate() {
        if index == 1 {
            requirements.push_str(&format!("'{chantype}'"));
        } else if index == chantypes.len() {
            if chantypes.len() == 2 {
                requirements.push_str(&format!(" or '{chantype}'"));
            } else {
                requirements.push_str(&format!(", or '{chantype}'"));
            }
        } else {
            requirements.push_str(&format!(", '{chantype}'"));
        }
    }

    requirements.push_str(" and cannot contain a ',' or '^G'");

    requirements
}

#[cfg(test)]
mod tests {
    use super::{AutoFormat, Command, Error, Internal, isupport, parse};
    use crate::Config;
    use crate::capabilities::Capabilities;

    #[test]
    fn parse_exec_preserves_raw_command() {
        let mut config = Config::default();
        config.buffer.commands.exec.enabled = true;

        let command = parse(
            "/exec printf '/me hello world'",
            None,
            None,
            AutoFormat::default(),
            true,
            &isupport::DEFAULT,
            &Capabilities::default(),
            false,
            &config,
        )
        .unwrap();

        assert!(matches!(
            command,
            Command::Internal(Internal::Exec(command))
                if command == "printf '/me hello world'"
        ));
    }

    #[test]
    fn parse_exec_requires_command() {
        let mut config = Config::default();
        config.buffer.commands.exec.enabled = true;

        let error = parse(
            "/exec   ",
            None,
            None,
            AutoFormat::default(),
            true,
            &isupport::DEFAULT,
            &Capabilities::default(),
            false,
            &config,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            Error::IncorrectArgCount {
                min: 1,
                max: 1,
                actual: 0
            }
        ));
    }

    #[test]
    fn parse_exec_when_disabled() {
        let mut config = Config::default();
        config.buffer.commands.exec.enabled = false;

        let error = parse(
            "/exec echo hi",
            None,
            None,
            AutoFormat::default(),
            true,
            &isupport::DEFAULT,
            &Capabilities::default(),
            false,
            &config,
        )
        .unwrap_err();

        assert!(matches!(error, Error::ExecDisabled));
    }
}
