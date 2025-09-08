use std::borrow::Cow;
use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash as _, Hasher};
use std::iter;
use std::sync::LazyLock;

use chrono::{DateTime, Local, Utc};
use const_format::concatcp;
use fancy_regex::{Regex, RegexBuilder};
use irc::proto;
use irc::proto::Command;
use itertools::{Either, Itertools};
use serde::{Deserialize, Serialize};
use url::Url;

pub use self::broadcast::Broadcast;
pub use self::formatting::{Color, Formatting};
pub use self::source::Source;
pub use self::source::server::{Kind, StandardReply};
use crate::config::Highlights;
use crate::config::buffer::UsernameFormat;
use crate::log::Level;
use crate::serde::fail_as_none;
use crate::server::Server;
use crate::target::Channel;
use crate::time::Posix;
use crate::user::{ChannelUsers, Nick, NickRef};
use crate::{Config, User, ctcp, isupport, target};

// References:
// - https://datatracker.ietf.org/doc/html/rfc1738#section-5
// - https://www.ietf.org/rfc/rfc2396.txt

const URL_PATH_UNRESERVED: &str = r#"\p{Letter}\p{Number}\-_.!~*'()"#;

const URL_PATH_RESERVED: &str = r#";?:@&=+$,"#;

const URL_PATH: &str =
    concatcp!(r#"["#, URL_PATH_UNRESERVED, URL_PATH_RESERVED, r#"%\/#]"#);

const URL_PATH_UNRESERVED_EXC_PUNC: &str = r#"\p{Letter}\p{Number}\-_~*'("#;

const URL_PATH_RESERVED_EXC_PUNC: &str = r#"@&=+$"#;

const URL_PATH_EXC_PUNC: &str = concatcp!(
    r#"["#,
    URL_PATH_UNRESERVED_EXC_PUNC,
    URL_PATH_RESERVED_EXC_PUNC,
    r#"%\/#]"#
);

static URL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
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
    .delegate_size_limit(15728640) // 1.5x default size_limit
    .build()
    .unwrap()
});

static CHANNEL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(r#"(?i)(?<!\w)(#[^ ,\x07]+)(?!\w)"#)
        .build()
        .unwrap()
});

static USER_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(r#"(?i)(?<!\w)([\w"\-\[\]\\`^{|}\/]+)(?!\w)"#)
        .build()
        .unwrap()
});

pub(crate) mod broadcast;
pub mod formatting;
pub mod source;

#[derive(Debug, Clone)]
pub struct Encoded(proto::Message);

impl Encoded {
    pub fn user(&self, casemapping: isupport::CaseMap) -> Option<User> {
        let source = self.source.as_ref()?;

        match source {
            proto::Source::User(user) => {
                Some(User::from_proto_user(user.clone(), casemapping))
            }
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
        channel: target::Channel,
        source: Source,
    },
    Query {
        query: target::Query,
        source: Source,
    },
    Logs {
        source: Source,
    },
    Highlights {
        server: Server,
        channel: target::Channel,
        source: Source,
    },
}

impl Target {
    pub fn prefixes(&self) -> Option<&[char]> {
        match self {
            Target::Server { .. } => None,
            Target::Channel { channel, .. } => {
                if channel.prefixes().is_empty() {
                    None
                } else {
                    Some(channel.prefixes())
                }
            }
            Target::Query { .. } => None,
            Target::Logs { .. } => None,
            Target::Highlights { .. } => None,
        }
    }

