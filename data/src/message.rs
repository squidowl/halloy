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
    // pub fn text(&self) -> String {
    //     match self {}
    // }

    // pub fn source(&self, channels: &[String]) -> Option<Source> {
    //     if self.is_server_message(channels) {
    //         Some(Source::Server)
    //     } else {
    //         channels.iter().find_map(|channel| {
    //             is_for_channel(self.inner(), channel).then(|| Source::Channel(channel.to_string()))
    //         })
    //     }
    // }

    // pub fn is_for_channel(&self, channel: &str) -> bool {
    //     is_for_channel(self.inner(), channel)
    // }

    // pub fn is_server_message(&self, channels: &[String]) -> bool {
    //     match self {
    //         Message::Sent { .. } => false,
    //         Message::Received(message) => is_for_server(message),
    //     }
    // }

    // pub fn user(&self) -> Option<User> {
    //     match &self {
    //         Message::Sent { nickname, .. } => Some(User::new(nickname)),
    //         Message::Received(message) => message
    //             .prefix
    //             .as_ref()
    //             .and_then(|prefix| User::try_from(prefix).ok()),
    //     }
    // }

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
