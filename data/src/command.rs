use std::collections::HashMap;
use std::str::FromStr;

use fancy_regex::Regex;
use irc::proto;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::buffer::{self, Upstream};
use crate::isupport::{self, find_target_limit};
use crate::message::{self, formatting};
use crate::user::{ChannelUsers, NickRef};
use crate::{Config, Message, Target, Url, User, ctcp, target};

#[derive(Debug, Clone)]
pub enum Command {
    Internal(Internal),
    Irc(Irc),
}

#[derive(Debug, Clone)]
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
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum Irc {
    List(Option<String>, Option<String>),
    Join(String, Option<String>),
    Motd(Option<String>),
    Nick(String),
    Quit(Option<String>),
    Msg(String, String),
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
    Raw(String),
    Unknown(String, Vec<String>),
    Ctcp(ctcp::Command, String, Option<String>),
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
        let to_targets = |target: &str, source| {
            let target =
                Target::parse(target, chantypes, statusmsg, casemapping);

            let message_target = match &target {
                Target::Channel(channel) => message::Target::Channel {
                    channel: channel.clone(),
                    source,
                },

                Target::Query(query) => message::Target::Query {
                    query: query.clone(),
                    source,
                },
            };

            (target, message_target)
        };

        match self {
            Irc::Msg(targets, text) => Some(
                targets
                    .split(',')
                    .map(|target| {
                        let (target, message_target) = to_targets(
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
                        let (target, message_target) = to_targets(
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
                let (_, message_target) = to_targets(
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
    Plain,
    Away,
    SetName,
    Ctcp,
    Hop,
    Notice,
    Delay,
    Clear,
    List,
    ClearTopic,
    SysInfo,
    Detach,
    Connect,
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
            "msg" | "query" => Ok(Kind::Msg),
            "me" | "describe" => Ok(Kind::Me),
            "whois" => Ok(Kind::Whois),
            "part" | "leave" => Ok(Kind::Part),
            "topic" | "t" => Ok(Kind::Topic),
            "kick" => Ok(Kind::Kick),
            "mode" | "m" => Ok(Kind::Mode),
            "format" | "f" => Ok(Kind::Format),
            "plain" | "p" => Ok(Kind::Plain),
            "away" => Ok(Kind::Away),
            "setname" => Ok(Kind::SetName),
            "notice" => Ok(Kind::Notice),
            "raw" => Ok(Kind::Raw),
            "ctcp" => Ok(Kind::Ctcp),
            "hop" | "rejoin" => Ok(Kind::Hop),
            "delay" => Ok(Kind::Delay),
            "clear" => Ok(Kind::Clear),
            "list" => Ok(Kind::List),
            "cleartopic" | "ct" => Ok(Kind::ClearTopic),
            "sysinfo" => Ok(Kind::SysInfo),
            "detach" => Ok(Kind::Detach),
            "connect" => Ok(Kind::Connect),
            _ => Err(()),
        }
    }
}

pub fn parse(
    s: &str,
    buffer: Option<&buffer::Upstream>,
    our_nickname: Option<NickRef>,
    isupport: &HashMap<isupport::Kind, isupport::Parameter>,
    config: &Config,
) -> Result<Command, Error> {
    let (head, rest) = s.split_once('/').ok_or(Error::MissingSlash)?;
    // Don't allow leading whitespace before slash
    if !head.is_empty() {
        return Err(Error::MissingSlash);
    }

    let mut split = rest.split(' ');

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
                if let Ok(seconds) = seconds.parse::<u64>() {
                    if seconds > 0 {
                        Ok(Command::Internal(Internal::Delay(seconds)))
                    } else {
                        Err(Error::NotPositiveInteger)
                    }
                } else {
                    Err(Error::NotPositiveInteger)
                }
            }),
            Kind::Connect => validated::<1, 0, false>(args, |[server], _| {
                if let Ok(url) = Url::from_str(&server)
                    && matches!(url, Url::ServerConnect { .. })
                {
                    Ok(Command::Internal(Internal::Connect(server)))
                } else {
                    Err(Error::InvalidServerUrl)
                }
            }),
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
            Irc::Raw(raw) => proto::Command::Raw(raw),
            Irc::Unknown(command, args) => proto::Command::new(&command, args),
            Irc::Ctcp(command, target, params) => {
                ctcp::query_command(&command, target, params)
            }
            Irc::List(channels, elistcond) => {
                proto::Command::LIST(channels, elistcond)
            }
        })
    }
}

impl TryFrom<Irc> for message::Encoded {
    type Error = ();

    fn try_from(command: Irc) -> Result<Self, Self::Error> {
        Ok(message::Encoded::from(proto::Message::from(
            proto::Command::try_from(command)?,
        )))
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
