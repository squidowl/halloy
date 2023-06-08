use std::str::FromStr;

use irc::proto;

#[derive(Debug, Clone, Copy)]
pub enum Kind {
    Join,
    Motd,
    Nick,
    Quit,
}

impl FromStr for Kind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "join" => Ok(Kind::Join),
            "motd" => Ok(Kind::Motd),
            "nick" => Ok(Kind::Nick),
            "quit" => Ok(Kind::Quit),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Command {
    Join(String),
    Motd,
    Nick(String),
    Quit,
    Unknown(String, Vec<String>),
}

impl FromStr for Command {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (_, rest) = s.split_once('/').ok_or(Error::MissingSlash)?;
        let mut split = rest.split_ascii_whitespace();

        let command_str = split.next().ok_or(Error::MissingCommand)?;
        let args = split.collect::<Vec<_>>();

        fn validated<const N: usize>(
            args: Vec<&str>,
            f: impl Fn([&str; N]) -> Command,
        ) -> Result<Command, Error> {
            if args.len() == N {
                Ok((f)(args.try_into().unwrap()))
            } else {
                Err(Error::IncorrectArgCount {
                    expected: N,
                    actual: args.len(),
                })
            }
        }

        match command_str.parse::<Kind>() {
            Ok(command) => match command {
                Kind::Join => validated::<1>(args, |[channel]| Command::Join(channel.to_string())),
                Kind::Motd => validated::<0>(args, |[]| Command::Motd),
                Kind::Nick => validated::<1>(args, |[nick]| Command::Nick(nick.to_string())),
                Kind::Quit => validated::<0>(args, |_| Command::Quit),
            },
            Err(_) => Ok(Command::Unknown(
                command_str.to_string(),
                args.into_iter().map(String::from).collect(),
            )),
        }
    }
}

impl TryFrom<Command> for proto::Command {
    type Error = proto::error::MessageParseError;

    fn try_from(command: Command) -> Result<Self, Self::Error> {
        Ok(match command {
            // TODO: Support chankeys & realname
            Command::Join(channel) => proto::Command::JOIN(channel, None, None),
            Command::Motd => proto::Command::MOTD(None),
            Command::Nick(nick) => proto::Command::NICK(nick),
            // TODO: Support comment?
            Command::Quit => proto::Command::QUIT(None),
            Command::Unknown(command, args) => {
                let args = args.iter().map(|arg| arg.as_str()).collect();

                return proto::Command::new(command.as_str(), args);
            }
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("expected {expected} {}, received {actual}", if *expected == 1 { "argument" } else { "arguments" })]
    IncorrectArgCount { expected: usize, actual: usize },
    #[error("missing slash")]
    MissingSlash,
    #[error("missing command")]
    MissingCommand,
}