    pub fn source(&self) -> &Source {
        match self {
            Target::Server { source } => source,
            Target::Channel { source, .. } => source,
            Target::Query { source, .. } => source,
            Target::Logs { source } => source,
            Target::Highlights { source, .. } => source,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Direction {
    Sent,
    Received,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub received_at: Posix,
    pub server_time: DateTime<Utc>,
    pub direction: Direction,
    pub target: Target,
    pub content: Content,
    pub id: Option<String>,
    pub hash: Hash,
    pub hidden_urls: HashSet<Url>,
    pub is_echo: bool,
    pub blocked: bool,
}

impl Message {
    pub fn triggers_unread(&self) -> bool {
        matches!(self.direction, Direction::Received)
            && !self.is_echo
            && match self.target.source() {
                Source::User(_) => true,
                Source::Action(_) => true,
                Source::Server(Some(server)) => {
                    matches!(
                        server.kind(),
                        Kind::MonitoredOnline
                            | Kind::MonitoredOffline
                            | Kind::StandardReply(_)
                            | Kind::WAllOps
                            | Kind::Kick
                    )
                }
                Source::Internal(source::Internal::Logs(level)) => {
                    match level {
                        Level::Warn | Level::Error => true,
                        Level::Info | Level::Debug | Level::Trace => false,
                    }
                }
                _ => false,
            }
    }

    pub fn triggers_highlight(&self) -> bool {
        matches!(self.direction, Direction::Received)
            && !self.is_echo
            && self.highlight_description().is_some()
    }

    pub fn can_reference(&self) -> bool {
        if matches!(self.direction, Direction::Sent)
            || self.is_echo
            || matches!(self.target.source(), Source::Internal(_))
        {
            return false;
        } else if let Source::Server(Some(source)) = self.target.source()
            && matches!(
                source.kind(),
                Kind::ReplyTopic
                    | Kind::MonitoredOnline
                    | Kind::MonitoredOffline
                    | Kind::ChangeHost
            )
        {
            return false;
        }

        true
    }

    pub fn references(&self) -> MessageReferences {
        MessageReferences {
            timestamp: self.server_time,
            id: self.id.clone(),
        }
    }

    pub fn received<'a>(
        encoded: Encoded,
        our_nick: Nick,
        config: &'a Config,
        resolve_attributes: impl Fn(&User, &target::Channel) -> Option<User>,
        channel_users: impl Fn(&target::Channel) -> Option<&'a ChannelUsers>,
        chantypes: &[char],
        statusmsg: &[char],
        casemapping: isupport::CaseMap,
        prefix: &[isupport::PrefixMap],
    ) -> Option<Message> {
        let server_time = server_time(&encoded);
        let id = message_id(&encoded);
        let is_echo = encoded
            .user(casemapping)
            .is_some_and(|user| user.nickname() == our_nick);
        let content = content(
            &encoded,
            &our_nick,
            config,
            &resolve_attributes,
            &channel_users,
            chantypes,
            statusmsg,
            casemapping,
            prefix,
        )?;
        let target = target(
            encoded,
            &our_nick,
            &resolve_attributes,
            chantypes,
            statusmsg,
            casemapping,
        )?;
        let received_at = Posix::now();
        let hash = Hash::new(&server_time, &content);

        Some(Message {
            received_at,
            server_time,
            direction: Direction::Received,
            target,
            content,
            id,
            hash,
            hidden_urls: HashSet::default(),
            is_echo,
            blocked: false,
        })
    }

    pub fn sent(target: Target, content: Content) -> Self {
        let received_at = Posix::now();
        let server_time = Utc::now();
        let hash = Hash::new(&server_time, &content);

        Message {
            received_at,
            server_time,
            direction: Direction::Sent,
            target,
            content,
            id: None,
            hash,
            hidden_urls: HashSet::default(),
            is_echo: false,
            blocked: false,
        }
    }

    pub fn file_transfer_request_received(
        from: &User,
        query: &target::Query,
        filename: &str,
    ) -> Message {
        let received_at = Posix::now();
        let server_time = Utc::now();
        let content = plain(format!(
            "{} wants to send you \"{filename}\"",
            from.nickname()
        ));
        let hash = Hash::new(&server_time, &content);

        Message {
            received_at,
            server_time,
            direction: Direction::Received,
            target: Target::Query {
                query: query.clone(),
                source: Source::Action(None),
            },
            content,
            id: None,
            hash,
            hidden_urls: HashSet::default(),
            is_echo: false,
            blocked: false,
        }
    }

    pub fn file_transfer_request_sent(
        to: &User,
        query: &target::Query,
        filename: &str,
    ) -> Message {
        let received_at = Posix::now();
        let server_time = Utc::now();
        let content =
            plain(format!("offering to send {} \"{filename}\"", to.nickname()));
        let hash = Hash::new(&server_time, &content);

        Message {
            received_at,
            server_time,
            direction: Direction::Sent,
            target: Target::Query {
                query: query.clone(),
                source: Source::Action(None),
            },
            content,
            id: None,
            hash,
            hidden_urls: HashSet::default(),
            is_echo: false,
            blocked: false,
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

    pub fn text(&self) -> String {
        self.content.text().to_string()
    }

    pub fn log(record: crate::log::Record) -> Self {
        let received_at = Posix::now();
        let server_time = record.timestamp;
        let target = Target::Logs {
            source: Source::Internal(source::Internal::Logs(record.level)),
        };
        let content = Content::Log(record);
        let hash = Hash::new(&server_time, &content);

        Self {
            received_at,
            server_time,
            direction: Direction::Received,
            target,
            content,
            id: None,
            hash,
            hidden_urls: HashSet::default(),
            is_echo: false,
            blocked: false,
        }
    }

    fn highlight_description(&self) -> Option<String> {
        if let Content::Fragments(fragments) = &self.content {
            if fragments.iter().any(|fragment| {
                matches!(fragment, Fragment::HighlightNick(_, _))
            }) {
                Some(String::from("highlighted you"))
            } else {
                fragments.iter().find_map(|fragment| {
                    if let Fragment::HighlightMatch(text) = fragment {
                        Some(format!("matched highlight {text}"))
                    } else {
                        None
                    }
                })
            }
        } else {
            None
        }
    }

    pub fn into_highlight(
        &self,
        server: Server,
    ) -> Option<(Self, Channel, User, String)> {
        if !self.is_echo
            && let Some(highlight_description) = self.highlight_description()
        {
            let (channel, user, source) = match self.target.clone() {
                Target::Channel {
                    channel,
                    source: Source::User(user),
                    ..
                } => (channel, user.clone(), Source::User(user)),
                Target::Channel {
                    channel,
                    source: Source::Action(Some(user)),
                    ..
                } => (channel, user.clone(), Source::Action(Some(user))),
                _ => return None,
            };

            let message = Message {
                target: Target::Highlights {
                    server,
                    channel: channel.clone(),
                    source,
                },
                ..self.clone()
            };

            return Some((message, channel, user, highlight_description));
        }

        None
    }
}

// When changing how Message (or its constituent parts) is serialized, run
// `data/scripts/generate-message-tests-json.sh` to produce test messages for
// backwards compatibility tests
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
            id: &'a Option<String>,
            // Old field before we had fragments,
            // added for downgrade compatibility
            text: Cow<'a, str>,
            hidden_urls: &'a HashSet<url::Url>,
            is_echo: &'a bool,
        }

        Data {
            received_at: &self.received_at,
            server_time: &self.server_time,
            direction: &self.direction,
            target: &self.target,
            content: &self.content,
            id: &self.id,
            text: self.content.text(),
            hidden_urls: &self.hidden_urls,
            is_echo: &self.is_echo,
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
            // New field, optional for upgrade compatibility
            #[serde(default, deserialize_with = "fail_as_none")]
            content: Option<Content>,
            // Old field before we had fragments
            text: Option<String>,
            id: Option<String>,
            #[serde(default)]
            hidden_urls: HashSet<url::Url>,
            // New field, optional for upgrade compatibility
            #[serde(default, deserialize_with = "fail_as_none")]
            is_echo: Option<bool>,
        }

        let Data {
            received_at,
            server_time,
            direction,
            target,
            content,
            text,
            id,
            hidden_urls,
            is_echo,
        } = Data::deserialize(deserializer)?;

        let content = if let Some(content) = content {
            content
        } else if let Some(text) = text {
            // First time upgrading, convert text into content
            parse_fragments(text)
        } else {
            // Unreachable
            Content::Plain(String::new())
        };

        let is_echo = is_echo.unwrap_or_default();

        let hash = Hash::new(&server_time, &content);

        Ok(Message {
            received_at,
            server_time,
            direction,
            target,
            content,
            id,
            hash,
            hidden_urls,
            is_echo,
            blocked: false,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hash(u64);

impl Hash {
    pub fn new(server_time: &DateTime<Utc>, content: &Content) -> Self {
        let mut hasher = DefaultHasher::new();
        server_time.hash(&mut hasher);
        content.hash(&mut hasher);
        Self(hasher.finish())
    }
}

pub fn plain(text: String) -> Content {
    Content::Plain(text)
}

pub fn parse_fragments_with_highlights(
    text: String,
    channel_users: Option<&ChannelUsers>,
    target: &str,
    our_nick: Option<&Nick>,
    highlights: &Highlights,
    casemapping: isupport::CaseMap,
) -> Content {
    let mut fragments =
        parse_fragments_with_users_inner(text, channel_users, casemapping)
            .map(|fragment| match fragment {
                Fragment::User(user, raw)
                    if highlights.nickname.is_target_included(target)
                        && ((our_nick
                            .is_some_and(|nick| user.nickname() == *nick)
                            && highlights.nickname.case_insensitive)
                            || (our_nick.is_some_and(|nick| {
                                raw.as_str() == nick.as_str()
                            }))) =>
                {
                    Fragment::HighlightNick(user, raw)
                }
                f => f,
            })
            .collect::<Vec<_>>();

    for regex in highlights
        .matches
        .iter()
        .filter_map(|m| m.is_target_included(target).then_some(&m.regex))
    {
        fragments = fragments
            .into_iter()
            .flat_map(|fragment| {
                if let Fragment::Text(text) = &fragment {
                    return Either::Left(
                        parse_regex_fragments(regex, text, |text| {
                            Some(Fragment::HighlightMatch(text.to_owned()))
                        })
                        .into_iter(),
                    );
                }

                Either::Right(iter::once(fragment))
            })
            .collect();
    }

    if fragments.len() == 1 && matches!(&fragments[0], Fragment::Text(_)) {
        let Some(Fragment::Text(text)) = fragments.into_iter().next() else {
            unreachable!();
        };

        Content::Plain(text)
    } else {
        Content::Fragments(fragments)
    }
}

pub fn parse_fragments_with_user(
    text: String,
    user: &User,
    casemapping: isupport::CaseMap,
) -> Content {
    // XXX(pounce) annoying clone. Cow somewhere?
    parse_fragments_with_users(
        text,
        Some(&[user.clone()].into_iter().collect()),
        casemapping,
    )
}

pub fn parse_fragments_with_users(
    text: String,
    channel_users: Option<&ChannelUsers>,
    casemapping: isupport::CaseMap,
) -> Content {
    let fragments =
        parse_fragments_with_users_inner(text, channel_users, casemapping)
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

pub fn parse_fragments(text: String) -> Content {
    let fragments = parse_fragments_inner(text).collect::<Vec<_>>();

    if fragments.len() == 1 && matches!(&fragments[0], Fragment::Text(_)) {
        let Some(Fragment::Text(text)) = fragments.into_iter().next() else {
            unreachable!();
        };

        Content::Plain(text)
    } else {
        Content::Fragments(fragments)
    }
}

fn parse_fragments_with_users_inner(
    text: String,
    channel_users: Option<&ChannelUsers>,
    casemapping: isupport::CaseMap,
) -> impl Iterator<Item = Fragment> + use<'_> {
    parse_fragments_inner(text).flat_map(move |fragment| {
        if let Fragment::Text(text) = &fragment {
            return Either::Left(
                parse_regex_fragments(&USER_REGEX, text, |text| {
                    channel_users?
                        .get_by_nick(
                            Nick::from_str(text, casemapping).as_nickref(),
                        )
                        .map(|user| {
                            Fragment::User(user.clone(), text.to_owned())
                        })
                })
                .into_iter(),
            );
        }

        Either::Right(iter::once(fragment))
    })
}

fn parse_fragments_inner<'a>(
    text: String,
) -> impl Iterator<Item = Fragment> + use<'a> {
    let mut modifiers = HashSet::new();
    let mut fg = None;
    let mut bg = None;

    parse_regex_fragments(&URL_REGEX, text, |url| {
        let url = if url.starts_with("www") {
            format!("https://{url}")
        } else {
            url.to_string()
        };

        Url::parse(&url).ok().map(Fragment::Url)
    })
    .into_iter()
    .flat_map(|fragment| {
        if let Fragment::Text(text) = &fragment {
            return Either::Left(
                parse_regex_fragments(&CHANNEL_REGEX, text, |channel| {
                    Some(Fragment::Channel(channel.to_owned()))
                })
                .into_iter(),
            );
        }

        Either::Right(iter::once(fragment))
    })
    .flat_map(move |fragment| {
        if let Fragment::Text(text) = &fragment {
            if let Some(fragments) =
                formatting::parse(text, &mut modifiers, &mut fg, &mut bg)
            {
                if fragments.is_empty() {
                    return Either::Right(Either::Left(iter::empty()));
                }

                if fragments.iter().any(|fragment| {
                    matches!(fragment, formatting::Fragment::Formatted(_, _))
                }) {
                    return Either::Left(
                        fragments.into_iter().map(Fragment::from),
                    );
                // If there are no Formatted fragments,
                // then fragments should contain a single Unformatted fragment
                } else if let Some(text) = fragments
                    .into_iter()
                    .next()
                    .and_then(|fragment| match fragment {
                        formatting::Fragment::Unformatted(text) => Some(text),
                        formatting::Fragment::Formatted(_, _) => None,
                    })
                {
                    // Even if the fragment is Unformatted there may have been formatting
                    // characters in the text input into formatting::parse. They are
                    // stripped from the text contained in the fragment.
                    return Either::Right(Either::Right(iter::once(
                        Fragment::Text(text),
                    )));
                }
            } else if text.is_empty() {
                return Either::Right(Either::Left(iter::empty()));
            } else {
                return Either::Right(Either::Right(iter::once(
                    Fragment::Text(text.clone()),
                )));
            }
        }

        Either::Right(Either::Right(iter::once(fragment)))
    })
}

fn parse_regex_fragments<'a>(
    regex: &Regex,
    text: impl Into<Cow<'a, str>>,
    f: impl Fn(&str) -> Option<Fragment>,
) -> Vec<Fragment> {
    let text: Cow<'a, str> = text.into();

    let mut i = 0;
    let mut fragments = Vec::with_capacity(1);

    for (re_match, fragment) in regex.find_iter(&text).filter_map(|result| {
        result.ok().and_then(|re_match| {
            (f)(re_match.as_str()).map(|fragment| (re_match, fragment))
        })
    }) {
        if i < re_match.start() {
            fragments
                .push(Fragment::Text(text[i..re_match.start()].to_string()));
        }
        i = re_match.end();
        fragments.push(fragment);
    }

    if i == 0 {
        fragments.push(Fragment::Text(text.into_owned()));
    } else {
        fragments.push(Fragment::Text(text[i..text.len()].to_string()));
    }

    fragments
}

#[derive(Debug, Clone, Eq, Serialize, Deserialize)]
pub enum Content {
    Plain(String),
    Fragments(Vec<Fragment>),
    Log(crate::log::Record),
}

impl Content {
    fn text(&self) -> Cow<'_, str> {
        match self {
            Content::Plain(s) => s.into(),
            Content::Fragments(fragments) => {
                fragments.iter().map(Fragment::as_str).join("").into()
            }
            Content::Log(record) => (&record.message).into(),
        }
    }

    pub fn echo_cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.text().trim_end().cmp(other.text().trim_end())
    }
}

impl PartialEq for Content {
    fn eq(&self, other: &Self) -> bool {
        self.text() == other.text()
    }
}

impl std::hash::Hash for Content {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.text().hash(state);
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
    HighlightNick(User, String),
    HighlightMatch(String),
}

impl Fragment {
    pub fn url(&self) -> Option<&Url> {
        if let Self::Url(url) = self {
            Some(url)
        } else {
            None
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Fragment::Text(s) => s,
            Fragment::Channel(c) => c,
            Fragment::User(_, t) => t,
            Fragment::Url(u) => u.as_str(),
            Fragment::Formatted { text, .. } => text,
            Fragment::HighlightNick(_, s) => s,
            Fragment::HighlightMatch(s) => s,
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
    resolve_attributes: &dyn Fn(&User, &target::Channel) -> Option<User>,
    chantypes: &[char],
    statusmsg: &[char],
    casemapping: isupport::CaseMap,
) -> Option<Target> {
    use proto::command::Numeric::*;

    let user = message.user(casemapping);

    match message.0.command {
        // Channel
        Command::MODE(target, ..) => {
            if let Ok(channel) = target::Channel::parse(
                &target,
                chantypes,
                statusmsg,
                casemapping,
            ) {
                Some(Target::Channel {
                    channel,
                    source: Source::Server(Some(source::Server::new(
                        Kind::ChangeMode,
                        Some(user?.nickname().to_owned()),
                    ))),
                })
            } else {
                Some(Target::Server {
                    source: Source::Server(None),
                })
            }
        }
        Command::TOPIC(channel, _) => {
            let channel = target::Channel::parse(
                &channel,
                chantypes,
                statusmsg,
                casemapping,
            )
            .ok()?;

            Some(Target::Channel {
                channel,
                source: Source::Server(None),
            })
        }
        Command::KICK(channel, _, _) => {
            let channel = target::Channel::parse(
                &channel,
                chantypes,
                statusmsg,
                casemapping,
            )
            .ok()?;

            Some(Target::Channel {
                channel,
                source: Source::Server(Some(source::Server::new(
                    Kind::Kick,
                    Some(user?.nickname().to_owned()),
                ))),
            })
        }
        Command::PART(channel, _) => {
            let channel = target::Channel::parse(
                &channel,
                chantypes,
                statusmsg,
                casemapping,
            )
            .ok()?;

            Some(Target::Channel {
                channel,
                source: Source::Server(Some(source::Server::new(
                    Kind::Part,
                    Some(user?.nickname().to_owned()),
                ))),
            })
        }
        Command::JOIN(channel, _) => {
            let channel = target::Channel::parse(
                &channel,
                chantypes,
                statusmsg,
                casemapping,
            )
            .ok()?;

            Some(Target::Channel {
                channel,
                source: Source::Server(Some(source::Server::new(
                    Kind::Join,
                    Some(user?.nickname().to_owned()),
                ))),
            })
        }
        Command::Numeric(RPL_TOPIC | RPL_TOPICWHOTIME, params) => {
            let channel = target::Channel::parse(
                params.get(1)?,
                chantypes,
                statusmsg,
                casemapping,
            )
            .ok()?;
            Some(Target::Channel {
                channel,
                source: Source::Server(Some(source::Server::new(
                    Kind::ReplyTopic,
                    None,
                ))),
            })
        }
        Command::Numeric(RPL_CHANNELMODEIS, params) => {
            let channel = target::Channel::parse(
                params.get(1)?,
                chantypes,
                statusmsg,
                casemapping,
            )
            .ok()?;
            Some(Target::Channel {
                channel,
                source: Source::Server(None),
            })
        }
        Command::Numeric(RPL_AWAY, params) => {
            let query = target::Query::parse(
                params.get(1)?,
                chantypes,
                statusmsg,
                casemapping,
            )
            .ok()?;

            Some(Target::Query {
                query,
                source: Source::Action(None),
            })
        }
        Command::PRIVMSG(target, text) | Command::NOTICE(target, text) => {
            let is_action = is_action(&text);

            // CTCP Handling.
            if ctcp::is_query(&text) && !is_action {
                let user = user?;
                let target = User::from(Nick::from_string(target, casemapping));

                // We want to show both requests, and responses in query with the client.
                let user = if user.nickname() == *our_nick {
                    target
                } else {
                    user
                };

                Some(Target::Query {
                    query: target::Query::from(user),
                    source: Source::Server(None),
                })
            } else {
                let source = |user| {
                    if is_action {
                        Source::Action(Some(user))
                    } else {
                        Source::User(user)
                    }
                };

                match (
                    target::Target::parse(
                        &target,
                        chantypes,
                        statusmsg,
                        casemapping,
                    ),
                    user,
                ) {
                    (target::Target::Channel(channel), Some(user)) => {
                        let source = source(
                            resolve_attributes(&user, &channel).unwrap_or(user),
                        );
                        Some(Target::Channel { channel, source })
                    }
                    (target::Target::Query(query), Some(user)) => {
                        let query = if user.nickname() == *our_nick {
                            // Message from ourself, from another client.
                            query
                        } else {
                            // Message from conversation partner.
                            target::Query::parse(
                                user.as_str(),
                                chantypes,
                                statusmsg,
                                casemapping,
                            )
                            .ok()?
                        };

                        Some(Target::Query {
                            query,
                            source: source(user),
                        })
                    }
                    _ => None,
                }
            }
        }
        Command::CHGHOST(_, _) => Some(Target::Server {
            source: Source::Server(Some(source::Server::new(
                Kind::ChangeHost,
                user.map(|user| user.nickname().to_owned()),
            ))),
        }),
        Command::Numeric(RPL_MONONLINE, _) => Some(Target::Server {
            source: Source::Server(Some(source::Server::new(
                Kind::MonitoredOnline,
                None,
            ))),
        }),
        Command::Numeric(RPL_MONOFFLINE, _) => Some(Target::Server {
            source: Source::Server(Some(source::Server::new(
                Kind::MonitoredOffline,
                None,
            ))),
        }),
        Command::FAIL(_, _, _, _) => Some(Target::Server {
            source: Source::Server(Some(source::Server::new(
                Kind::StandardReply(StandardReply::Fail),
                None,
            ))),
        }),
        Command::WARN(_, _, _, _) => Some(Target::Server {
            source: Source::Server(Some(source::Server::new(
                Kind::StandardReply(StandardReply::Warn),
                None,
            ))),
        }),
        Command::NOTE(_, _, _, _) => Some(Target::Server {
            source: Source::Server(Some(source::Server::new(
                Kind::StandardReply(StandardReply::Note),
                None,
            ))),
        }),
        Command::WALLOPS(_) => Some(Target::Server {
            source: Source::Server(Some(source::Server::new(
                Kind::WAllOps,
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
        | Command::USERHOST(_)
        | Command::CAP(_, _, _, _)
        | Command::AUTHENTICATE(_)
        | Command::ACCOUNT(_)
        | Command::BATCH(_, _)
        | Command::CHATHISTORY(_, _)
        | Command::CNOTICE(_, _, _)
        | Command::CPRIVMSG(_, _, _)
        | Command::KNOCK(_, _)
        | Command::MARKREAD(_, _)
        | Command::MONITOR(_, _)
        | Command::SETNAME(_)
        | Command::TAGMSG(_)
        | Command::USERIP(_)
        | Command::HELP(_)
        | Command::Numeric(_, _)
        | Command::Unknown(_, _)
        | Command::BOUNCER(_, _)
        | Command::Raw(_) => Some(Target::Server {
            source: Source::Server(None),
        }),
    }
}

pub fn message_id(message: &Encoded) -> Option<String> {
    message.tags.get("msgid").cloned()
}

pub fn server_time(message: &Encoded) -> DateTime<Utc> {
    message
        .tags
        .get("time")
        .and_then(|rfc3339| DateTime::parse_from_rfc3339(rfc3339).ok())
        .map_or_else(Utc::now, |dt| dt.with_timezone(&Utc))
}

fn content<'a>(
    message: &Encoded,
    our_nick: &Nick,
    config: &Config,
    resolve_attributes: &dyn Fn(&User, &target::Channel) -> Option<User>,
    channel_users: impl Fn(&target::Channel) -> Option<&'a ChannelUsers>,
    chantypes: &[char],
    statusmsg: &[char],
    casemapping: isupport::CaseMap,
    prefix: &[isupport::PrefixMap],
) -> Option<Content> {
    use irc::proto::command::Numeric::*;

    match &message.command {
        Command::TOPIC(target, topic) => {
            let raw_user = message.user(casemapping)?;
            let user = target::Channel::parse(
                target,
                chantypes,
                statusmsg,
                casemapping,
            )
            .ok()
            .and_then(|channel| resolve_attributes(&raw_user, &channel))
            .unwrap_or(raw_user);

            let topic = topic.as_ref()?;

            if topic.is_empty() {
                Some(parse_fragments_with_user(
                    format!("{} cleared the topic", user.nickname()),
                    &user,
                    casemapping,
                ))
            } else {
                Some(parse_fragments_with_user(
                    format!("{} changed the topic to {topic}", user.nickname()),
                    &user,
                    casemapping,
                ))
            }
        }
        Command::PART(target, text) => {
            let raw_user = message.user(casemapping)?;
            let user = target::Channel::parse(
                target,
                chantypes,
                statusmsg,
                casemapping,
            )
            .ok()
            .and_then(|channel| resolve_attributes(&raw_user, &channel))
            .unwrap_or(raw_user);

            let text = text
                .as_ref()
                .map(|text| format!(" ({text})"))
                .unwrap_or_default();

            Some(parse_fragments_with_user(
                format!(
                    "⟵ {} has left the channel{text}",
                    user.formatted(
                        config.buffer.server_messages.part.username_format
                    )
                ),
                &user,
                casemapping,
            ))
        }
        Command::JOIN(target, _) => {
            let raw_user = message.user(casemapping)?;
            let user = target::Channel::parse(
                target,
                chantypes,
                statusmsg,
                casemapping,
            )
            .ok()
            .and_then(|channel| resolve_attributes(&raw_user, &channel))
            .unwrap_or(raw_user);

            (user.nickname() != *our_nick).then(|| {
                parse_fragments_with_user(
                    format!(
                        "⟶ {} has joined the channel",
                        user.formatted(
                            config.buffer.server_messages.join.username_format
                        )
                    ),
                    &user,
                    casemapping,
                )
            })
        }
        Command::KICK(channel, victim, reason) => {
            let raw_victim_user =
                User::from(Nick::from_str(victim.as_str(), casemapping));
            let victim = target::Channel::parse(
                victim,
                chantypes,
                statusmsg,
                casemapping,
            )
            .ok()
            .and_then(|channel| resolve_attributes(&raw_victim_user, &channel))
            .unwrap_or(raw_victim_user);

            let ourself = victim.nickname() == *our_nick;

            let raw_user = message.user(casemapping)?;
            let user = target::Channel::parse(
                channel,
                chantypes,
                statusmsg,
                casemapping,
            )
            .ok()
            .and_then(|channel| resolve_attributes(&raw_user, &channel))
            .unwrap_or(raw_user);

            Some(kick_text(user, victim, ourself, reason, None, casemapping))
        }
        Command::MODE(target, modes, args) => {
            let raw_user = message.user(casemapping)?;

            let modes = modes
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ");

            let mut args = args
                .iter()
                .flatten()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ");

            if !args.is_empty() {
                args.insert(0, ' ');
            }

            if let Ok(channel) = target::Channel::parse(
                target,
                chantypes,
                statusmsg,
                casemapping,
            ) {
                let user =
                    resolve_attributes(&raw_user, &channel).unwrap_or(raw_user);

                let channel_users = target::Channel::parse(
                    target,
                    chantypes,
                    statusmsg,
                    casemapping,
                )
                .ok()
                .and_then(|channel| channel_users(&channel));

                Some(parse_fragments_with_users(
                    format!("{} sets mode {modes}{args}", user.nickname()),
                    channel_users,
                    casemapping,
                ))
            } else if raw_user.nickname() == *our_nick {
                if casemapping.normalize(target) == our_nick.as_normalized_str()
                {
                    Some(parse_fragments(format!(
                        "User mode set {modes}{args}"
                    )))
                } else {
                    Some(parse_fragments(format!(
                        "Set {target} mode {modes}{args}"
                    )))
                }
            } else {
                let channel_users =
                    [raw_user.clone(), User::from(our_nick.clone())]
                        .into_iter()
                        .collect::<ChannelUsers>();

                Some(parse_fragments_with_users(
                    format!(
                        "{} sets {target} mode {modes}{args}",
                        raw_user.nickname()
                    ),
                    Some(&channel_users),
                    casemapping,
                ))
            }
        }
        Command::PRIVMSG(target, text) | Command::NOTICE(target, text) => {
            let channel_users = target::Channel::parse(
                target,
                chantypes,
                statusmsg,
                casemapping,
            )
            .ok()
            .and_then(|channel| channel_users(&channel));

            // Check if a synthetic action message

            if let Some(nick) =
                message.user(casemapping).as_ref().map(User::nickname)
                && let Some(action) = parse_action(
                    nick,
                    text,
                    channel_users,
                    target,
                    Some(our_nick),
                    &config.highlights,
                    casemapping,
                )
            {
                return Some(action);
            }

            if let Some(query) = ctcp::parse_query(text) {
                let arrow = if casemapping.normalize(target)
                    == our_nick.as_normalized_str()
                {
                    "⟵"
                } else {
                    "⟶"
                };

                let command = query.command.as_ref();

                let text = if let Some(params) = query.params {
                    [arrow, command, params].join(" ")
                } else {
                    [arrow, command].join(" ")
                };

                return Some(parse_fragments(text));
            }

            Some(parse_fragments_with_highlights(
                text.clone(),
                channel_users,
                target,
                Some(our_nick),
                &config.highlights,
                casemapping,
            ))
        }
        Command::Numeric(RPL_TOPIC, params) => {
            let topic = params.get(2)?;

            Some(parse_fragments(format!("topic is {topic}")))
        }
        Command::Numeric(RPL_ENDOFWHOIS, _) => {
            // We skip the end message of a WHOIS.
            None
        }
        Command::Numeric(RPL_WHOISIDLE, params) => {
            let user = User::from(Nick::from_str(
                params.get(1)?.as_str(),
                casemapping,
            ));

            let idle = params.get(2)?.parse::<u64>().ok()?;
            let sign_on = params.get(3)?.parse::<u64>().ok()?;

            let sign_on = Posix::from_seconds(sign_on);
            let sign_on_datetime = sign_on.datetime()?.to_string();

            let mut formatter = timeago::Formatter::new();
            // Remove "ago" from relative time.
            formatter.ago("");

            let duration = std::time::Duration::from_secs(idle);
            let idle_readable = formatter.convert(duration);

            Some(parse_fragments_with_user(
                format!(
                    "{} signed on at {sign_on_datetime} and has been idle for {idle_readable}",
                    user.nickname()
                ),
                &user,
                casemapping,
            ))
        }
        Command::Numeric(RPL_WHOISSERVER, params) => {
            let user = User::from(Nick::from_str(
                params.get(1)?.as_str(),
                casemapping,
            ));

            let server = params.get(2)?;
            let region = params.get(3)?;

            Some(parse_fragments_with_user(
                format!(
                    "{} is connected on {server} ({region})",
                    user.nickname()
                ),
                &user,
                casemapping,
            ))
        }
        Command::Numeric(RPL_WHOISUSER, params) => {
            let user = User::from(Nick::from_str(
                params.get(1)?.as_str(),
                casemapping,
            ));

            let userhost = format!("{}@{}", params.get(2)?, params.get(3)?);
            let real_name = params.get(5)?;

            Some(parse_fragments_with_user(
                format!(
                    "{} has userhost {userhost} and real name '{real_name}'",
                    user.nickname()
                ),
                &user,
                casemapping,
            ))
        }
        Command::Numeric(RPL_WHOISCHANNELS, params) => {
            let user = User::from(Nick::from_str(
                params.get(1)?.as_str(),
                casemapping,
            ));
            let channels = params.get(2)?;

            Some(parse_fragments_with_user(
                format!("{} is in {channels}", user.nickname()),
                &user,
                casemapping,
            ))
        }
        Command::Numeric(RPL_WHOISACTUALLY, params) => {
            let user: User = User::from(Nick::from_str(
                params.get(1)?.as_str(),
                casemapping,
            ));
            let ip = params.get(2)?;
            let status_text = params.get(3)?;

            Some(parse_fragments_with_user(
                format!("{} {status_text} {ip}", user.nickname()),
                &user,
                casemapping,
            ))
        }
        Command::Numeric(
            RPL_WHOISCERTFP | RPL_WHOISHOST | RPL_WHOISSECURE,
            params,
        ) => {
            let user: User = User::from(Nick::from_str(
                params.get(1)?.as_str(),
                casemapping,
            ));
            let status_text = params.get(2)?;

            Some(parse_fragments_with_user(
                format!("{} {status_text}", user.nickname()),
                &user,
                casemapping,
            ))
        }
        Command::Numeric(RPL_WHOISACCOUNT, params) => {
            let user: User = User::from(Nick::from_str(
                params.get(1)?.as_str(),
                casemapping,
            ));
            let account = params.get(2)?;
            let status_text = params.get(3)?;

            Some(parse_fragments_with_user(
                format!("{} {status_text} {account}", user.nickname()),
                &user,
                casemapping,
            ))
        }
        Command::Numeric(RPL_TOPICWHOTIME, params) => {
            let user = User::parse(
                params.get(2)?.as_str(),
                Some(casemapping),
                Some(prefix),
            )
            .ok()?;

            let datetime = params
                .get(3)?
                .parse::<u64>()
                .ok()
                .map(Posix::from_seconds)
                .as_ref()
                .and_then(Posix::datetime)?
                .with_timezone(&Local)
                .to_rfc2822();

            Some(parse_fragments_with_user(
                format!("topic set by {} at {datetime}", user.nickname()),
                &user,
                casemapping,
            ))
        }
        Command::Numeric(RPL_CHANNELMODEIS, params) => {
            let mode = params
                .iter()
                .skip(2)
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(" ");

            Some(parse_fragments(format!("Channel mode is {mode}")))
        }
        Command::Numeric(RPL_UMODEIS, params) => {
            let mode = params
                .iter()
                .skip(1)
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(" ");

            Some(parse_fragments(format!("User mode is {mode}")))
        }
        Command::Numeric(RPL_AWAY, params) => {
            let user = User::from(Nick::from_str(
                params.get(1)?.as_str(),
                casemapping,
            ));
            let away_message = params
                .get(2)
                .map(|away| format!(" ({away})"))
                .unwrap_or_default();

            Some(parse_fragments_with_user(
                format!("{} is away{away_message}", user.nickname()),
                &user,
                casemapping,
            ))
        }
        Command::Numeric(numeric, params)
            if matches!(numeric, RPL_MONONLINE | RPL_MONOFFLINE) =>
        {
            let target_users = params
                .get(1)?
                .split(',')
                .map(|target| {
                    User::parse(target, Some(casemapping), Some(prefix))
                        .unwrap_or(User::from(Nick::from_str(
                            target,
                            casemapping,
                        )))
                })
                .collect::<ChannelUsers>();

            let target_usernames = target_users
                .iter()
                .map(|user| user.formatted(UsernameFormat::Full))
                .collect::<Vec<_>>();

            let targets = monitored_targets_text(target_usernames)?;

            Some(parse_fragments_with_users(
                match numeric {
                    RPL_MONONLINE => format!("Monitored {targets} online"),
                    RPL_MONOFFLINE => format!("Monitored {targets} offline"),
                    _ => {
                        log::debug!("Unexpected numeric {numeric:?}");
                        format!("Monitored {targets}")
                    }
                },
                Some(&target_users),
                casemapping,
            ))
        }
        Command::CHATHISTORY(sub, args) => {
            if sub == "TARGETS" {
                let target = args.first()?;
                let timestamp = args.get(1)?;

                Some(plain(format!("Chat history for {target} at {timestamp}")))
            } else {
                None
            }
        }
        Command::FAIL(command, _, context, description) => {
            if let Some(context) = context {
                Some(plain(format!(
                    "{command} ({}) failed: {description}",
                    context.join(", ")
                )))
            } else {
                Some(plain(format!("{command} failed: {description}")))
            }
        }
        Command::WARN(command, _, context, description) => {
            if let Some(context) = context {
                Some(plain(format!(
                    "{command} ({}) warning: {description}",
                    context.join(", ")
                )))
            } else {
                Some(plain(format!("{command} warning: {description}")))
            }
        }
        Command::NOTE(command, _, context, description) => {
            if let Some(context) = context {
                Some(plain(format!(
                    "{command} ({}) notice: {description}",
                    context.join(", ")
                )))
            } else {
                Some(plain(format!("{command} notice: {description}")))
            }
        }
        Command::WALLOPS(text) => {
            let user = message.user(casemapping)?;

            Some(parse_fragments_with_user(
                format!("WALLOPS from {}: {}", user.nickname(), text.clone()),
                &user,
                casemapping,
            ))
        }
        Command::Numeric(RPL_WELCOME, params) => {
            Some(parse_fragments_with_user(
                params
                    .iter()
                    .map(String::as_str)
                    .skip(1)
                    .collect::<Vec<_>>()
                    .join(" "),
                &User::from(our_nick.clone()),
                casemapping,
            ))
        }
        Command::Numeric(_, responses) | Command::Unknown(_, responses) => {
            Some(parse_fragments(
                responses
                    .iter()
                    .map(String::as_str)
                    .skip(1)
                    .collect::<Vec<_>>()
                    .join(" "),
            ))
        }
        _ => None,
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Limit {
    Top(usize),
    Bottom(usize),
    Since(DateTime<Utc>),
}

pub fn is_action(text: &str) -> bool {
    if let Some(query) = ctcp::parse_query(text) {
        matches!(query.command, ctcp::Command::Action)
    } else {
        false
    }
}

fn parse_action(
    nick: NickRef,
    text: &str,
    channel_users: Option<&ChannelUsers>,
    target: &str,
    our_nick: Option<&Nick>,
    highlights: &Highlights,
    casemapping: isupport::CaseMap,
) -> Option<Content> {
    if !is_action(text) {
        return None;
    }

    let query = ctcp::parse_query(text)?;

    Some(action_text(
        nick,
        query.params,
        channel_users,
        target,
        our_nick,
        highlights,
        casemapping,
    ))
}

pub fn action_text(
    nick: NickRef,
    action: Option<&str>,
    channel_users: Option<&ChannelUsers>,
    target: &str,
    our_nick: Option<&Nick>,
    highlights: &Highlights,
    casemapping: isupport::CaseMap,
) -> Content {
    let text = if let Some(action) = action {
        format!("{nick} {action}")
    } else {
        format!("{nick}")
    };

    parse_fragments_with_highlights(
        text,
        channel_users,
        target,
        our_nick,
        highlights,
        casemapping,
    )
}

fn kick_text(
    kicker: User,
    victim: User,
    ourself: bool,
    reason: &Option<String>,
    channel: Option<target::Channel>,
    casemapping: isupport::CaseMap,
) -> Content {
    let target = if ourself {
        if channel.is_some() {
            "You have".to_string()
        } else {
            "you have".to_string()
        }
    } else {
        format!("{} has", victim.nickname())
    };

    let reason = reason
        .as_ref()
        .map(|reason| format!(" ({reason})"))
        .unwrap_or_default();

    parse_fragments_with_users(
        if let Some(channel) = channel {
            format!(
                "{target} been kicked from {channel} by {}{reason}",
                kicker.nickname()
            )
        } else {
            format!("⟵ {target} been kicked by {}{reason}", kicker.nickname())
        },
        Some(&[kicker, victim].into_iter().collect()),
        casemapping,
    )
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

#[derive(Debug, Clone)]
pub enum Link {
    Channel(target::Channel),
    Url(String),
    User(User),
    GoToMessage(Server, target::Channel, Hash),
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MessageReferences {
    pub timestamp: DateTime<Utc>,
    pub id: Option<String>,
}

impl MessageReferences {
    pub fn message_reference(
        &self,
        message_reference_types: &[isupport::MessageReferenceType],
    ) -> isupport::MessageReference {
        for message_reference_type in message_reference_types {
            match message_reference_type {
                isupport::MessageReferenceType::MessageId => {
                    if let Some(id) = &self.id {
                        return isupport::MessageReference::MessageId(
                            id.clone(),
                        );
                    }
                }
                isupport::MessageReferenceType::Timestamp => {
                    return isupport::MessageReference::Timestamp(
                        self.timestamp,
                    );
                }
            }
        }

        isupport::MessageReference::None
    }
}

impl PartialEq for MessageReferences {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp.eq(&other.timestamp)
    }
}

impl Eq for MessageReferences {}

impl Ord for MessageReferences {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.timestamp.cmp(&other.timestamp)
    }
}

impl PartialOrd for MessageReferences {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(any(test, feature = "message_tests"))]
pub mod tests {
    #[allow(unused_imports)]
    use super::{
        parse_fragments, parse_fragments_with_highlights,
        parse_fragments_with_users,
    };
    #[allow(unused_imports)]
    use crate::config::Highlights;
    #[allow(unused_imports)]
    use crate::config::highlights::Nickname;
    #[allow(unused_imports)]
    use crate::message::formatting::Color;
    #[allow(unused_imports)]
    use crate::message::{
        Broadcast, Content, Formatting, Fragment, Message, broadcast,
    };
    #[allow(unused_imports)]
    use crate::user::{ChannelUsers, Nick, User};
    #[allow(unused_imports)]
    use crate::{isupport, target};

    #[test]
    fn fragments_parsing() {
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
                "We have a wiki at https://halloy.chat",
                vec![
                    Fragment::Text("We have a wiki at ".into()),
                    Fragment::Url("https://halloy.chat".parse().unwrap()),
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
                "https://www.reddit.com/r/witze/comments/1fcoz5a/ein_vampir_auf_einem_tandem_gerät_in_eine/", // spellchecker:disable-line
                vec![Fragment::Url(
                    "https://www.reddit.com/r/witze/comments/1fcoz5a/ein_vampir_auf_einem_tandem_gerät_in_eine/" // spellchecker:disable-line
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
            (
                "\u{f}\u{3}03VLC\u{f} \u{3}05master\u{f} \u{3}06somenick\u{f} \u{3}14http://some.website.com/\u{f} * describe commit * \u{3}14https://code.videolan.org/videolan/vlc/\u{f}",
                vec![
                    Fragment::Formatted{ text: "VLC".into(), formatting: Formatting { fg: Some(Color::Green), ..Formatting::default() }},
                    Fragment::Text(" ".into()),
                    Fragment::Formatted{ text: "master".into(), formatting: Formatting { fg: Some(Color::Brown), ..Formatting::default() }},
                    Fragment::Text(" ".into()),
                    Fragment::Formatted{ text: "somenick".into(), formatting: Formatting { fg: Some(Color::Magenta), ..Formatting::default() }},
                    Fragment::Text(" ".into()),
                    Fragment::Url("http://some.website.com/".parse().unwrap()),
                    Fragment::Text(" * describe commit * ".into()),
                    Fragment::Url("https://code.videolan.org/videolan/vlc/".parse().unwrap()),
                ],
            ),
            (
                "\u{f}\u{11}formatting that wraps a https://www.website.com/ like so\u{f}",
                vec![
                    Fragment::Formatted{ text: "formatting that wraps a ".into(), formatting: Formatting { monospace: true, ..Formatting::default() }},
                    Fragment::Url("https://www.website.com/".parse().unwrap()),
                    Fragment::Formatted{ text: " like so".into(), formatting: Formatting { monospace: true, ..Formatting::default() }},
                ],
            ),
            (
                "\u{f}\u{3}09color that wraps a https://www.website.com/ like so\u{f}",
                vec![
                    Fragment::Formatted{ text: "color that wraps a ".into(), formatting: Formatting { fg: Some(Color::LightGreen), ..Formatting::default() }},
                    Fragment::Url("https://www.website.com/".parse().unwrap()),
                    Fragment::Formatted{ text: " like so".into(), formatting: Formatting { fg: Some(Color::LightGreen), ..Formatting::default() }},
                ],
            ),
            (
                "\u{f}\u{3}11color that wraps a #channel like so\u{f}",
                vec![
                    Fragment::Formatted{ text: "color that wraps a ".into(), formatting: Formatting { fg: Some(Color::LightCyan), ..Formatting::default() }},
                    Fragment::Channel("#channel".into()),
                    Fragment::Formatted{ text: " like so".into(), formatting: Formatting { fg: Some(Color::LightCyan), ..Formatting::default() }},
                ],
            ),
            (
                "\u{3}22,33testing normal color with background and color \u{3}reset",
                vec![
                    Fragment::Formatted { text: "testing normal color with background and color ".into(), formatting: Formatting { fg: Some(Color::Code22), bg: Some(Color::Code33), ..Formatting::default() }},
                    Fragment::Text("reset".into()),
                ],
            ),
            (
                "\u{4}ffffff,0909aatesting hex color with background and color \u{4}reset",
                vec![
                    Fragment::Formatted { text: "testing hex color with background and color ".into(), formatting: Formatting { fg: Some(Color::Rgb(255, 255, 255)), bg: Some(Color::Rgb(9, 9, 170)), ..Formatting::default() }},
                    Fragment::Text("reset".into()),
                ],
            ),
        ];

        for (text, expected) in tests {
            let actual = parse_fragments(text.to_string());

            assert_eq!(Content::Fragments(expected), actual);
        }
    }

    #[test]
    fn fragments_with_users_parsing() {
        let casemapping = isupport::CaseMap::default();
        let tests = [(
            (
                "Hey Dave!~Dave@user/Dave have you seen &`Bill`?".to_string(),
                ["Greg", "Dave", "Bob", "George_", "`Bill`"]
                    .into_iter()
                    .map(|nick| User::from(Nick::from_str(nick, casemapping)))
                    .collect::<ChannelUsers>(),
            ),
            vec![
                Fragment::Text("Hey ".into()),
                Fragment::User(
                    User::from(Nick::from_str("Dave", casemapping)),
                    "Dave".into(),
                ),
                Fragment::Text("!~".into()),
                Fragment::User(
                    User::from(Nick::from_str("Dave", casemapping)),
                    "Dave".into(),
                ),
                Fragment::Text("@user/Dave have you seen &".into()),
                Fragment::User(
                    User::from(Nick::from_str("`Bill`", casemapping)),
                    "`Bill`".into(),
                ),
                Fragment::Text("?".into()),
            ],
        )];
        for ((text, channel_users), expected) in tests {
            if let Content::Fragments(actual) = parse_fragments_with_users(
                text,
                Some(&channel_users),
                casemapping,
            ) {
                assert_eq!(expected, actual);
            } else {
                panic!("expected fragments with users");
            }
        }
    }

    #[test]
    fn fragments_with_highlights_parsing() {
        let casemapping = isupport::CaseMap::default();

        let tests = [
            (
                (
                    "Bob: I'm in #interesting with Greg, George_, &`bill`. I hope @Dave doesn't notice.".to_string(),
                    [
                        "Greg",
                        "Dave",
                        "Bob",
                        "George_",
                        "`Bill`",
                    ].into_iter().map(|nick| User::from(Nick::from_str(nick, casemapping))).collect::<ChannelUsers>(),
                    "#interesting",
                    Some(Nick::from_str("Bob", casemapping)),
                    &Highlights {
                        nickname: Nickname {exclude: vec![], include: vec!["#interesting".into()], case_insensitive: true},
                        matches: vec![],
                    },
                ),
                vec![
                    Fragment::HighlightNick(User::from(Nick::from_str("Bob", casemapping)), "Bob".into()),
                    Fragment::Text(": I'm in ".into()),
                    Fragment::Channel("#interesting".into()),
                    Fragment::Text(" with ".into()),
                    Fragment::User(User::from(Nick::from_str("Greg", casemapping)), "Greg".into()),
                    Fragment::Text(", ".into()),
                    Fragment::User(User::from(Nick::from_str("George_", casemapping)), "George_".into()),
                    Fragment::Text(", &".into()),
                    Fragment::User(User::from(Nick::from_str("`Bill`", casemapping)), "`bill`".into()),
                    Fragment::Text(". I hope @".into()),
                    Fragment::User(User::from(Nick::from_str("Dave", casemapping)), "Dave".into()),
                    Fragment::Text(" doesn't notice.".into()),
                ],
            ),
            (
                (
                    "the boat would bob up and down!".to_string(),
                    [
                        "Greg",
                        "Dave",
                        "Bob",
                        "George_",
                        "`Bill`",
                    ].into_iter().map(|nick| User::from(Nick::from_str(nick, casemapping))).collect::<ChannelUsers>(),
                    "#interesting",
                    Some(Nick::from_str("Bob", casemapping)),
                    &Highlights {
                        nickname: Nickname {exclude: vec![], include: vec![], case_insensitive: false},
                        matches: vec![],
                    },
                ),
                vec![
                    Fragment::Text("the boat would ".into()),
                    Fragment::User(User::from(Nick::from_str("Bob", casemapping)), "bob".into()),
                    Fragment::Text(" up and down!".into()),
                ],
            ),
            (
                (
                    "\u{3}14<\u{3}\u{3}04lurk_\u{3}\u{3}14/rx>\u{3} f_~oftc: > A��\u{1f}qj\u{14}��L�5�g���5�P��yn_?�i3g�1\u{7f}mE�\\X��� Xe�\u{5fa}{d�+�`@�^��NK��~~ޏ\u{7}\u{8}\u{15}\\�\u{4}A� \u{f}\u{1c}�N\u{11}6�r�\u{4}t��Q��\u{1c}�m\u{19}��".to_string(),
                    [
                        "f_",
                        "rx",
                    ].into_iter().map(|nick| User::from(Nick::from_str(nick, casemapping))).collect::<ChannelUsers>(),
                    "#funderscore-sucks",
                    Some(Nick::from_str("f_", casemapping)),
                    &Highlights {
                        nickname: Nickname {exclude: vec![], include: vec!["*".into()], case_insensitive: true},
                        matches: vec![],
                    },
                ),
                vec![
                    Fragment::Text("\u{3}14<\u{3}\u{3}04lurk_\u{3}\u{3}14/rx>\u{3} ".into()),
                    Fragment::HighlightNick(User::from(Nick::from_str("f_", casemapping)), "f_".into()),
                    Fragment::Text("~oftc: > A��\u{1f}qj\u{14}��L�5�g���5�P��yn_?�i3g�1\u{7f}mE�\\X��� Xe�\u{5fa}{d�+�`@�^��NK��~~ޏ\u{7}\u{8}\u{15}\\�\u{4}A� \u{f}\u{1c}�N\u{11}6�r�\u{4}t��Q��\u{1c}�m\u{19}��".into())
                ],
            ),
        ];
        for ((text, channel_users, target, our_nick, highlights), expected) in
            tests
        {
            if let Content::Fragments(actual) = parse_fragments_with_highlights(
                text,
                Some(&channel_users),
                target,
                our_nick.as_ref(),
                highlights,
                casemapping,
            ) {
                assert_eq!(expected, actual);
            } else {
                panic!("expected fragments with highlighting");
            }
        }
    }

    pub const SERDE_IRC_MESSAGES: &[&str] = &[
        "@time=2023-07-20T21:19:11.000Z :chat!test@user/test/bot/chat PRIVMSG ##chat :\\_o< quack!\r\n",
        "@id=234AB :dan!d@localhost PRIVMSG #chan :Hey what's up! \r\n",
        "@time=2025-02-11T20:28:47.354Z :our_nick PRIVMSG #test-chan :\u{1}ACTION wants to generate an action message for testing XD\u{1}\r\n",
        "@id=DTSA :our_nick PRIVMSG #halloy :check out https://unstable.halloy.squidowl.org/, when you're building from source\r\n",
        ":dan!d@localhost PRIVMSG #chan-chan \u{1f}how\u{1f} \u{11}about\u{11} \u{2}\u{1d}some\u{1d}\u{2} \u{2}markdown\u{2} \u{1d}for\u{1d} \u{1e}testing\u{1e} \u{3}4too\u{3}? \r\n",
        ":WiZ JOIN #Twilight_zone\r\n",
        ":`whammer`!warhammer@40k PART #test\r\n",
        ":soju.bouncer FAIL * ACCOUNT_REQUIRED :Authentication required\r\n",
        ":rabbit MODE #토끼세계 +o bunny\r\n",
    ];

    pub fn message_from_irc_message(irc_message: &str) -> Message {
        use std::collections::HashMap;

        use irc::proto;

        use crate::config::Config;
        use crate::message::Encoded;
        use crate::{isupport, target};

        let isupport = HashMap::<isupport::Kind, isupport::Parameter>::new();

        let our_nick = Nick::from_str(
            "our_nick",
            isupport::get_casemapping_or_default(&isupport),
        );

        let channel_users: ChannelUsers = [
            User::from(Nick::from_str(
                "chat",
                isupport::get_casemapping_or_default(&isupport),
            )),
            User::from(Nick::from_str(
                "dan",
                isupport::get_casemapping_or_default(&isupport),
            )),
            User::from(our_nick.clone()),
        ]
        .into_iter()
        .collect();

        let encoded = proto::parse::message(irc_message).unwrap();

        Message::received(
            Encoded::from(encoded.clone()),
            our_nick.clone(),
            &Config::default(),
            |user: &User, _channel: &target::Channel| {
                channel_users
                    .iter()
                    .find(|channel_user| *channel_user == user)
                    .cloned()
            },
            |_channel: &target::Channel| Some(&channel_users),
            isupport::get_chantypes_or_default(&isupport),
            isupport::get_statusmsg_or_default(&isupport),
            isupport::get_casemapping_or_default(&isupport),
            isupport::get_prefix_or_default(&isupport),
        )
        .unwrap_or_else(|| panic!("failed to create Message from {encoded:?}"))
    }

    pub fn serde_broadcasts() -> Vec<Broadcast> {
        use std::collections::HashMap;

        use crate::{isupport, target};

        let isupport = HashMap::<isupport::Kind, isupport::Parameter>::new();

        let user_channels = vec![
            target::Channel::from_str(
                "##chat",
                isupport::get_chantypes_or_default(&isupport),
                isupport::get_casemapping_or_default(&isupport),
            ),
            target::Channel::from_str(
                "#halloy",
                isupport::get_chantypes_or_default(&isupport),
                isupport::get_casemapping_or_default(&isupport),
            ),
        ];

        vec![
            Broadcast::Connecting,
            Broadcast::Connected,
            Broadcast::ConnectionFailed {
                error: "a TLS error occurred: \
                            io error: received fatal alert: HandshakeFailure"
                    .to_string(),
            },
            Broadcast::Disconnected { error: None },
            Broadcast::Reconnected,
            Broadcast::Quit {
                user: User::parse(
                    "+nieve!snow@yeti",
                    isupport::get_casemapping(&isupport),
                    isupport::get_prefix(&isupport),
                )
                .unwrap(),
                comment: Some("see you later our_nick".to_string()),
                user_channels: user_channels.clone(),
                casemapping: isupport::get_casemapping_or_default(&isupport),
            },
            Broadcast::Nickname {
                old_nick: Nick::from_str(
                    "dan",
                    isupport::get_casemapping_or_default(&isupport),
                ),
                new_nick: Nick::from_str(
                    "dandadan",
                    isupport::get_casemapping_or_default(&isupport),
                ),
                ourself: false,
                user_channels: user_channels.clone(),
                casemapping: isupport::get_casemapping_or_default(&isupport),
            },
            Broadcast::Nickname {
                old_nick: Nick::from_str(
                    "our_old_nick",
                    isupport::get_casemapping_or_default(&isupport),
                ),
                new_nick: Nick::from_str(
                    "our_new_nick",
                    isupport::get_casemapping_or_default(&isupport),
                ),
                ourself: true,
                user_channels: user_channels.clone(),
                casemapping: isupport::get_casemapping_or_default(&isupport),
            },
            Broadcast::Invite {
                inviter: Nick::from_str(
                    "`whammer`",
                    isupport::get_casemapping_or_default(&isupport),
                ),
                channel: target::Channel::from_str(
                    "#40k",
                    isupport::get_chantypes_or_default(&isupport),
                    isupport::get_casemapping_or_default(&isupport),
                ),
                user_channels: user_channels.clone(),
                casemapping: isupport::get_casemapping_or_default(&isupport),
            },
            Broadcast::ChangeHost {
                old_user: User::parse(
                    "our_nick!old_user@old_host",
                    isupport::get_casemapping(&isupport),
                    isupport::get_prefix(&isupport),
                )
                .unwrap(),
                new_username: "new_user".to_string(),
                new_hostname: "new_host".to_string(),
                ourself: true,
                logged_in: false,
                user_channels: user_channels.clone(),
                casemapping: isupport::get_casemapping_or_default(&isupport),
            },
            Broadcast::ChangeHost {
                old_user: User::parse(
                    "+nieve!snow@yeti",
                    isupport::get_casemapping(&isupport),
                    isupport::get_prefix(&isupport),
                )
                .unwrap(),
                new_username: "lava".to_string(),
                new_hostname: "troll".to_string(),
                ourself: false,
                logged_in: true,
                user_channels: user_channels.clone(),
                casemapping: isupport::get_casemapping_or_default(&isupport),
            },
        ]
    }

    pub fn messages_from_broadcast(broadcast: Broadcast) -> Vec<Message> {
        use chrono::Utc;

        use crate::config::Config;

        let channels = vec![
            target::Channel::from_str(
                "##chat",
                isupport::DEFAULT_CHANTYPES,
                isupport::CaseMap::default(),
            ),
            target::Channel::from_str(
                "#halloy",
                isupport::DEFAULT_CHANTYPES,
                isupport::CaseMap::default(),
            ),
            target::Channel::from_str(
                "#libera",
                isupport::DEFAULT_CHANTYPES,
                isupport::CaseMap::default(),
            ),
            target::Channel::from_str(
                "&test-chan",
                isupport::DEFAULT_CHANTYPES,
                isupport::CaseMap::default(),
            ),
        ]
        .into_iter();

        let queries = vec![
            target::Query::from(&User::from(Nick::from_str(
                "dan",
                isupport::CaseMap::default(),
            ))),
            target::Query::from(&User::from(Nick::from_str(
                "WiZ",
                isupport::CaseMap::default(),
            ))),
        ]
        .into_iter();

        broadcast::into_messages(
            broadcast,
            &Config::default(),
            Utc::now(),
            channels,
            queries,
        )
    }

    // Test consistency between current Message serialization & deserialization
    #[test]
    fn messages_serde() {
        let mut messages = SERDE_IRC_MESSAGES
            .iter()
            .map(|irc_message| message_from_irc_message(irc_message))
            .collect::<Vec<Message>>();

        for broadcast in serde_broadcasts() {
            messages.append(&mut messages_from_broadcast(broadcast));
        }

        for expected in messages {
            let bytes = serde_json::to_vec(&expected).unwrap();

            let actual: Message = serde_json::from_slice(&bytes).unwrap();

            assert_eq!(expected, actual);
        }
    }

    // Test Message deserialization from samples of messages serialized by
    // earlier versions (i.e. backward compatibility)
    #[test]
    fn messages_json_deserialize() {
        use std::fs;

        for file in fs::read_dir("tests/message/").unwrap() {
            let path = file.unwrap().path();
            let file_string = fs::read_to_string(&path).unwrap();
            let _messages: Vec<Message> =
                serde_json::from_str(&file_string).unwrap();
        }
    }
}
