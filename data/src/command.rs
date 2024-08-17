use std::str::FromStr;

use irc::proto;
use itertools::Itertools;
use regex::Regex;

use crate::{ctcp, message::formatting, Buffer};

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
    Away,
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
            "raw" => Ok(Kind::Raw),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Command {
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
    Raw(String),
    Unknown(String, Vec<String>),
}

pub fn parse(s: &str, buffer: Option<&Buffer>) -> Result<Command, Error> {
    let (head, rest) = s.split_once('/').ok_or(Error::MissingSlash)?;
    // Don't allow leading whitespace before slash
    if !head.is_empty() {
        return Err(Error::MissingSlash);
    }

    let mut split = rest.split_ascii_whitespace();

    let cmd = split.next().ok_or(Error::MissingCommand)?;

    if rest.len() == cmd.len() {
        return Err(Error::MissingArgs);
    }

    let args = split.collect::<Vec<_>>();
    let raw = &rest[cmd.len() + 1..];

    let unknown = || {
        Command::Unknown(
            cmd.to_string(),
            args.iter().map(|s| s.to_string()).collect(),
        )
    };

    match cmd.parse::<Kind>() {
        Ok(kind) => match kind {
            Kind::Join => validated::<1, 1, false>(args, |[chanlist], [chankeys]| {
                Command::Join(chanlist, chankeys)
            }),
            Kind::Motd => validated::<0, 1, false>(args, |_, [target]| Command::Motd(target)),
            Kind::Nick => validated::<1, 0, false>(args, |[nick], _| Command::Nick(nick)),
            Kind::Quit => validated::<0, 1, true>(args, |_, [comment]| Command::Quit(comment)),
            Kind::Msg => {
                validated::<2, 0, true>(args, |[target, msg], []| Command::Msg(target, msg))
            }
            Kind::Me => {
                if let Some(target) = buffer.and_then(|b| b.target()) {
                    validated::<1, 0, true>(args, |[text], _| Command::Me(target, text))
                } else {
                    Ok(unknown())
                }
            }
            Kind::Whois => validated::<1, 0, false>(args, |[nick], _| {
                // Leaving out optional [server] for now.
                Command::Whois(None, nick)
            }),
            Kind::Part => validated::<1, 1, true>(args, |[chanlist], [reason]| {
                Command::Part(chanlist, reason)
            }),
            Kind::Topic => {
                validated::<1, 1, true>(args, |[channel], [topic]| Command::Topic(channel, topic))
            }
            Kind::Kick => validated::<2, 1, true>(args, |[channel, user], [comment]| {
                Command::Kick(channel, user, comment)
            }),
            Kind::Mode => {
                if let Some((target, rest)) = args.split_first() {
                    if let Some((mode_string, mode_arguments)) = rest.split_first() {
                        let mode_string_regex = Regex::new(r"^((\+|\-)[A-Za-z]*)+$").unwrap();
                        if!mode_string_regex.is_match(mode_string) {
                            Err(Error::InvalidModeString)
                        }
                        else {
                            let mode_arguments: Vec<String> = mode_arguments.iter().map(|v| v.to_string()).collect();
                            Ok(Command::Mode(
                                target.to_string(),
                                Some(mode_string.to_string()),
                                (!mode_arguments.is_empty()).then_some(mode_arguments)
                            ))
                        }
                    }
                    else {
                        Ok(Command::Mode(
                            target.to_string(),
                            None,
                            None,
                        ))
                    }
                }
                else {
                    Err(Error::MissingArgs)
                }
            },
            Kind::Away => validated::<0, 1, true>(args, |_, [comment]| Command::Away(comment)),
            Kind::Raw => Ok(Command::Raw(raw.to_string())),
            Kind::Format => {
                if let Some(target) = buffer.and_then(|b| b.target()) {
                    Ok(Command::Msg(target, formatting::encode(raw, false)))
                } else {
                    Ok(unknown())
                }
            }
        },
        Err(_) => Ok(unknown()),
    }
}

// TODO: Expand `validated` so we can better indicate which parameters is optional.
fn validated<const EXACT: usize, const OPT: usize, const TEXT: bool>(
    args: Vec<&str>,
    f: impl FnOnce([String; EXACT], [Option<String>; OPT]) -> Command,
) -> Result<Command, Error> {
    let max = EXACT + OPT;

    let args: Vec<String> = if TEXT {
        // Combine everything from last arg on
        let combined = args.iter().skip(max.saturating_sub(1)).join(" ");
        args.iter()
            .take(max.saturating_sub(1))
            .map(|s| s.to_string())
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

        Ok((f)(exact, opt))
    } else {
        Err(Error::IncorrectArgCount {
            min: EXACT,
            max,
            actual: args.len(),
        })
    }
}

impl TryFrom<Command> for proto::Command {
    type Error = ();

    fn try_from(command: Command) -> Result<Self, Self::Error> {
        Ok(match command {
            Command::Join(chanlist, chankeys) => proto::Command::JOIN(chanlist, chankeys),
            Command::Motd(target) => proto::Command::MOTD(target),
            Command::Nick(nick) => proto::Command::NICK(nick),
            Command::Quit(comment) => proto::Command::QUIT(comment),
            Command::Msg(target, msg) => proto::Command::PRIVMSG(target, msg),
            Command::Me(target, text) => {
                ctcp::query_command(&ctcp::Command::Action, target, Some(text))
            }
            Command::Whois(channel, user) => proto::Command::WHOIS(channel, user),
            Command::Part(chanlist, reason) => proto::Command::PART(chanlist, reason),
            Command::Topic(channel, topic) => proto::Command::TOPIC(channel, topic),
            Command::Kick(channel, user, comment) => proto::Command::KICK(channel, user, comment),
            Command::Mode(target, modestring, modearguments) => proto::Command::MODE(target, modestring, modearguments),
            Command::Away(comment) => proto::Command::AWAY(comment),
            Command::Raw(raw) => proto::Command::Raw(raw),
            Command::Unknown(command, args) => proto::Command::new(&command, args),
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
    #[error("missing args")]
    MissingArgs,
    #[error("invalid mode string")]
    InvalidModeString
}

fn fmt_incorrect_arg_count(min: usize, max: usize, actual: usize) -> String {
    if min == max {
        let s = if min == 1 { "" } else { "s" };

        format!("expected {min} argument{s}, received {actual}")
    } else {
        format!("expected {min} to {max} arguments, recevied {actual}")
    }
}
