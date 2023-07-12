use chrono::{DateTime, Utc};
use irc::proto;
use irc::proto::ChannelExt;
use serde::{Deserialize, Serialize};

use crate::time::{self, Posix};
use crate::user::{Nick, NickRef};
use crate::User;

pub type Channel = String;

#[derive(Debug, Clone)]
pub struct Encoded(proto::Message);

impl Encoded {
    pub fn user(&self) -> Option<User> {
        fn not_empty(s: &str) -> Option<&str> {
            (!s.is_empty()).then_some(s)
        }

        let prefix = self.prefix.as_ref()?;
        match prefix {
            proto::Prefix::Nickname(nickname, username, hostname) => Some(User::new(
                Nick::from(nickname.as_str()),
                not_empty(username),
                not_empty(hostname),
            )),
            _ => None,
        }
    }
}

impl std::ops::Deref for Encoded {
    type Target = proto::Message;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Encoded {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<proto::Message> for Encoded {
    fn from(proto: proto::Message) -> Self {
        Self(proto)
    }
}

impl From<Encoded> for proto::Message {
    fn from(encoded: Encoded) -> Self {
        encoded.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Source {
    Server,
    Channel(Channel, Sender),
    Query(Nick, Sender),
    Status(Status),
}

impl Source {
    pub fn sender(&self) -> Option<&Sender> {
        match self {
            Source::Server => None,
            Source::Channel(_, sender) => Some(sender),
            Source::Query(_, sender) => Some(sender),
            Source::Status(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Sender {
    User(User),
    Server,
    Action,
    Status(Status),
}

impl Sender {
    pub fn user(&self) -> Option<&User> {
        match self {
            Sender::User(user) => Some(user),
            Sender::Server => None,
            Sender::Action => None,
            Sender::Status(_) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Status {
    Success,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Direction {
    Sent,
    Received,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub received_at: Posix,
    pub server_time: DateTime<Utc>,
    pub direction: Direction,
    pub source: Source,
    pub text: String,
}

impl Message {
    pub fn channel(&self) -> Option<&str> {
        if let Source::Channel(channel, _) = &self.source {
            Some(channel)
        } else {
            None
        }
    }

    pub fn sent_by(&self) -> Option<&User> {
        match &self.source {
            Source::Server => None,
            Source::Channel(_, kind) => kind.user(),
            Source::Query(_, kind) => kind.user(),
            Source::Status(_) => None,
        }
    }

    pub fn triggers_unread(&self) -> bool {
        matches!(self.direction, Direction::Received)
            && matches!(self.source.sender(), Some(Sender::User(_) | Sender::Action))
    }

    pub fn received(encoded: Encoded, our_nick: Nick) -> Option<Message> {
        let server_time = server_time(&encoded);
        let text = text(&encoded, &our_nick)?;
        let source = source(encoded, &our_nick)?;

        Some(Message {
            received_at: Posix::now(),
            server_time,
            direction: Direction::Received,
            source,
            text,
        })
    }

    pub fn with_source(self, source: Source) -> Self {
        Self { source, ..self }
    }
}

fn source(message: Encoded, our_nick: &Nick) -> Option<Source> {
    let user = message.user();

    match message.0.command {
        // Channel
        proto::Command::TOPIC(channel, _)
        | proto::Command::PART(channel, _)
        | proto::Command::ChannelMODE(channel, _)
        | proto::Command::KICK(channel, _, _)
        | proto::Command::SAJOIN(_, channel)
        | proto::Command::JOIN(channel, _, _) => Some(Source::Channel(channel, Sender::Server)),
        proto::Command::Response(
            proto::Response::RPL_TOPIC | proto::Response::RPL_TOPICWHOTIME,
            params,
        ) => {
            let channel = params.get(1)?.clone();
            Some(Source::Channel(channel, Sender::Server))
        }
        proto::Command::PRIVMSG(target, text) => {
            let is_action = is_action(&text);
            let sender = |user| {
                if is_action {
                    Sender::Action
                } else {
                    Sender::User(user)
                }
            };

            match (target.is_channel_name(), user) {
                (true, Some(user)) => Some(Source::Channel(target, sender(user))),
                (false, Some(user)) => {
                    let target = User::try_from(target.as_str()).ok()?;

                    (target.nickname() == *our_nick)
                        .then(|| Source::Query(user.nickname().to_owned(), sender(user)))
                }
                _ => None,
            }
        }
        proto::Command::NOTICE(target, text) => {
            let is_action = is_action(&text);
            let sender = |user| {
                if is_action {
                    Sender::Action
                } else {
                    Sender::User(user)
                }
            };

            match (target.is_channel_name(), user) {
                (true, Some(user)) => Some(Source::Channel(target, sender(user))),
                (false, Some(user)) => {
                    let target = User::try_from(target.as_str()).ok()?;

                    (target.nickname() == *our_nick)
                        .then(|| Source::Query(user.nickname().to_owned(), sender(user)))
                }
                _ => Some(Source::Server),
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
        | proto::Command::SAQUIT(_, _) => Some(Source::Server),
    }
}

fn server_time(message: &Encoded) -> DateTime<Utc> {
    message
        .tags
        .as_ref()
        .and_then(|tags| tags.iter().find(|tag| tag.0 == "time"))
        .and_then(|tag| tag.1.clone())
        .and_then(|rfc3339| DateTime::parse_from_rfc3339(&rfc3339).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now)
}

fn text(message: &Encoded, our_nick: &Nick) -> Option<String> {
    let user = message.user();
    match &message.command {
        proto::Command::TOPIC(_, topic) => {
            let user = user?;
            let topic = topic.as_ref()?;

            Some(format!(" ∙ {user} changed topic to {topic}"))
        }
        proto::Command::PART(_, text) => {
            let user = user?.formatted();
            let text = text
                .as_ref()
                .map(|text| format!(" ({text})"))
                .unwrap_or_default();

            Some(format!("⟵ {user} has left the channel{text}"))
        }
        proto::Command::JOIN(_, _, _) | proto::Command::SAJOIN(_, _) => {
            let user = user?;

            (user.nickname() != *our_nick)
                .then(|| format!("⟶ {} has joined the channel", user.formatted()))
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
        proto::Command::PRIVMSG(_, text) => {
            // Check if a synthetic action message
            if let Some(nick) = user.as_ref().map(User::nickname) {
                if let Some(action) = parse_action(nick, text) {
                    return Some(action);
                }
            }

            Some(text.clone())
        }
        proto::Command::NOTICE(_, text) => Some(text.clone()),
        proto::Command::Response(proto::Response::RPL_TOPIC, params) => {
            let topic = params.get(2)?;

            Some(format!(" ∙ topic is {topic}"))
        }
        proto::Command::Response(proto::Response::RPL_TOPICWHOTIME, params) => {
            let nick = params.get(2)?;
            let datetime = params
                .get(3)?
                .parse::<u64>()
                .ok()
                .map(Posix::from_seconds)
                .as_ref()
                .and_then(Posix::datetime)?
                .to_rfc2822();

            Some(format!(" ∙ topic set by {nick} at {datetime}"))
        }
        proto::Command::Response(_, responses) | proto::Command::Raw(_, responses) => Some(
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
}

fn is_action(text: &str) -> bool {
    text.starts_with("\u{1}ACTION ") && text.ends_with('\u{1}')
}

pub fn parse_action(nick: NickRef, text: &str) -> Option<String> {
    let action = text.strip_prefix("\u{1}ACTION ")?.strip_suffix('\u{1}')?;
    Some(action_text(nick, action))
}

pub fn action_text(nick: NickRef, action: &str) -> String {
    format!(" ∙ {nick} {action}")
}

pub(crate) mod broadcast {
    //! Generate messages that can be broadcast into every buffer
    use chrono::Utc;

    use super::{Direction, Message, Sender, Source, Status};
    use crate::time::Posix;
    use crate::user::Nick;
    use crate::User;

    enum Cause {
        Server,
        Status(Status),
    }

    fn expand(
        channels: impl IntoIterator<Item = String>,
        queries: impl IntoIterator<Item = Nick>,
        include_server: bool,
        cause: Cause,
        text: String,
    ) -> Vec<Message> {
        let message = |source, text| -> Message {
            Message {
                received_at: Posix::now(),
                server_time: Utc::now(),
                direction: Direction::Received,
                source,
                text,
            }
        };

        let (source, sender) = match cause {
            Cause::Server => (Source::Server, Sender::Server),
            Cause::Status(status) => (Source::Status(status), Sender::Status(status)),
        };

        channels
            .into_iter()
            .map(|channel| message(Source::Channel(channel, sender.clone()), text.clone()))
            .chain(
                queries
                    .into_iter()
                    .map(|nick| message(Source::Query(nick, sender.clone()), text.clone())),
            )
            .chain(include_server.then(|| message(source, text.clone())))
            .collect()
    }

    pub fn connecting() -> Vec<Message> {
        let text = " ∙ connecting to server...".into();
        expand([], [], true, Cause::Status(Status::Success), text)
    }

    pub fn connected() -> Vec<Message> {
        let text = " ∙ connected".into();
        expand([], [], true, Cause::Status(Status::Success), text)
    }

    pub fn connection_failed(error: String) -> Vec<Message> {
        let text = format!(" ∙ connection to server failed ({error})");
        expand([], [], true, Cause::Status(Status::Error), text)
    }

    pub fn disconnected(
        channels: impl IntoIterator<Item = String>,
        queries: impl IntoIterator<Item = Nick>,
        error: Option<String>,
    ) -> Vec<Message> {
        let error = error.map(|error| format!(" ({error})")).unwrap_or_default();
        let text = format!(" ∙ connection to server lost{error}");
        expand(channels, queries, true, Cause::Status(Status::Error), text)
    }

    pub fn reconnected(
        channels: impl IntoIterator<Item = String>,
        queries: impl IntoIterator<Item = Nick>,
    ) -> Vec<Message> {
        let text = " ∙ connection to server restored".into();
        expand(
            channels,
            queries,
            true,
            Cause::Status(Status::Success),
            text,
        )
    }

    pub fn quit(
        channels: impl IntoIterator<Item = String>,
        queries: impl IntoIterator<Item = Nick>,
        user: &User,
        comment: &Option<String>,
    ) -> Vec<Message> {
        let comment = comment
            .as_ref()
            .map(|comment| format!(" ({comment})"))
            .unwrap_or_default();
        let text = format!("⟵ {} has quit{comment}", user.formatted());

        expand(channels, queries, false, Cause::Server, text)
    }

    pub fn nickname(
        channels: impl IntoIterator<Item = String>,
        queries: impl IntoIterator<Item = Nick>,
        old_nick: &Nick,
        new_nick: &Nick,
        ourself: bool,
    ) -> Vec<Message> {
        let text = if ourself {
            format!(" ∙ You're now known as {new_nick}")
        } else {
            format!(" ∙ {old_nick} is now known as {new_nick}")
        };

        expand(channels, queries, false, Cause::Server, text)
    }
}
