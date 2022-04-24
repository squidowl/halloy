use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct Message {
    raw: irc::proto::Message,
    command: Command,
}

impl Message {
    pub fn is_for_channel(&self, channel: &Channel) -> bool {
        match &self.command {
            Command::PrivMsg { msg_target, .. } | Command::Notice { msg_target, .. } => {
                match msg_target {
                    MsgTarget::Channel(c) => c == channel,
                    MsgTarget::User(_) => false,
                }
            }
            Command::Other(_) => false,
        }
    }
}

impl From<irc::proto::Message> for Message {
    fn from(raw: irc::proto::Message) -> Self {
        let command = Command::from(raw.command.clone());

        Self { raw, command }
    }
}

#[derive(Debug, Clone)]
pub enum Command {
    PrivMsg { msg_target: MsgTarget, text: String },
    Notice { msg_target: MsgTarget, text: String },
    Other(irc::proto::Command),
}

impl From<irc::proto::Command> for Command {
    fn from(command: irc::proto::Command) -> Self {
        match command {
            irc::proto::Command::PRIVMSG(msg_target, text) => Command::PrivMsg {
                msg_target: MsgTarget::from(msg_target),
                text,
            },
            irc::proto::Command::NOTICE(msg_target, text) => Command::Notice {
                msg_target: MsgTarget::from(msg_target),
                text,
            },
            _ => Command::Other(command),
        }
    }
}

#[derive(Debug, Clone)]
pub enum MsgTarget {
    Channel(Channel),
    User(String),
}

impl From<String> for MsgTarget {
    fn from(msg_target: String) -> Self {
        match msg_target.parse::<Channel>() {
            Ok(channel) => MsgTarget::Channel(channel),
            Err(_) => MsgTarget::User(msg_target),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Target {
    Nickname(String),
    Server(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Channel {
    first: char,
    id: Option<String>,
    name: String,
    mask: Option<String>,
}

impl FromStr for Channel {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars();

        let first = match chars.next() {
            Some(c) | Some(c) | Some(c) | Some(c) if ['#', '&', '+', '!'].contains(&c) => c,
            _ => return Err(Error::InvalidChannel(s.to_string())),
        };

        let id = if first == '!' {
            let id = (0..5)
                .into_iter()
                .filter_map(|_| chars.next())
                .collect::<String>();

            if id.len() != 5 {
                return Err(Error::InvalidChannel(s.to_string()));
            }

            if !id
                .chars()
                .all(|c| (c.is_ascii_alphabetic() && c.is_ascii_uppercase()) || c.is_numeric())
            {
                return Err(Error::InvalidChannel(s.to_string()));
            }

            Some(id)
        } else {
            None
        };

        let rest = chars.collect::<String>();
        let mut split = rest.split(':');

        // TODO: Check if name is only valid chars
        let name = split
            .next()
            .ok_or_else(|| Error::InvalidChannel(s.to_string()))?;
        let mask = split.next();

        Ok(Self {
            first,
            id,
            name: name.to_string(),
            mask: mask.map(String::from),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid channel: {0}")]
    InvalidChannel(String),
}
