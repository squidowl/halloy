use std::str::FromStr;

use irc::proto;

#[derive(Debug, Clone, Copy)]
pub enum Kind {
    Join,
    Motd,
    Nick,
    Quit,
    PrivMsg,
}

impl FromStr for Kind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "join" => Ok(Kind::Join),
            "motd" => Ok(Kind::Motd),
            "nick" => Ok(Kind::Nick),
            "quit" => Ok(Kind::Quit),
            "privmsg" => Ok(Kind::PrivMsg),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Command {
    Join(String, Option<String>, Option<String>),
    Motd(Option<String>),
    Nick(String),
    Quit(Option<String>),
    PrivMsg(String, String),
    Unknown(String, Vec<String>, Option<String>),
}

impl FromStr for Command {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (head, rest) = s.split_once('/').ok_or(Error::MissingSlash)?;
        // Don't allow leading whitespace before slash
        if !head.is_empty() {
            return Err(Error::MissingSlash);
        }

        // Text must be prepended by a colon, treat it as a single arg
        let (leading, text) = rest.split_once(':').unwrap_or((rest, ""));
        let mut split = leading.split_ascii_whitespace();

        let cmd = split.next().ok_or(Error::MissingCommand)?;
        let args = split.collect::<Vec<_>>();

        match cmd.parse::<Kind>() {
            Ok(kind) => match kind {
                Kind::Join => {
                    validated::<1, 1, false>(args, text, |[chanlist], [chankeys], real_name| {
                        Command::Join(chanlist, chankeys, real_name)
                    })
                }
                Kind::Motd => {
                    validated::<0, 1, false>(args, text, |_, [target], _| Command::Motd(target))
                }
                Kind::Nick => {
                    validated::<1, 0, false>(args, text, |[nick], _, _| Command::Nick(nick))
                }
                Kind::Quit => {
                    validated::<0, 0, false>(args, text, |_, _, text| Command::Quit(text))
                }
                Kind::PrivMsg => validated::<1, 0, true>(args, text, |[target], _, text| {
                    Command::PrivMsg(target, text.unwrap_or_default())
                }),
            },
            Err(_) => Ok(Command::Unknown(
                cmd.to_string(),
                args.into_iter().map(String::from).collect(),
                (!text.is_empty()).then(|| text.to_string()),
            )),
        }
    }
}

fn validated<const EXACT: usize, const OPT: usize, const TEXT_REQUIRED: bool>(
    args: Vec<&str>,
    text: &str,
    f: impl Fn([String; EXACT], [Option<String>; OPT], Option<String>) -> Command,
) -> Result<Command, Error> {
    let max = EXACT + OPT;

    if TEXT_REQUIRED && text.is_empty() {
        return Err(Error::MissingColon);
    }

    if args.len() >= EXACT && args.len() <= max {
        let exact = args[0..EXACT]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let opt = args[EXACT..args.len()]
            .iter()
            .map(|s| Some(s.to_string()))
            .chain((args.len()..max).map(|_| None))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let last = (!text.is_empty()).then(|| text.to_string());

        Ok((f)(exact, opt, last))
    } else {
        Err(Error::InvalidUsage)
    }
}

impl TryFrom<Command> for proto::Command {
    type Error = proto::error::MessageParseError;

    fn try_from(command: Command) -> Result<Self, Self::Error> {
        Ok(match command {
            Command::Join(chanlist, chankeys, real_name) => {
                proto::Command::JOIN(chanlist, chankeys, real_name)
            }
            Command::Motd(target) => proto::Command::MOTD(target),
            Command::Nick(nick) => proto::Command::NICK(nick),
            Command::Quit(comment) => proto::Command::QUIT(comment),
            Command::PrivMsg(target, msg) => proto::Command::PRIVMSG(target, msg),
            Command::Unknown(command, args, text) => {
                let args = args
                    .iter()
                    .map(|arg| arg.as_str())
                    .chain(text.as_deref())
                    .collect();

                return proto::Command::new(command.as_str(), args);
            }
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("wrong # of arguments or missing ':' before text")]
    InvalidUsage,
    #[error("':' must prepend text argument")]
    MissingColon,
    #[error("missing slash")]
    MissingSlash,
    #[error("missing command")]
    MissingCommand,
}
