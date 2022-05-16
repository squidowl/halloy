use std::fmt::{self, Write};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct Message {
    raw: irc::proto::Message,
    command: Command,
}

impl Message {
    pub fn command(&self) -> &Command {
        &self.command
    }

    pub fn is_for_channel(&self, channel: &Channel) -> bool {
        match &self.command {
            Command::PrivMsg { msg_target, .. } | Command::Notice { msg_target, .. } => {
                match msg_target {
                    MsgTarget::Channel(c) => c == channel,
                    MsgTarget::User(_) => false,
                }
            }
            Command::Response { .. } | Command::Other(_) => false,
        }
    }

    pub fn is_for_server(&self) -> bool {
        match &self.command {
            Command::Response { .. } => true,
            _ => false,
        }
    }

    pub fn nickname(&self) -> String {
        if let Some(prefix) = self.raw.prefix.clone() {
            return match prefix {
                irc::proto::Prefix::ServerName(name) => name,
                // TODO: How to get mods, like '@'
                irc::proto::Prefix::Nickname(name, _, _) => name,
            };
        }

        // TODO: When can this happen?
        String::new()
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
    PrivMsg {
        msg_target: MsgTarget,
        text: String,
    },
    Notice {
        msg_target: MsgTarget,
        text: String,
    },
    Response {
        response: Response,
        text: Vec<String>,
    },
    Other(irc::proto::Command),
}

#[derive(Debug, Clone)]
pub enum Response {
    Welcome,
    MOTDStart,
    MOTD,
    MOTDEnd,
    Other,
}

impl Response {
    pub fn parse(&self, text: &Vec<String>) -> Option<String> {
        match self {
            Response::Welcome => text.get(1).cloned(),
            Response::MOTDStart => text.get(1).cloned(),
            Response::MOTD => text.get(1).cloned(),
            Response::MOTDEnd => text.get(1).cloned(),
            Response::Other => None,
        }
    }
}

impl From<irc::proto::Response> for Response {
    fn from(response: irc::proto::Response) -> Self {
        match response {
            irc::proto::Response::RPL_WELCOME => Response::Welcome,
            irc::proto::Response::RPL_MOTD => Response::MOTD,
            irc::proto::Response::RPL_MOTDSTART => Response::MOTDStart,
            irc::proto::Response::RPL_ENDOFMOTD => Response::MOTDEnd,
            _ => Response::Other,
        }
    }
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
            irc::proto::Command::Response(response, text) => Command::Response {
                response: response.into(),
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

impl fmt::Display for MsgTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MsgTarget::Channel(channel) => channel.fmt(f),
            MsgTarget::User(user) => user.fmt(f),
        }
    }
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

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = String::default();

        write!(s, "{}", self.first);

        if let Some(id) = &self.id {
            write!(s, "{}", id);
        }

        write!(s, "{}", self.name);

        if let Some(mask) = &self.mask {
            write!(s, ":{}", mask);
        }

        write!(f, "{}", s)
    }
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
