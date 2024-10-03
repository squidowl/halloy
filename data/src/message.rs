use std::borrow::Cow;
use std::hash::{DefaultHasher, Hash as _, Hasher};
use std::iter;

use chrono::{DateTime, Utc};
use const_format::concatcp;
use irc::proto;
use irc::proto::Command;
use itertools::{Either, Itertools};
use once_cell::sync::Lazy;
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Deserializer, Serialize};
use url::Url;

pub use self::formatting::Formatting;
pub use self::source::Source;

use crate::config::buffer::UsernameFormat;
use crate::time::{self, Posix};
use crate::user::{Nick, NickRef};
use crate::{ctcp, Config, Server, User};

// References:
// - https://datatracker.ietf.org/doc/html/rfc1738#section-5
// - https://www.ietf.org/rfc/rfc2396.txt

const URL_PATH_UNRESERVED: &str = r#"\p{Letter}\p{Number}\-_.!~*'()"#;

const URL_PATH_RESERVED: &str = r#";?:@&=+$,"#;

const URL_PATH: &str = concatcp!(r#"["#, URL_PATH_UNRESERVED, URL_PATH_RESERVED, r#"%\/#]"#);

const URL_PATH_UNRESERVED_EXC_PUNC: &str = r#"\p{Letter}\p{Number}\-_~*'("#;

const URL_PATH_RESERVED_EXC_PUNC: &str = r#"@&=+$"#;

const URL_PATH_EXC_PUNC: &str = concatcp!(
    r#"["#,
    URL_PATH_UNRESERVED_EXC_PUNC,
    URL_PATH_RESERVED_EXC_PUNC,
    r#"%\/#]"#
);

static URL_REGEX: Lazy<Regex> = Lazy::new(|| {
    RegexBuilder::new(concatcp!(
        r#"(?i)(((https?|ircs?):\/\/|www\.)[\p{Letter}\p{Number}\-@:%._+~#=]{1,256}\.[\p{Letter}\p{Number}()]{1,63}\b"#,
        r#"("#,
        URL_PATH,
        r#"*"#,
        URL_PATH_EXC_PUNC,
        r#"|"#,
        URL_PATH_EXC_PUNC,
        r#"?)|halloy:\/\/[^ ]*)"#
    ))
    .size_limit(15728640) // 1.5x default size_limit
    .build()
    .unwrap()
});

pub type Channel = String;

pub(crate) mod broadcast;
pub mod formatting;
pub mod source;

#[derive(Debug, Clone)]
pub struct Encoded(proto::Message);

impl Encoded {
    pub fn user(&self) -> Option<User> {
        let source = self.source.as_ref()?;

        match source {
            proto::Source::User(user) => Some(User::from(user.clone())),
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {
    Server {
        source: Source,
    },
    Channel {
        channel: Channel,
        source: Source,
        prefix: Option<char>,
    },
    Query {
        nick: Nick,
        source: Source,
    },
    Logs,
    Highlights {
        server: Server,
        channel: Channel,
        source: Source,
    },
}

impl Target {
    pub fn prefix(&self) -> Option<&char> {
        match self {
            Target::Server { .. } => None,
            Target::Channel { prefix, .. } => prefix.as_ref(),
            Target::Query { .. } => None,
            Target::Logs => None,
            Target::Highlights { .. } => None,
        }
    }

    pub fn source(&self) -> &Source {
        match self {
            Target::Server { source } => source,
            Target::Channel { source, .. } => source,
            Target::Query { source, .. } => source,
            Target::Logs => &Source::Internal(source::Internal::Logs),
            Target::Highlights { source, .. } => source,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Direction {
    Sent,
    Received,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub received_at: Posix,
    pub server_time: DateTime<Utc>,
    pub direction: Direction,
    pub target: Target,
    pub content: Content,
    pub id: Option<String>,
    pub hash: Hash,
}

impl Message {
    pub fn triggers_unread(&self) -> bool {
        matches!(self.direction, Direction::Received)
            && match self.target.source() {
                Source::User(_) => true,
                Source::Action => true,
                Source::Server(Some(server)) => {
                    matches!(
                        server.kind(),
                        source::server::Kind::MonitoredOnline
                            | source::server::Kind::MonitoredOffline
                    )
                }
                Source::Internal(source::Internal::Logs) => true,
                _ => false,
            }
    }

    pub fn received<'a>(
        encoded: Encoded,
        our_nick: Nick,
        config: &'a Config,
        resolve_attributes: impl Fn(&User, &str) -> Option<User>,
        channel_users: impl Fn(&str) -> &'a [User],
    ) -> Option<Message> {
        let server_time = server_time(&encoded);
        let id = message_id(&encoded);
        let content = content(
            &encoded,
            &our_nick,
            config,
            &resolve_attributes,
            &channel_users,
        )?;
        let target = target(encoded, &our_nick, &resolve_attributes)?;
        let received_at = Posix::now();
        let hash = Hash::new(&received_at, &content);

        Some(Message {
            received_at,
            server_time,
            direction: Direction::Received,
            target,
            content,
            id,
            hash,
        })
    }

    pub fn sent(target: Target, content: Content) -> Self {
        let received_at = Posix::now();
        let hash = Hash::new(&received_at, &content);

        Message {
            received_at,
            server_time: Utc::now(),
            direction: Direction::Sent,
            target,
            content,
            id: None,
            hash,
        }
    }

    pub fn file_transfer_request_received(from: &Nick, filename: &str) -> Message {
        let received_at = Posix::now();
        let content = plain(format!("{from} wants to send you \"{filename}\""));
        let hash = Hash::new(&received_at, &content);

        Message {
            received_at,
            server_time: Utc::now(),
            direction: Direction::Received,
            target: Target::Query {
                nick: from.clone(),
                source: Source::Action,
            },
            content,
            id: None,
            hash,
        }
    }

    pub fn file_transfer_request_sent(to: &Nick, filename: &str) -> Message {
        let received_at = Posix::now();
        let content = plain(format!("offering to send {to} \"{filename}\""));
        let hash = Hash::new(&received_at, &content);

        Message {
            received_at,
            server_time: Utc::now(),
            direction: Direction::Sent,
            target: Target::Query {
                nick: to.clone(),
                source: Source::Action,
            },
            content,
            id: None,
            hash,
        }
    }

    pub fn with_target(self, target: Target) -> Self {
        Self { target, ..self }
    }

    pub fn plain(&self) -> Option<&str> {
        match &self.content {
            Content::Plain(s) => Some(s),
            Content::Fragments(_) => None,
            Content::Log(_) => None,
        }
    }

    pub fn log(record: crate::log::Record) -> Self {
        let received_at = Posix::now();
        let server_time = record.timestamp;
        let content = Content::Log(record);
        let hash = Hash::new(&received_at, &content);

        Self {
            received_at,
            server_time,
            direction: Direction::Received,
            target: Target::Logs,
            content,
            id: None,
            hash,
        }
    }

    pub fn into_highlight(mut self, server: Server) -> Option<Self> {
        self.target = match self.target {
            Target::Channel {
                channel,
                source: Source::User(user),
                ..
            } => Target::Highlights {
                server,
                channel,
                source: Source::User(user),
            },
            _ => return None,
        };

        Some(self)
    }
}

impl Serialize for Message {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct Data<'a> {
            received_at: &'a Posix,
            server_time: &'a DateTime<Utc>,
            direction: &'a Direction,
            target: &'a Target,
            content: &'a Content,
            // Old field before we had fragments,
            // added for downgrade compatability
            text: Cow<'a, str>,
        }

        Data {
            received_at: &self.received_at,
            server_time: &self.server_time,
            direction: &self.direction,
            target: &self.target,
            content: &self.content,
            text: self.content.text(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Message {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Data {
            received_at: Posix,
            server_time: DateTime<Utc>,
            direction: Direction,
            target: Target,
            // New field, optional for upgrade compatability
            #[serde(default, deserialize_with = "fail_as_none")]
            content: Option<Content>,
            // Old field before we had fragments
            text: Option<String>,
            id: Option<String>,
        }

        let Data {
            received_at,
            server_time,
            direction,
            target,
            content,
            text,
            id,
        } = Data::deserialize(deserializer)?;

        let content = if let Some(content) = content {
            content
        } else if let Some(text) = text {
            // First time upgrading, convert text into content
            parse_fragments(text, &[])
        } else {
            // Unreachable
            Content::Plain("".to_string())
        };

        let hash = Hash::new(&received_at, &content);

        Ok(Message {
            received_at,
            server_time,
            direction,
            target,
            content,
            id,
            hash,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hash(u64);

impl Hash {
    pub fn new(received_at: &time::Posix, content: &Content) -> Self {
        let mut hasher = DefaultHasher::new();
        received_at.hash(&mut hasher);
        content.hash(&mut hasher);
        Self(hasher.finish())
    }
}

pub fn plain(text: String) -> Content {
    Content::Plain(text)
}

pub fn parse_fragments(text: String, channel_users: &[User]) -> Content {
    let fragments = parse_url_fragments(text)
        .into_iter()
        .flat_map(|fragment| {
            if let Fragment::Text(text) = &fragment {
                if let Some(formatted) = formatting::parse(text) {
                    return Either::Left(formatted.into_iter().map(Fragment::from));
                }
            }

            Either::Right(iter::once(fragment))
        })
        .flat_map(|fragment| {
            if let Fragment::Text(text) = &fragment {
                return Either::Left(
                    parse_user_and_channel_fragments(text, channel_users).into_iter(),
                );
            }

            Either::Right(iter::once(fragment))
        })
        .collect::<Vec<_>>();

    if fragments.len() == 1 && matches!(&fragments[0], Fragment::Text(_)) {
        let Some(Fragment::Text(text)) = fragments.into_iter().next() else {
            unreachable!();
        };

        Content::Plain(text)
    } else {
        Content::Fragments(fragments)
    }
}

fn parse_url_fragments(text: String) -> Vec<Fragment> {
    let mut i = 0;
    let mut fragments = Vec::with_capacity(1);

    for (re_match, url) in URL_REGEX.find_iter(&text).filter_map(|re_match| {
        let url = if re_match.as_str().starts_with("www") {
            format!("https://{}", re_match.as_str())
        } else {
            re_match.as_str().to_string()
        };

        Url::parse(&url).ok().map(|url| (re_match, url))
    }) {
        if i < re_match.start() {
            fragments.push(Fragment::Text(text[i..re_match.start()].to_string()));
        }
        i = re_match.end();
        fragments.push(Fragment::Url(url));
    }

    if i == 0 {
        fragments.push(Fragment::Text(text));
    } else {
        fragments.push(Fragment::Text(text[i..text.len()].to_string()));
    }

    fragments
}

/// Checks if a given `text` contains or matches a user's nickname.
fn text_references_nickname(text: &str, nickname: NickRef) -> Option<bool> {
    // TODO: Consider server case-mapping settings vs just ascii lowercase
    let nick = nickname.as_ref();
    let nick_lower = nick.to_ascii_lowercase();
    let lower = text.to_ascii_lowercase();
    let trimmed = text.trim_matches(|c: char| c.is_ascii_punctuation());
    let lower_trimmed = trimmed.to_ascii_lowercase();

    if nick == text || nick_lower == lower {
        // Contains the user's nickname without trimming.
        Some(false)
    } else if nick == trimmed || nick_lower == lower_trimmed {
        // Contains the user's nickname with trimming.
        Some(true)
    } else {
        // Doesn't contain the user's nickname.
        None
    }
}

fn parse_user_and_channel_fragments(text: &str, channel_users: &[User]) -> Vec<Fragment> {
    text.chars()
        .group_by(|c| c.is_whitespace())
        .into_iter()
        .flat_map(|(is_whitespace, chars)| {
            let text = chars.collect::<String>();
            if !is_whitespace {
                if let Some((is_trimmed, user)) = channel_users.iter().find_map(|user| {
                    text_references_nickname(text.as_str(), user.nickname())
                        .map(|is_trimmed| (is_trimmed, user.clone()))
                }) {
                    if is_trimmed {
                        let prefix_end = text.find(|c: char| !c.is_ascii_punctuation());
                        let suffix_start = text
                            .rfind(|c: char| !c.is_ascii_punctuation())
                            .map(|i| i + 1)
                            .filter(|i| *i < text.len());
                        let middle = prefix_end.unwrap_or(0)..suffix_start.unwrap_or(text.len());

                        return Either::Right(
                            prefix_end
                                .map(|i| Fragment::Text(text[0..i].to_string()))
                                .into_iter()
                                .chain(Some(Fragment::User(user, text[middle].to_string())))
                                .chain(
                                    suffix_start
                                        .map(|i| Fragment::Text(text[i..text.len()].to_string())),
                                ),
                        );
                    } else {
                        return Either::Left(iter::once(Fragment::User(user.clone(), text)));
                    }
                }
                // Only parse on `#` since it's most common and
                // using &!+ leads to more false positives than not
                else if text
                    .strip_prefix('#')
                    .map_or(false, |rest| !rest.is_empty())
                    && !text.contains(proto::CHANNEL_BLACKLIST_CHARS)
                {
                    return Either::Left(iter::once(Fragment::Channel(text)));
                }
            }

            Either::Left(iter::once(Fragment::Text(text)))
        })
        .fold(vec![], |mut acc, fragment| {
            if let Some(Fragment::Text(text)) = acc.last_mut() {
                if let Fragment::Text(next) = &fragment {
                    text.push_str(next);
                    return acc;
                }
            }

            acc.push(fragment);
            acc
        })
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Content {
    Plain(String),
    Fragments(Vec<Fragment>),
    Log(crate::log::Record),
}

impl Content {
    fn text(&self) -> Cow<str> {
        match self {
            Content::Plain(s) => s.into(),
            Content::Fragments(fragments) => fragments.iter().map(Fragment::as_str).join("").into(),
            Content::Log(record) => (&record.message).into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Fragment {
    Text(String),
    Channel(String),
    User(User, String),
    Url(Url),
    Formatted {
        text: String,
        formatting: Formatting,
    },
}

impl Fragment {
    pub fn as_str(&self) -> &str {
        match self {
            Fragment::Text(s) => s,
            Fragment::Channel(c) => c,
            Fragment::User(_, t) => t,
            Fragment::Url(u) => u.as_str(),
            Fragment::Formatted { text, .. } => text,
        }
    }
}

impl From<formatting::Fragment> for Fragment {
    fn from(value: formatting::Fragment) -> Self {
        match value {
            formatting::Fragment::Unformatted(text) => Self::Text(text),
            formatting::Fragment::Formatted(text, formatting) => {
                Self::Formatted { text, formatting }
            }
        }
    }
}

fn target(
    message: Encoded,
    our_nick: &Nick,
    resolve_attributes: &dyn Fn(&User, &str) -> Option<User>,
) -> Option<Target> {
    use proto::command::Numeric::*;

    let user = message.user();

    match message.0.command {
        // Channel
        Command::MODE(target, ..) if proto::is_channel(&target) => Some(Target::Channel {
            channel: target,
            source: source::Source::Server(None),
            prefix: None,
        }),
        Command::TOPIC(channel, _) | Command::KICK(channel, _, _) => Some(Target::Channel {
            channel,
            source: source::Source::Server(None),
            prefix: None,
        }),
        Command::PART(channel, _) => Some(Target::Channel {
            channel,
            source: source::Source::Server(Some(source::Server::new(
                source::server::Kind::Part,
                Some(user?.nickname().to_owned()),
            ))),
            prefix: None,
        }),
        Command::JOIN(channel, _) => Some(Target::Channel {
            channel,
            source: source::Source::Server(Some(source::Server::new(
                source::server::Kind::Join,
                Some(user?.nickname().to_owned()),
            ))),
            prefix: None,
        }),
        Command::Numeric(RPL_TOPIC | RPL_TOPICWHOTIME, params) => {
            let channel = params.get(1)?.clone();
            Some(Target::Channel {
                channel,
                source: source::Source::Server(Some(source::Server::new(
                    source::server::Kind::ReplyTopic,
                    None,
                ))),
                prefix: None,
            })
        }
        Command::Numeric(RPL_CHANNELMODEIS, params) => {
            let channel = params.get(1)?.clone();
            Some(Target::Channel {
                channel,
                source: source::Source::Server(None),
                prefix: None,
            })
        }
        Command::Numeric(RPL_AWAY, params) => {
            let user = params.get(1)?;
            let target = User::try_from(user.as_str()).ok()?;

            Some(Target::Query {
                nick: target.nickname().to_owned(),
                source: Source::Action,
            })
        }
        Command::PRIVMSG(target, text) => {
            let is_action = is_action(&text);
            let source = |user| {
                if is_action {
                    Source::Action
                } else {
                    Source::User(user)
                }
            };

            match (proto::parse_channel_from_target(&target), user) {
                (Some((prefix, channel)), Some(user)) => {
                    let source = source(resolve_attributes(&user, &channel).unwrap_or(user));
                    Some(Target::Channel {
                        channel,
                        source,
                        prefix,
                    })
                }
                (None, Some(user)) => {
                    let (nick, source) = if user.nickname() == *our_nick {
                        // Message from ourself, from another client.
                        let target = User::try_from(target.as_str()).ok()?;
                        (target.nickname().to_owned(), source(user))
                    } else {
                        // Message from conversation partner.
                        (user.nickname().to_owned(), source(user))
                    };

                    Some(Target::Query { nick, source })
                }
                _ => None,
            }
        }
        Command::NOTICE(target, text) => {
            let is_action = is_action(&text);
            let source = |user| {
                if is_action {
                    Source::Action
                } else {
                    Source::User(user)
                }
            };

            match (proto::parse_channel_from_target(&target), user) {
                (Some((prefix, channel)), Some(user)) => {
                    let source = source(resolve_attributes(&user, &channel).unwrap_or(user));
                    Some(Target::Channel {
                        channel,
                        source,
                        prefix,
                    })
                }
                (None, Some(user)) => {
                    let target = User::try_from(target.as_str()).ok()?;

                    (target.nickname() == *our_nick).then(|| Target::Query {
                        nick: user.nickname().to_owned(),
                        source: source(user),
                    })
                }
                _ => Some(Target::Server {
                    source: Source::Server(None),
                }),
            }
        }
        Command::CHGHOST(_, _) => Some(Target::Server {
            source: source::Source::Server(Some(source::Server::new(
                source::server::Kind::ChangeHost,
                user.map(|user| user.nickname().to_owned()),
            ))),
        }),
        Command::Numeric(RPL_MONONLINE, _) => Some(Target::Server {
            source: source::Source::Server(Some(source::Server::new(
                source::server::Kind::MonitoredOnline,
                None,
            ))),
        }),
        Command::Numeric(RPL_MONOFFLINE, _) => Some(Target::Server {
            source: source::Source::Server(Some(source::Server::new(
                source::server::Kind::MonitoredOffline,
                None,
            ))),
        }),

        // Server
        Command::PASS(_)
        | Command::NICK(_)
        | Command::USER(_, _)
        | Command::OPER(_, _)
        | Command::QUIT(_)
        | Command::SQUIT(_, _)
        | Command::NAMES(_)
        | Command::LIST(_, _)
        | Command::INVITE(_, _)
        | Command::MOTD(_)
        | Command::LUSERS
        | Command::VERSION(_)
        | Command::STATS(_, _)
        | Command::LINKS
        | Command::TIME(_)
        | Command::CONNECT(_, _, _)
        | Command::ADMIN(_)
        | Command::INFO
        | Command::WHO(_, _, _)
        | Command::WHOIS(_, _)
        | Command::WHOWAS(_, _)
        | Command::KILL(_, _)
        | Command::PING(_)
        | Command::PONG(_, _)
        | Command::ERROR(_)
        | Command::AWAY(_)
        | Command::REHASH
        | Command::RESTART
        | Command::WALLOPS(_)
        | Command::USERHOST(_)
        | Command::CAP(_, _, _, _)
        | Command::AUTHENTICATE(_)
        | Command::ACCOUNT(_)
        | Command::BATCH(_, _)
        | Command::CNOTICE(_, _, _)
        | Command::CPRIVMSG(_, _, _)
        | Command::KNOCK(_, _)
        | Command::MARKREAD(_, _)
        | Command::MONITOR(_, _)
        | Command::TAGMSG(_)
        | Command::USERIP(_)
        | Command::HELP(_)
        | Command::MODE(_, _, _)
        | Command::Numeric(_, _)
        | Command::Unknown(_, _)
        | Command::Raw(_) => Some(Target::Server {
            source: Source::Server(None),
        }),
    }
}

pub fn message_id(message: &Encoded) -> Option<String> {
    message
        .tags
        .iter()
        .find(|tag| &tag.key == "msgid")
        .and_then(|tag| tag.value.clone())
}

pub fn server_time(message: &Encoded) -> DateTime<Utc> {
    message
        .tags
        .iter()
        .find(|tag| &tag.key == "time")
        .and_then(|tag| tag.value.clone())
        .and_then(|rfc3339| DateTime::parse_from_rfc3339(&rfc3339).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now)
}

fn content<'a>(
    message: &Encoded,
    our_nick: &Nick,
    config: &Config,
    resolve_attributes: &dyn Fn(&User, &str) -> Option<User>,
    channel_users: &dyn Fn(&str) -> &'a [User],
) -> Option<Content> {
    use irc::proto::command::Numeric::*;

    match &message.command {
        Command::TOPIC(target, topic) => {
            let raw_user = message.user()?;
            let user = resolve_attributes(&raw_user, target).unwrap_or(raw_user);

            let topic = topic.as_ref()?;
            let with_access_levels = config.buffer.nickname.show_access_levels;
            let user = user.display(with_access_levels);

            Some(parse_fragments(
                format!("{user} changed topic to {topic}"),
                &[],
            ))
        }
        Command::PART(target, text) => {
            let raw_user = message.user()?;
            let user = resolve_attributes(&raw_user, target)
                .unwrap_or(raw_user)
                .formatted(config.buffer.server_messages.part.username_format);

            let text = text
                .as_ref()
                .map(|text| format!(" ({text})"))
                .unwrap_or_default();

            Some(parse_fragments(
                format!("⟵ {user} has left the channel{text}"),
                &[],
            ))
        }
        Command::JOIN(target, _) => {
            let raw_user = message.user()?;
            let user = resolve_attributes(&raw_user, target).unwrap_or(raw_user);

            (user.nickname() != *our_nick).then(|| {
                parse_fragments(
                    format!(
                        "⟶ {} has joined the channel",
                        user.formatted(config.buffer.server_messages.join.username_format)
                    ),
                    &[],
                )
            })
        }
        Command::KICK(channel, victim, comment) => {
            let raw_user = message.user()?;
            let with_access_levels = config.buffer.nickname.show_access_levels;
            let user = resolve_attributes(&raw_user, channel)
                .unwrap_or(raw_user)
                .display(with_access_levels);

            let comment = comment
                .as_ref()
                .map(|comment| format!(" ({comment})"))
                .unwrap_or_default();
            let target = if victim == our_nick.as_ref() {
                "you have".to_string()
            } else {
                format!("{victim} has")
            };

            Some(parse_fragments(
                format!("⟵ {target} been kicked by {user}{comment}"),
                &[],
            ))
        }
        Command::MODE(target, modes, args) if proto::is_channel(target) => {
            let raw_user = message.user()?;
            let with_access_levels = config.buffer.nickname.show_access_levels;
            let user = resolve_attributes(&raw_user, target)
                .unwrap_or(raw_user)
                .display(with_access_levels);

            let modes = modes
                .iter()
                .map(|mode| mode.to_string())
                .collect::<Vec<_>>()
                .join(" ");

            let args = args
                .iter()
                .flatten()
                .map(|arg| arg.to_string())
                .collect::<Vec<_>>()
                .join(" ");

            Some(parse_fragments(
                format!("{user} sets mode {modes} {args}"),
                &[],
            ))
        }
        Command::PRIVMSG(target, text) => {
            // Check if a synthetic action message
            if let Some(nick) = message.user().as_ref().map(User::nickname) {
                if let Some(action) = parse_action(nick, text) {
                    return Some(action);
                }
            }

            let channel_users = channel_users(target);
            Some(parse_fragments(text.clone(), channel_users))
        }
        Command::NOTICE(_, text) => Some(parse_fragments(text.clone(), &[])),
        Command::Numeric(RPL_TOPIC, params) => {
            let topic = params.get(2)?;

            Some(parse_fragments(format!("topic is {topic}"), &[]))
        }
        Command::Numeric(RPL_ENDOFWHOIS, _) => {
            // We skip the end message of a WHOIS.
            None
        }
        Command::Numeric(RPL_WHOISIDLE, params) => {
            let nick = params.get(1)?;
            let idle = params.get(2)?.parse::<u64>().ok()?;
            let sign_on = params.get(3)?.parse::<u64>().ok()?;

            let sign_on = Posix::from_seconds(sign_on);
            let sign_on_datetime = sign_on.datetime()?.to_string();

            let mut formatter = timeago::Formatter::new();
            // Remove "ago" from relative time.
            formatter.ago("");

            let duration = std::time::Duration::from_secs(idle);
            let idle_readable = formatter.convert(duration);

            Some(parse_fragments(
                format!(
                    "{nick} signed on at {sign_on_datetime} and has been idle for {idle_readable}"
                ),
                &[],
            ))
        }
        Command::Numeric(RPL_WHOISSERVER, params) => {
            let nick = params.get(1)?;
            let server = params.get(2)?;
            let region = params.get(3)?;

            Some(parse_fragments(
                format!("{nick} is connected on {server} ({region})"),
                &[],
            ))
        }
        Command::Numeric(RPL_WHOISUSER, params) => {
            let nick = params.get(1)?;
            let userhost = format!("{}@{}", params.get(2)?, params.get(3)?);
            let real_name = params.get(5)?;

            Some(parse_fragments(
                format!("{nick} has userhost {userhost} and real name '{real_name}'"),
                &[],
            ))
        }
        Command::Numeric(RPL_WHOISCHANNELS, params) => {
            let nick = params.get(1)?;
            let channels = params.get(2)?;

            Some(parse_fragments(format!("{nick} is in {channels}"), &[]))
        }
        Command::Numeric(RPL_WHOISACTUALLY, params) => {
            let nick = params.get(1)?;
            let ip = params.get(2)?;
            let status_text = params.get(3)?;

            Some(parse_fragments(format!("{nick} {status_text} {ip}"), &[]))
        }
        Command::Numeric(RPL_WHOISSECURE, params) => {
            let nick = params.get(1)?;
            let status_text = params.get(2)?;

            Some(parse_fragments(format!("{nick} {status_text}"), &[]))
        }
        Command::Numeric(RPL_WHOISACCOUNT, params) => {
            let nick = params.get(1)?;
            let account = params.get(2)?;
            let status_text = params.get(3)?;

            Some(parse_fragments(
                format!("{nick} {status_text} {account}"),
                &[],
            ))
        }
        Command::Numeric(RPL_TOPICWHOTIME, params) => {
            let nick = params.get(2)?;
            let datetime = params
                .get(3)?
                .parse::<u64>()
                .ok()
                .map(Posix::from_seconds)
                .as_ref()
                .and_then(Posix::datetime)?
                .to_rfc2822();

            Some(parse_fragments(
                format!("topic set by {nick} at {datetime}"),
                &[],
            ))
        }
        Command::Numeric(RPL_CHANNELMODEIS, params) => {
            let mode = params
                .iter()
                .skip(2)
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(" ");

            Some(parse_fragments(format!("Channel mode is {mode}"), &[]))
        }
        Command::Numeric(RPL_UMODEIS, params) => {
            let mode = params
                .iter()
                .skip(1)
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(" ");

            Some(parse_fragments(format!("User mode is {mode}"), &[]))
        }
        Command::Numeric(RPL_AWAY, params) => {
            let user = params.get(1)?;
            let away_message = params
                .get(2)
                .map(|away| format!(" ({away})"))
                .unwrap_or_default();

            Some(parse_fragments(
                format!("{user} is away{away_message}"),
                &[],
            ))
        }
        Command::Numeric(RPL_MONONLINE, params) => {
            let targets = params
                .get(1)?
                .split(',')
                .filter_map(|target| User::try_from(target).ok())
                .map(|user| user.formatted(UsernameFormat::Full))
                .collect::<Vec<_>>();

            let targets = monitored_targets_text(targets)?;

            Some(plain(format!("Monitored {targets} online")))
        }
        Command::Numeric(RPL_MONOFFLINE, params) => {
            let targets = params
                .get(1)?
                .split(',')
                .map(String::from)
                .collect::<Vec<_>>();

            let targets = monitored_targets_text(targets)?;

            Some(plain(format!("Monitored {targets} offline")))
        }
        Command::Numeric(_, responses) | Command::Unknown(_, responses) => Some(parse_fragments(
            responses
                .iter()
                .map(|s| s.as_str())
                .skip(1)
                .collect::<Vec<_>>()
                .join(" "),
            &[],
        )),
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
    pub const DEFAULT_COUNT: usize = 500;

    pub fn top() -> Self {
        Self::Top(Self::DEFAULT_COUNT)
    }

    pub fn bottom() -> Self {
        Self::Bottom(Self::DEFAULT_COUNT)
    }
}

pub fn is_action(text: &str) -> bool {
    if let Some(query) = ctcp::parse_query(text) {
        matches!(query.command, ctcp::Command::Action)
    } else {
        false
    }
}

fn parse_action(nick: NickRef, text: &str) -> Option<Content> {
    let query = ctcp::parse_query(text)?;

    Some(action_text(nick, query.params))
}

pub fn action_text(nick: NickRef, action: Option<&str>) -> Content {
    if let Some(action) = action {
        parse_fragments(format!("{nick} {action}"), &[])
    } else {
        plain(format!("{nick}"))
    }
}

fn monitored_targets_text(targets: Vec<String>) -> Option<String> {
    let (last_target, targets) = targets.split_last()?;

    if targets.is_empty() {
        Some(format!("user {last_target} is"))
    } else if targets.len() == 1 {
        Some(format!("users {} and {last_target} are", targets.first()?))
    } else {
        Some(format!(
            "users {}, and {last_target} are",
            targets.join(", ")
        ))
    }
}

pub fn references_user(sender: NickRef, own_nick: NickRef, message: &Message) -> bool {
    match &message.content {
        Content::Plain(text) => references_user_text(sender, own_nick, text),
        Content::Fragments(fragments) => fragments
            .iter()
            .any(|f| references_user_text(sender, own_nick, f.as_str())),
        Content::Log(_) => false,
    }
}

pub fn references_user_text(sender: NickRef, own_nick: NickRef, text: &str) -> bool {
    sender != own_nick
        && text
            .chars()
            .group_by(|c| c.is_whitespace())
            .into_iter()
            .any(|(is_whitespace, chars)| {
                if !is_whitespace {
                    let text = chars.collect::<String>();
                    text_references_nickname(&text, own_nick).is_some()
                } else {
                    false
                }
            })
}

#[derive(Debug, Clone)]
pub enum Link {
    Channel(String),
    Url(String),
    User(User),
    GoToMessage(Server, String, Hash),
}

fn fail_as_none<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    // We must fully consume valid json otherwise the error leaves the
    // deserializer in an invalid state and it'll still fail
    //
    // This assumes we always use a json format
    let intermediate = serde_json::Value::deserialize(deserializer)?;

    Ok(Option::<T>::deserialize(intermediate).unwrap_or_default())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn fragment_parsing() {
        let tests = [
            (
                "Checkout https://foo.bar/asdf?1=2 now!",
                vec![
                    Fragment::Text("Checkout ".into()),
                    Fragment::Url("https://foo.bar/asdf?1=2".parse().unwrap()),
                    Fragment::Text(" now!".into()),
                ],
            ),
            (
                "http://google.us.edi?34535/534534?dfg=g&fg",
                vec![Fragment::Url(
                    "http://google.us.edi?34535/534534?dfg=g&fg"
                        .parse()
                        .unwrap(),
                )],
            ),
            (
                "http://regexr.com is a great tool",
                vec![
                    Fragment::Url("http://regexr.com".parse().unwrap()),
                    Fragment::Text(" is a great tool".into()),
                ],
            ),
            (
                "We have a wiki at https://halloy.squidowl.org",
                vec![
                    Fragment::Text("We have a wiki at ".into()),
                    Fragment::Url("https://halloy.squidowl.org".parse().unwrap()),
                ],
            ),
            (
                "https://catgirl.delivery/2024/07/25/sometimes-it-is-correct-to-blame-the-compiler/",
                vec![Fragment::Url(
                    "https://catgirl.delivery/2024/07/25/sometimes-it-is-correct-to-blame-the-compiler/"
                    .parse()
                    .unwrap()
                )],
            ),
            (
                "https://www.google.com/maps/@61.0873595,-27.322408,3z?entry=ttu",
                vec![Fragment::Url(
                    "https://www.google.com/maps/@61.0873595,-27.322408,3z?entry=ttu"
                    .parse()
                    .unwrap()
                )],
            ),
            (
                "https://doc.rust-lang.org/book/ch03-05-control-flow.html#loop-labels-to-disambiguate-between-multiple-loops",
                vec![Fragment::Url(
                    "https://doc.rust-lang.org/book/ch03-05-control-flow.html#loop-labels-to-disambiguate-between-multiple-loops"
                    .parse()
                    .unwrap()
                )],
            ),
            (
                "(https://yt.drgnz.club/watch?v=s_VH36ChGXw and https://invidious.incogniweb.net/watch?v=H3v9unphfi0).",
                vec![
                    Fragment::Text("(".into()),
                    Fragment::Url("https://yt.drgnz.club/watch?v=s_VH36ChGXw".parse().unwrap()),
                    Fragment::Text(" and ".into()),
                    Fragment::Url("https://invidious.incogniweb.net/watch?v=H3v9unphfi0".parse().unwrap()),
                    Fragment::Text(").".into()),
                ],
            ),
            (
                "https://www.reddit.com/r/witze/comments/1fcoz5a/ein_vampir_auf_einem_tandem_gerät_in_eine/",
                vec![Fragment::Url(
                    "https://www.reddit.com/r/witze/comments/1fcoz5a/ein_vampir_auf_einem_tandem_gerät_in_eine/"
                    .parse()
                    .unwrap()
                )],
            ),
            (
                "http://öbb.at",
                vec![
                    Fragment::Url("http://öbb.at".parse().unwrap()),
                ],
            ),
        ];

        for (text, expected) in tests {
            let actual = parse_fragments(text.to_string(), &[]);

            assert_eq!(Content::Fragments(expected), actual);
        }
    }
}
