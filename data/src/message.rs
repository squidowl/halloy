use irc::proto;
use irc::proto::ChannelExt;
use serde::{Deserialize, Serialize};

use crate::{time, User};

pub type Raw = irc::proto::Message;
pub type Channel = String;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Source {
    Server,
    Channel(Channel, ChannelSender),
    Query(User),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChannelSender {
    /// `ChannelSender::User(_)` is coming from another client.
    User(User),
    /// `ChannelSender::Server()` is from the server, targeting a channel.
    Server,
}

impl ChannelSender {
    pub fn user(&self) -> Option<&User> {
        match self {
            ChannelSender::User(user) => Some(user),
            ChannelSender::Server => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Direction {
    Sent,
    Received,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub timestamp: time::Posix,
    pub direction: Direction,
    pub source: Source,
    pub text: String,
}

impl Message {
    pub fn is_server(&self) -> bool {
        matches!(self.source, Source::Server)
    }

    pub fn channel(&self) -> Option<&str> {
        if let Source::Channel(channel, _) = &self.source {
            Some(channel)
        } else {
            None
        }
    }

    pub fn query(&self) -> Option<&User> {
        if let Source::Query(user) = &self.source {
            Some(user)
        } else {
            None
        }
    }

    pub fn user(&self) -> Option<&User> {
        match &self.source {
            Source::Server => None,
            Source::Channel(_, kind) => kind.user(),
            Source::Query(user) => Some(user),
        }
    }

    pub fn received(proto: proto::Message) -> Option<Message> {
        let text = text(&proto)?;
        let source = match &proto.command {
            // Channel
            proto::Command::TOPIC(channel, _)
            | proto::Command::PART(channel, _)
            | proto::Command::ChannelMODE(channel, _)
            | proto::Command::KICK(channel, _, _)
            | proto::Command::SAJOIN(_, channel)
            | proto::Command::JOIN(channel, _, _) => {
                Source::Channel(channel.clone(), ChannelSender::Server)
            }
            proto::Command::PRIVMSG(target, _) | proto::Command::NOTICE(target, _) => {
                let user = user(&proto);

                match (target.is_channel_name(), user) {
                    (true, Some(user)) => {
                        Source::Channel(target.clone(), ChannelSender::User(user))
                    }
                    (false, Some(user)) => Source::Query(user),
                    _ => {
                        return None;
                    }
                }
            }

            // Server
            proto::Command::SANICK(_, _)
            | proto::Command::SAMODE(_, _, _)
            | proto::Command::PASS(_)
            | proto::Command::NICK(_)
            | proto::Command::USER(_, _, _)
            | proto::Command::OPER(_, _)
            | proto::Command::UserMODE(_, _)
            | proto::Command::SERVICE(_, _, _, _, _, _)
            | proto::Command::QUIT(_)
            | proto::Command::SQUIT(_, _)
            | proto::Command::NAMES(_, _)
            | proto::Command::LIST(_, _)
            | proto::Command::INVITE(_, _)
            | proto::Command::MOTD(_)
            | proto::Command::LUSERS(_, _)
            | proto::Command::VERSION(_)
            | proto::Command::STATS(_, _)
            | proto::Command::LINKS(_, _)
            | proto::Command::TIME(_)
            | proto::Command::CONNECT(_, _, _)
            | proto::Command::TRACE(_)
            | proto::Command::ADMIN(_)
            | proto::Command::INFO(_)
            | proto::Command::SERVLIST(_, _)
            | proto::Command::SQUERY(_, _)
            | proto::Command::WHO(_, _)
            | proto::Command::WHOIS(_, _)
            | proto::Command::WHOWAS(_, _, _)
            | proto::Command::KILL(_, _)
            | proto::Command::PING(_, _)
            | proto::Command::PONG(_, _)
            | proto::Command::ERROR(_)
            | proto::Command::AWAY(_)
            | proto::Command::REHASH
            | proto::Command::DIE
            | proto::Command::RESTART
            | proto::Command::SUMMON(_, _, _)
            | proto::Command::USERS(_)
            | proto::Command::WALLOPS(_)
            | proto::Command::USERHOST(_)
            | proto::Command::ISON(_)
            | proto::Command::SAPART(_, _)
            | proto::Command::NICKSERV(_)
            | proto::Command::CHANSERV(_)
            | proto::Command::OPERSERV(_)
            | proto::Command::BOTSERV(_)
            | proto::Command::HOSTSERV(_)
            | proto::Command::MEMOSERV(_)
            | proto::Command::CAP(_, _, _, _)
            | proto::Command::AUTHENTICATE(_)
            | proto::Command::ACCOUNT(_)
            | proto::Command::METADATA(_, _, _)
            | proto::Command::MONITOR(_, _)
            | proto::Command::BATCH(_, _, _)
            | proto::Command::CHGHOST(_, _)
            | proto::Command::Response(_, _)
            | proto::Command::Raw(_, _)
            | proto::Command::SAQUIT(_, _) => Source::Server,
        };

        Some(Message {
            timestamp: time::Posix::now(),
            direction: Direction::Received,
            source,
            text,
        })
    }
}

fn user(proto: &proto::Message) -> Option<User> {
    fn not_empty(s: &str) -> Option<&str> {
        (!s.is_empty()).then_some(s)
    }

    let prefix = proto.clone().prefix?;
    match prefix {
        proto::Prefix::Nickname(nickname, username, hostname) => Some(User::new(
            &nickname,
            not_empty(&username),
            not_empty(&hostname),
        )),
        _ => None,
    }
}

fn text(message: &irc::proto::Message) -> Option<String> {
    let user = user(message);
    match &message.command {
        proto::Command::TOPIC(_, topic) => {
            let user = user?;
            let topic = topic.as_ref()?;

            Some(format!(" ∙ {user} changed topic to {topic}"))
        }
        proto::Command::PART(_, text) => {
            let user = user?;
            let text = text
                .as_ref()
                .map(|text| format!(" ({text})"))
                .unwrap_or_default();

            Some(format!("⟵ {user}{text} has left the channel"))
        }
        proto::Command::JOIN(_, _, _) | proto::Command::SAJOIN(_, _) => {
            let user = user?;

            Some(format!("⟶ {user} has joined the channel"))
        }
        proto::Command::ChannelMODE(_, modes) => {
            let user = user?;
            let modes = modes
                .iter()
                .map(|mode| mode.to_string())
                .collect::<Vec<_>>()
                .join(" ");

            Some(format!(" ∙ {user} sets mode {modes}"))
        }
        proto::Command::PRIVMSG(_, text) | proto::Command::NOTICE(_, text) => Some(text.clone()),
        proto::Command::Response(_, responses) => Some(
            responses
                .iter()
                .map(|s| s.as_str())
                .skip(1)
                .collect::<Vec<_>>()
                .join(" "),
        ),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Limit {
    Top(usize),
    Bottom(usize),
    Since(time::Posix),
}

impl Limit {
    pub const DEFAULT_STEP: usize = 50;
    const DEFAULT_COUNT: usize = 500;

    pub fn top() -> Self {
        Self::Top(Self::DEFAULT_COUNT)
    }

    pub fn bottom() -> Self {
        Self::Bottom(Self::DEFAULT_COUNT)
    }

    fn value_mut(&mut self) -> Option<&mut usize> {
        match self {
            Limit::Top(i) => Some(i),
            Limit::Bottom(i) => Some(i),
            Limit::Since(_) => None,
        }
    }

    pub fn increase(&mut self, n: usize) {
        if let Some(value) = self.value_mut() {
            *value += n;
        }
    }
}
