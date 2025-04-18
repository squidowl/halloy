use std::collections::HashMap;
use std::str::FromStr;

use fancy_regex::Regex;
use irc::proto;
use itertools::Itertools;

use crate::isupport::{self, find_target_limit};
use crate::message::formatting;
use crate::{buffer::{self, Upstream}, ctcp};

#[derive(Debug, Clone)]
pub enum Command {
    Internal(Internal),
    Irc(Irc),
}

#[derive(Debug, Clone)]
pub enum Internal {
    OpenBuffers(String),
}

#[derive(Debug, Clone)]
pub enum Irc {
    Join(String, Option<String>),
    Motd(Option<String>),
    Nick(String),
    Quit(Option<String>),
    Msg(String, String),
    Me(String, String),
    Whois(Option<String>, String),
    Part(String, Option<String>),
    Topic(String, Option<String>),
    Kick(String, String, Option<String>),
    Mode(String, Option<String>, Option<Vec<String>>),
    Away(Option<String>),
    SetName(String),
    Notice(String, String),
    Raw(String),
    Unknown(String, Vec<String>),
}

#[derive(Debug, Clone, Copy)]
enum Kind {
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
    Away,
    SetName,
    Notice,
    Raw,
}

impl FromStr for Kind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "join" | "j" => Ok(Kind::Join),
            "motd" => Ok(Kind::Motd),
            "nick" => Ok(Kind::Nick),
            "quit" => Ok(Kind::Quit),
            "msg" => Ok(Kind::Msg),
            "me" | "describe" => Ok(Kind::Me),
            "whois" => Ok(Kind::Whois),
            "part" | "leave" => Ok(Kind::Part),
            "topic" | "t" => Ok(Kind::Topic),
            "kick" => Ok(Kind::Kick),
            "mode" | "m" => Ok(Kind::Mode),
            "format" | "f" => Ok(Kind::Format),
            "away" => Ok(Kind::Away),
            "setname" => Ok(Kind::SetName),
            "notice" => Ok(Kind::Notice),
            "raw" => Ok(Kind::Raw),
            _ => Err(()),
        }
    }
}

