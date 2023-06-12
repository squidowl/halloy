use irc::proto;
use irc::proto::ChannelExt;

use crate::User;

#[derive(Debug, Clone)]
pub enum Source {
    Server,
    Channel(String, User),
    Private(User),
}

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Sent,
    Received,
}

#[derive(Debug, Clone)]
pub struct Message {
    // TODO: Add timestamp
    pub timestamp: u64,
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

    pub fn user(&self) -> Option<&User> {
        match &self.source {
            Source::Server => None,
            Source::Channel(_, user) => Some(user),
            Source::Private(user) => Some(user),
        }
    }

    pub fn received(proto: proto::Message) -> Option<Message> {
        let text = text(&proto)?;
        let prefix = proto.prefix?;

        let source = match prefix {
            proto::Prefix::ServerName(_) => Source::Server,
            proto::Prefix::Nickname(nick, user, host) => match proto.command {
                proto::Command::PRIVMSG(target, _) | proto::Command::NOTICE(target, _) => {
                    fn not_empty<'a>(s: &'a str) -> Option<&'a str> {
                        (!s.is_empty()).then_some(s)
                    }

                    let user = User::new(&nick, not_empty(&user), not_empty(&host));

                    if target.is_channel_name() {
                        Source::Channel(target, user)
                    } else {
                        Source::Private(user)
                    }
                }
                _ => return None,
            },
        };

        Some(Message {
            timestamp: 0,
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
}

impl Default for Limit {
    fn default() -> Self {
        Self::Bottom(500)
    }
}

impl Limit {
    pub const DEFAULT_STEP: usize = 50;

    pub fn value(self) -> usize {
        match self {
            Limit::Top(i) => i,
            Limit::Bottom(i) => i,
        }
    }

    fn value_mut(&mut self) -> &mut usize {
        match self {
            Limit::Top(i) => i,
            Limit::Bottom(i) => i,
        }
    }

    pub fn increase(&mut self, n: usize) {
        *self.value_mut() += n;
    }
}
