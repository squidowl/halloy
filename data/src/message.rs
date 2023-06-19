use irc::proto;
use irc::proto::ChannelExt;
use serde::{Deserialize, Serialize};

use crate::{time, User};

pub type Raw = irc::proto::Message;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Source {
    Server,
    Channel(String, User),
    Query(User),
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
            Source::Channel(_, user) => Some(user),
            Source::Query(user) => Some(user),
        }
    }

    pub fn received(proto: proto::Message) -> Option<Message> {
        let text = text(&proto)?;
        let prefix = proto.prefix?;

        let source = match prefix {
            proto::Prefix::ServerName(_) => Source::Server,
            proto::Prefix::Nickname(nick, user, host) => match proto.command {
                proto::Command::PRIVMSG(target, _) | proto::Command::NOTICE(target, _) => {
                    fn not_empty(s: &str) -> Option<&str> {
                        (!s.is_empty()).then_some(s)
                    }

                    let user = User::new(&nick, not_empty(&user), not_empty(&host));

                    if target.is_channel_name() {
                        Source::Channel(target, user)
                    } else {
                        Source::Query(user)
                    }
                }
                _ => return None,
            },
        };

        Some(Message {
            timestamp: time::Posix::now(),
            direction: Direction::Received,
            source,
            text,
        })
    }
}

fn text(message: &irc::proto::Message) -> Option<String> {
    match &message.command {
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