pub fn parse(
    s: &str,
    buffer: Option<&buffer::Upstream>,
    isupport: &HashMap<isupport::Kind, isupport::Parameter>,
) -> Result<Command, Error> {
    let (head, rest) = s.split_once('/').ok_or(Error::MissingSlash)?;
    // Don't allow leading whitespace before slash
    if !head.is_empty() {
        return Err(Error::MissingSlash);
    }

    let mut split = rest.split_ascii_whitespace();

    let cmd = split.next().ok_or(Error::MissingCommand)?;

    let args = split.collect::<Vec<_>>();
    let raw = if rest.len() == cmd.len() {
        ""
    } else {
        &rest[cmd.len() + 1..]
    };

    let unknown = || {
        Command::Irc(Irc::Unknown(
            cmd.to_string(),
            args.iter().map(ToString::to_string).collect(),
        ))
    };

    match cmd.parse::<Kind>() {
        Ok(kind) => match kind {
            Kind::Join => {
                validated::<1, 1, false>(args, |[chanlist], [chankeys]| {
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

                        if let Some(max_len) = channel_len {
                            if let Some(channel) = channels
                                .into_iter()
                                .find(|channel| channel.len() > max_len)
                            {
                                return Err(Error::ArgTooLong {
                                    name: "channel in chanlist",
                                    len: channel.len(),
                                    max_len,
                                });
                            }
                        }
                    }

                    if let Some(ref chankeys) = chankeys {
                        if let Some(isupport::Parameter::KEYLEN(max_len)) =
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
                Ok(Command::Irc(Irc::Quit(comment)))
            }),
            Kind::Msg => validated::<1, 1, true>(args, |[targets], [msg]| {
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
                    Ok(Command::Irc(Irc::Msg(targets, msg)))
                } else {
                    Ok(Command::Internal(Internal::OpenBuffers(targets)))
                }
            }),
            Kind::Me => {
                if let Some(target) = buffer.and_then(Upstream::target) {
                    validated::<1, 0, true>(args, |[text], _| {
                        Ok(Command::Irc(Irc::Me(target.to_string(), text)))
                    })
                } else {
                    Ok(unknown())
                }
            }
            Kind::Whois => validated::<1, 0, false>(args, |[nicks], _| {
                let target_limit = find_target_limit(isupport, "WHOIS")
                    .map(|limit| limit as usize);

                if let Some(target_limit) = target_limit {
                    let nicks = nicks.split(',').collect::<Vec<_>>();

                    if nicks.len() > target_limit {
                        return Err(Error::TooManyTargets {
                            name: "nicks",
                            number: nicks.len(),
                            max_number: target_limit,
                        });
                    }
                }

                // Leaving out optional [server] for now.
                Ok(Command::Irc(Irc::Whois(None, nicks)))
            }),
            Kind::Part => {
                validated::<1, 1, true>(args, |[chanlist], [reason]| {
                    if let Some(isupport::Parameter::CHANNELLEN(max_len)) =
                        isupport.get(&isupport::Kind::CHANNELLEN)
                    {
                        let max_len = *max_len as usize;

                        let channels = chanlist.split(',').collect::<Vec<_>>();

                        if let Some(channel) = channels
                            .into_iter()
                            .find(|channel| channel.len() > max_len)
                        {
                            return Err(Error::ArgTooLong {
                                name: "channel in chanlist",
                                len: channel.len(),
                                max_len,
                            });
                        }
                    }

                    Ok(Command::Irc(Irc::Part(chanlist, reason)))
                })
            }
            Kind::Topic => {
                validated::<1, 1, true>(args, |[channel], [topic]| {
                    if let Some(ref topic) = topic {
                        if let Some(isupport::Parameter::TOPICLEN(max_len)) =
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
                    }

                    Ok(Command::Irc(Irc::Topic(channel, topic)))
                })
            }
            Kind::Kick => {
                validated::<2, 1, true>(args, |[channel, users], [comment]| {
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

                    if let Some(ref comment) = comment {
                        if let Some(isupport::Parameter::KICKLEN(max_len)) =
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
                    }

                    Ok(Command::Irc(Irc::Kick(channel, users, comment)))
                })
            }
            Kind::Mode => validated::<1, 2, true>(
                args,
                |[target], [mode_string, mode_arguments]| {
                    if let Some(mode_string) = mode_string {
                        let mode_string_regex =
                            Regex::new(r"^((\+|\-)[A-Za-z]*)+$").unwrap();

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
                if let Some(ref comment) = comment {
                    if let Some(isupport::Parameter::AWAYLEN(max_len)) =
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
                }

                Ok(Command::Irc(Irc::Away(comment)))
            }),
            Kind::SetName => validated::<1, 0, true>(args, |[realname], _| {
                if let Some(isupport::Parameter::NAMELEN(max_len)) =
                    isupport.get(&isupport::Kind::NAMELEN)
                {
                    let max_len = *max_len as usize;

                    if realname.len() > max_len {
                        return Err(Error::ArgTooLong {
                            name: "realname",
                            len: realname.len(),
                            max_len,
                        });
                    }
                }

                Ok(Command::Irc(Irc::SetName(realname)))
            }),
            Kind::Notice => {
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
                        Ok(Command::Irc(Irc::Notice(targets, msg)))
                    } else {
                        Ok(Command::Internal(Internal::OpenBuffers(targets)))
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
        // Combine everything from last arg on
        let combined = args.iter().skip(max.saturating_sub(1)).join(" ");
        args.iter()
            .take(max.saturating_sub(1))
            .map(ToString::to_string)
            .chain((!combined.is_empty()).then_some(combined))
            .collect()
    } else {
        args.into_iter().map(String::from).collect()
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
            Irc::Me(target, text) => {
                ctcp::query_command(&ctcp::Command::Action, target, Some(text))
            }
            Irc::Whois(channel, user) => proto::Command::WHOIS(channel, user),
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
            Irc::Raw(raw) => proto::Command::Raw(raw),
            Irc::Unknown(command, args) => proto::Command::new(&command, args),
        })
    }
}

#[derive(Debug, thiserror::Error)]
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
