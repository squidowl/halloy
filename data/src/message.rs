use std::borrow::Cow;

use chrono::{DateTime, Utc};
use irc::proto;
use irc::proto::Command;
use itertools::Itertools;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize};
use url::Url;

pub use self::source::Source;
use crate::time::{self, Posix};
use crate::user::{Nick, NickRef};
use crate::{ctcp, Config, User};

static URL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?i)((https?|ircs?):\/\/|www\.)[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b([-a-zA-Z0-9()@:%_\+.~#?&//=]*)"#,
    )
    .unwrap()
});

pub type Channel = String;

pub(crate) mod broadcast;
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
    Server { source: Source },
    Channel { channel: Channel, source: Source },
    Query { nick: Nick, source: Source },
}

impl Target {
    pub fn source(&self) -> &Source {
        match self {
            Target::Server { source } => source,
            Target::Channel { source, .. } => source,
            Target::Query { source, .. } => source,
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
}

impl Message {
    pub fn triggers_unread(&self) -> bool {
        matches!(self.direction, Direction::Received)
            && matches!(self.target.source(), Source::User(_) | Source::Action)
    }

    pub fn received(
        encoded: Encoded,
        our_nick: Nick,
        config: &Config,
        resolve_attributes: impl Fn(&User, &str) -> Option<User>,
    ) -> Option<Message> {
        let server_time = server_time(&encoded);
        let content = content(&encoded, &our_nick, config, &resolve_attributes)?;
        let target = target(encoded, &our_nick, &resolve_attributes)?;

        Some(Message {
            received_at: Posix::now(),
            server_time,
            direction: Direction::Received,
            target,
            content,
        })
    }

    pub fn file_transfer_request_received(from: &Nick, filename: &str) -> Message {
        Message {
            received_at: Posix::now(),
            server_time: Utc::now(),
            direction: Direction::Received,
            target: Target::Query {
                nick: from.clone(),
                source: Source::Action,
            },
            content: plain(format!(" ∙ {from} wants to send you \"{filename}\"")),
        }
    }

    pub fn file_transfer_request_sent(to: &Nick, filename: &str) -> Message {
        Message {
            received_at: Posix::now(),
            server_time: Utc::now(),
            direction: Direction::Sent,
            target: Target::Query {
                nick: to.clone(),
                source: Source::Action,
            },
            content: plain(format!(" ∙ offering to send {to} \"{filename}\"")),
        }
    }

    pub fn with_target(self, target: Target) -> Self {
        Self { target, ..self }
    }

    pub fn plain(&self) -> Option<&str> {
        match &self.content {
            Content::Plain(s) => Some(s),
            Content::Fragments(_) => None,
        }
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
        }

        let Data {
            received_at,
            server_time,
            direction,
            target,
            content,
            text,
        } = Data::deserialize(deserializer)?;

        let content = if let Some(content) = content {
            content
        } else if let Some(text) = text {
            // First time upgrading, convert text into content
            parse_fragments(text)
        } else {
            // Unreachable
            Content::Plain("".to_string())
        };

        Ok(Message {
            received_at,
            server_time,
            direction,
            target,
            content,
        })
    }
}

pub fn plain(text: String) -> Content {
    Content::Plain(text)
}

pub fn parse_fragments(text: String) -> Content {
    let mut i = 0;
    let mut fragments = vec![];

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

    // No matches, avoid allocation
    if i == 0 {
        return plain(text);
    } else if i < text.len() {
        fragments.push(Fragment::Text(text[i..text.len()].to_string()));
    }

    Content::Fragments(fragments)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Content {
    Plain(String),
    Fragments(Vec<Fragment>),
}

impl Content {
    fn text(&self) -> Cow<str> {
        match self {
            Content::Plain(s) => s.into(),
            Content::Fragments(fragments) => fragments.iter().map(Fragment::as_str).join("").into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fragment {
    Text(String),
    Url(Url),
}

impl Fragment {
    pub fn as_str(&self) -> &str {
        match self {
            Fragment::Text(s) => s,
            Fragment::Url(u) => u.as_str(),
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
        }),
        Command::TOPIC(channel, _) | Command::KICK(channel, _, _) => Some(Target::Channel {
            channel,
            source: source::Source::Server(None),
        }),
        Command::PART(channel, _) => Some(Target::Channel {
            channel,
            source: source::Source::Server(Some(source::Server::new(
                source::server::Kind::Part,
                Some(user?.nickname().to_owned()),
            ))),
        }),
        Command::JOIN(channel, _) => Some(Target::Channel {
            channel,
            source: source::Source::Server(Some(source::Server::new(
                source::server::Kind::Join,
                Some(user?.nickname().to_owned()),
            ))),
        }),
        Command::Numeric(RPL_TOPIC | RPL_TOPICWHOTIME, params) => {
            let channel = params.get(1)?.clone();
            Some(Target::Channel {
                channel,
                source: source::Source::Server(Some(source::Server::new(
                    source::server::Kind::ReplyTopic,
                    None,
                ))),
            })
        }
        Command::Numeric(RPL_CHANNELMODEIS, params) => {
            let channel = params.get(1)?.clone();
            Some(Target::Channel {
                channel,
                source: source::Source::Server(None),
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

            match (proto::is_channel(&target), user) {
                (true, Some(user)) => {
                    let source = source(resolve_attributes(&user, &target).unwrap_or(user));
                    Some(Target::Channel {
                        channel: target,
                        source,
                    })
                }
                (false, Some(user)) => {
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

            match (proto::is_channel(&target), user) {
                (true, Some(user)) => {
                    let source = source(resolve_attributes(&user, &target).unwrap_or(user));
                    Some(Target::Channel {
                        channel: target,
                        source,
                    })
                }
                (false, Some(user)) => {
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
        | Command::BATCH(_, _)
        | Command::CNOTICE(_, _, _)
        | Command::CPRIVMSG(_, _, _)
        | Command::KNOCK(_, _)
        | Command::TAGMSG(_)
        | Command::USERIP(_)
        | Command::HELP(_)
        | Command::MODE(_, _, _)
        | Command::Numeric(_, _)
        | Command::Unknown(_, _) => Some(Target::Server {
            source: Source::Server(None),
        }),
    }
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

fn content(
    message: &Encoded,
    our_nick: &Nick,
    config: &Config,
    resolve_attributes: &dyn Fn(&User, &str) -> Option<User>,
) -> Option<Content> {
    use irc::proto::command::Numeric::*;

    match &message.command {
        Command::TOPIC(target, topic) => {
            let raw_user = message.user()?;
            let user = resolve_attributes(&raw_user, target).unwrap_or(raw_user);

            let topic = topic.as_ref()?;

            Some(parse_fragments(format!(
                " ∙ {user} changed topic to {topic}"
            )))
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

            Some(parse_fragments(format!(
                "⟵ {user} has left the channel{text}"
            )))
        }
        Command::JOIN(target, _) => {
            let raw_user = message.user()?;
            let user = resolve_attributes(&raw_user, target).unwrap_or(raw_user);

            (user.nickname() != *our_nick).then(|| {
                parse_fragments(format!(
                    "⟶ {} has joined the channel",
                    user.formatted(config.buffer.server_messages.join.username_format)
                ))
            })
        }
        Command::KICK(channel, victim, comment) => {
            let raw_user = message.user()?;
            let user = resolve_attributes(&raw_user, channel).unwrap_or(raw_user);

            let comment = comment
                .as_ref()
                .map(|comment| format!(" ({comment})"))
                .unwrap_or_default();
            let target = if victim == our_nick.as_ref() {
                "you have".to_string()
            } else {
                format!("{victim} has")
            };

            Some(parse_fragments(format!(
                "⟵ {target} been kicked by {user}{comment}"
            )))
        }
        Command::MODE(target, modes, args) if proto::is_channel(target) => {
            let raw_user = message.user()?;
            let user = resolve_attributes(&raw_user, target).unwrap_or(raw_user);

            let modes = modes
                .iter()
                .map(|mode| mode.to_string())
                .collect::<Vec<_>>()
                .join(" ");

            let args = args
                .iter()
                .map(|arg| arg.to_string())
                .collect::<Vec<_>>()
                .join(" ");

            Some(parse_fragments(format!(
                " ∙ {user} sets mode {modes} {args}"
            )))
        }
        Command::PRIVMSG(_, text) => {
            // Check if a synthetic action message
            if let Some(nick) = message.user().as_ref().map(User::nickname) {
                if let Some(action) = parse_action(nick, text) {
                    return Some(action);
                }
            }

            Some(parse_fragments(text.clone()))
        }
        Command::NOTICE(_, text) => Some(parse_fragments(text.clone())),
        Command::Numeric(RPL_TOPIC, params) => {
            let topic = params.get(2)?;

            Some(parse_fragments(format!(" ∙ topic is {topic}")))
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

            Some(parse_fragments(format!(
                " ∙ {nick} signed on at {sign_on_datetime} and has been idle for {idle_readable}"
            )))
        }
        Command::Numeric(RPL_WHOISSERVER, params) => {
            let nick = params.get(1)?;
            let server = params.get(2)?;
            let region = params.get(3)?;

            Some(parse_fragments(format!(
                " ∙ {nick} is connected on {server} ({region})"
            )))
        }
        Command::Numeric(RPL_WHOISUSER, params) => {
            let nick = params.get(1)?;
            let userhost = format!("{}@{}", params.get(2)?, params.get(3)?);
            let real_name = params.get(5)?;

            Some(parse_fragments(format!(
                " ∙ {nick} has userhost {userhost} and real name '{real_name}'"
            )))
        }
        Command::Numeric(RPL_WHOISCHANNELS, params) => {
            let nick = params.get(1)?;
            let channels = params.get(2)?;

            Some(parse_fragments(format!(" ∙ {nick} is in {channels}")))
        }
        Command::Numeric(RPL_WHOISACTUALLY, params) => {
            let nick = params.get(1)?;
            let ip = params.get(2)?;
            let status_text = params.get(3)?;

            Some(parse_fragments(format!(" ∙ {nick} {status_text} {ip}")))
        }
        Command::Numeric(RPL_WHOISSECURE, params) => {
            let nick = params.get(1)?;
            let status_text = params.get(2)?;

            Some(parse_fragments(format!(" ∙ {nick} {status_text}")))
        }
        Command::Numeric(RPL_WHOISACCOUNT, params) => {
            let nick = params.get(1)?;
            let account = params.get(2)?;
            let status_text = params.get(3)?;

            Some(parse_fragments(format!(
                " ∙ {nick} {status_text} {account}"
            )))
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

            Some(parse_fragments(format!(
                " ∙ topic set by {nick} at {datetime}"
            )))
        }
        Command::Numeric(RPL_CHANNELMODEIS, params) => {
            let mode = params
                .iter()
                .skip(2)
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(" ");

            Some(parse_fragments(format!(" ∙ Channel mode is {mode}")))
        }
        Command::Numeric(RPL_AWAY, params) => {
            let user = params.get(1)?;
            let away_message = params
                .get(2)
                .map(|away| format!(" ({away})"))
                .unwrap_or_default();

            Some(parse_fragments(format!(" ∙ {user} is away{away_message}")))
        }
        Command::Numeric(_, responses) | Command::Unknown(_, responses) => Some(parse_fragments(
            responses
                .iter()
                .map(|s| s.as_str())
                .skip(1)
                .collect::<Vec<_>>()
                .join(" "),
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
    const DEFAULT_COUNT: usize = 500;

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
        parse_fragments(format!(" ∙ {nick} {action}"))
    } else {
        plain(format!(" ∙ {nick}"))
    }
}

pub fn reference_user(sender: NickRef, own_nick: NickRef, message: &Message) -> bool {
    let has_nick = match &message.content {
        Content::Plain(text) => text.contains(own_nick.as_ref()),
        Content::Fragments(fragments) => fragments
            .iter()
            .any(|f| f.as_str().contains(own_nick.as_ref())),
    };

    sender != own_nick && has_nick
}

pub fn reference_user_text(sender: NickRef, own_nick: NickRef, text: &str) -> bool {
    sender != own_nick && text.contains(own_nick.as_ref())
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
        ];

        for (text, expected) in tests {
            let actual = parse_fragments(text.to_string());

            assert_eq!(Content::Fragments(expected), actual);
        }
    }
}
