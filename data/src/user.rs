use std::collections::HashSet;
use std::fmt;
use std::hash::Hash;

use irc::proto;
use serde::{Deserialize, Serialize};

use crate::{buffer, config::buffer::UsernameFormat, mode};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(into = "String")]
#[serde(try_from = "String")]
pub struct User {
    nickname: Nick,
    username: Option<String>,
    hostname: Option<String>,
    access_levels: HashSet<AccessLevel>,
    away: bool,
}

impl PartialEq for User {
    fn eq(&self, other: &Self) -> bool {
        self.nickname.eq(&other.nickname)
    }
}

impl Eq for User {}

impl Hash for User {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.nickname.hash(state);
    }
}

impl Ord for User {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.highest_access_level()
            .cmp(&other.highest_access_level())
            .reverse()
            .then_with(|| self.nickname().cmp(&other.nickname()))
    }
}

impl PartialOrd for User {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl TryFrom<String> for User {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl<'a> TryFrom<&'a str> for User {
    type Error = &'static str;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err("nickname can't be empty");
        }

        let access_levels = value
            .chars()
            .map(|c| AccessLevel::try_from(c).ok())
            .take_while(Option::is_some)
            .flatten()
            .collect::<HashSet<_>>();

        // Safe as access levels are just ASCII
        let rest = &value[access_levels.len()..];

        let (nickname, username, hostname) = match (rest.find('!'), rest.find('@')) {
            (None, None) => (rest, None, None),
            (Some(i), None) => (&rest[..i], Some(rest[i + 1..].to_string()), None),
            (None, Some(i)) => (&rest[..i], None, Some(rest[i + 1..].to_string())),
            (Some(i), Some(j)) => (
                &rest[..i],
                Some(rest[i + 1..j].to_string()),
                Some(rest[j + 1..].to_string()),
            ),
        };

        Ok(User {
            nickname: Nick::from(nickname),
            username,
            hostname,
            access_levels,
            away: false,
        })
    }
}

impl From<User> for String {
    fn from(user: User) -> Self {
        let nick = user.nickname();

        match (user.username(), user.hostname()) {
            (None, None) => nick.to_string(),
            (None, Some(host)) => format!("{nick}@{host}",),
            (Some(user), None) => format!("{nick}!{user}"),
            (Some(user), Some(host)) => format!("{nick}!{user}@{host}"),
        }
    }
}

impl From<Nick> for User {
    fn from(nickname: Nick) -> Self {
        User {
            nickname,
            username: None,
            hostname: None,
            access_levels: HashSet::default(),
            away: false,
        }
    }
}

impl User {
    pub fn color_seed(&self, color: &buffer::Color) -> Option<String> {
        match color {
            buffer::Color::Solid => None,
            buffer::Color::Unique => Some(self.nickname().as_ref().to_string()),
        }
    }

    pub fn is_away(&self) -> bool {
        self.away
    }

    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    pub fn nickname(&self) -> NickRef {
        NickRef(&self.nickname.0)
    }

    pub fn hostname(&self) -> Option<&str> {
        self.hostname.as_deref()
    }

    pub fn with_nickname(self, nickname: Nick) -> Self {
        Self { nickname, ..self }
    }

    pub fn highest_access_level(&self) -> AccessLevel {
        self.access_levels
            .iter()
            .max()
            .copied()
            .unwrap_or(AccessLevel::Member)
    }

    pub fn update_access_level(&mut self, operation: mode::Operation, mode: mode::Channel) {
        if let Ok(level) = AccessLevel::try_from(mode) {
            match operation {
                mode::Operation::Add => {
                    self.access_levels.insert(level);
                }
                mode::Operation::Remove => {
                    self.access_levels.remove(&level);
                }
            }
        }
    }

    pub fn update_away(&mut self, away: bool) {
        self.away = away;
    }

    pub fn formatted(&self, user_format: UsernameFormat) -> String {
        let user = self.username();
        let host = self.hostname();
        let nick = self.nickname();

        match user_format {
            UsernameFormat::Short => nick.to_string(),
            UsernameFormat::Full => match (user, host) {
                (None, None) => nick.to_string(),
                (None, Some(host)) => format!("{nick} ({host})"),
                (Some(user), None) => format!("{nick} ({user})"),
                (Some(user), Some(host)) => format!("{nick} ({user}@{host})"),
            },
        }
    }
}

impl From<proto::User> for User {
    fn from(user: proto::User) -> Self {
        User {
            nickname: Nick::from(user.nickname),
            username: user.username,
            hostname: user.hostname,
            access_levels: HashSet::default(),
            away: false,
        }
    }
}

impl fmt::Display for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.nickname())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Nick(String);

impl fmt::Display for Nick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for Nick {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl From<String> for Nick {
    fn from(nick: String) -> Self {
        Nick(nick)
    }
}

impl<'a> From<&'a str> for Nick {
    fn from(nick: &'a str) -> Self {
        Nick(nick.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NickRef<'a>(&'a str);

impl<'a> From<&'a str> for NickRef<'a> {
    fn from(nick: &'a str) -> Self {
        NickRef(nick)
    }
}

impl<'a> NickRef<'a> {
    pub fn to_owned(self) -> Nick {
        Nick(self.0.to_string())
    }
}

impl<'a> fmt::Display for NickRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'a> AsRef<str> for NickRef<'a> {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl<'a> PartialOrd for NickRef<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.0.to_lowercase().cmp(&other.0.to_lowercase()))
    }
}

impl<'a> Ord for NickRef<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.to_lowercase().cmp(&other.0.to_lowercase())
    }
}

impl<'a> PartialEq<Nick> for NickRef<'a> {
    fn eq(&self, other: &Nick) -> bool {
        self.0.eq(other.0.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AccessLevel {
    Member,
    Voice,
    HalfOp,
    Oper,
    Admin,
    Owner,
}

impl std::fmt::Display for AccessLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let access_level = match self {
            AccessLevel::Owner => "~",
            AccessLevel::Admin => "&",
            AccessLevel::Oper => "@",
            AccessLevel::HalfOp => "%",
            AccessLevel::Voice => "+",
            AccessLevel::Member => "",
        };

        write!(f, "{}", access_level)
    }
}

impl TryFrom<char> for AccessLevel {
    type Error = ();

    fn try_from(c: char) -> Result<AccessLevel, ()> {
        match c {
            '~' => Ok(AccessLevel::Owner),
            '&' => Ok(AccessLevel::Admin),
            '@' => Ok(AccessLevel::Oper),
            '%' => Ok(AccessLevel::HalfOp),
            '+' => Ok(AccessLevel::Voice),
            _ => Err(()),
        }
    }
}

impl TryFrom<mode::Channel> for AccessLevel {
    type Error = ();

    fn try_from(mode: mode::Channel) -> Result<Self, Self::Error> {
        Ok(match mode {
            mode::Channel::Founder => Self::Owner,
            mode::Channel::Admin => Self::Admin,
            mode::Channel::Oper => Self::Oper,
            mode::Channel::Halfop => Self::HalfOp,
            mode::Channel::Voice => Self::Voice,
            _ => return Err(()),
        })
    }
}
