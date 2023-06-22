use std::str::FromStr;

use irc::proto;
use itertools::Itertools;

#[derive(Debug, Clone, Copy)]
pub enum Kind {
    Join,
    Motd,
    Nick,
    Quit,
    Msg,
}

impl FromStr for Kind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "join" => Ok(Kind::Join),
            "motd" => Ok(Kind::Motd),
            "nick" => Ok(Kind::Nick),
            "quit" => Ok(Kind::Quit),
            "msg" => Ok(Kind::Msg),
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
    Unknown(String, Vec<String>),
}

impl FromStr for Command {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (head, rest) = s.split_once('/').ok_or(Error::MissingSlash)?;
        // Don't allow leading whitespace before slash
        if !head.is_empty() {
            return Err(Error::MissingSlash);
        }

        let mut split = rest.split_ascii_whitespace();

        let cmd = split.next().ok_or(Error::MissingCommand)?;
        let args = split.collect::<Vec<_>>();

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
            },
            Err(_) => Ok(Command::Unknown(
                cmd.to_string(),
                args.into_iter().map(String::from).collect(),
            )),
        }
    }
}

fn validated<const EXACT: usize, const OPT: usize, const TEXT: bool>(
    args: Vec<&str>,
    f: impl Fn([String; EXACT], [Option<String>; OPT]) -> Command,
) -> Result<Command, Error> {
    let max = EXACT + OPT;

    let args: Vec<String> = if TEXT {
        // Combine everything from last arg on
        args.iter()
            .take(max.saturating_sub(1))
            .map(|s| s.to_string())
            .chain(Some(args.iter().skip(max.saturating_sub(1)).join(" ")))
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
    type Error = proto::error::MessageParseError;

    fn try_from(command: Command) -> Result<Self, Self::Error> {
        Ok(match command {
            Command::Join(chanlist, chankeys) => proto::Command::JOIN(chanlist, chankeys, None),
            Command::Motd(target) => proto::Command::MOTD(target),
            Command::Nick(nick) => proto::Command::NICK(nick),
            Command::Quit(comment) => proto::Command::QUIT(comment),
            Command::Msg(target, msg) => proto::Command::PRIVMSG(target, msg),
            Command::Unknown(command, args) => {
                let args = args.iter().map(|arg| arg.as_str()).collect();

                return proto::Command::new(command.as_str(), args);
            }
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
}

fn fmt_incorrect_arg_count(min: usize, max: usize, actual: usize) -> String {
    if min == max {
        let s = if min == 1 { "" } else { "s" };

        format!("expected {min} argument{s}, received {actual}")
    } else {
        format!("expected {min} to {max} arguments, recevied {actual}")
    }
}
