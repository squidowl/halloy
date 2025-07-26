use std::cmp::Reverse;
use std::collections::BTreeSet;
use std::fmt;
use std::hash::Hash;

use indexmap::{Equivalent, IndexSet};
use irc::proto;
use itertools::sorted;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config::buffer::UsernameFormat;
use crate::{isupport, mode};

#[derive(Debug, Clone)]
pub struct User {
    nickname: Nick,
    username: Option<String>,
    hostname: Option<String>,
    accountname: Option<String>,
    access_levels: BTreeSet<AccessLevel>,
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

// our list of users is essentially a hashmap with an order defined by access level
#[derive(Debug, Default)]
pub struct ChannelUsers(IndexSet<User>);

impl FromIterator<User> for ChannelUsers {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = User>,
    {
        let mut set: IndexSet<User> = iter.into_iter().collect();
        // we can't use `.sort_by_cached_key` here since it borrows a user.
        set.sort_by(|k1, k2| k1.key().cmp(&k2.key()));
        Self(set)
    }
}
impl<'a> IntoIterator for &'a ChannelUsers {
    type Item = &'a User;
    type IntoIter = indexmap::set::Iter<'a, User>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl ChannelUsers {
    pub fn iter(&self) -> indexmap::set::Iter<'_, User> {
        self.0.iter()
    }

    pub fn resolve(&self, user: &User) -> Option<&User> {
        self.0.get(user)
    }

    pub fn take(&mut self, user: &User) -> Option<User> {
        self.0.shift_take(user)
    }

    pub fn insert(&mut self, user: User) -> bool {
        // TODO(pounce, #1070) change to `insert_sorted_by_key` when merged
        let (Ok(i) | Err(i)) =
            self.0.binary_search_by_key(&user.key(), User::key);
        self.0.insert_before(i, user).1
    }

    pub fn remove(&mut self, user: &User) -> bool {
        self.0.shift_remove(user)
    }

    pub fn get_by_nick(&self, nick: NickRef) -> Option<&User> {
        self.0.get(&nick)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Serialize for User {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let access_levels: String = sorted(self.access_levels.iter())
            .map(ToString::to_string)
            .collect();
        let nickname = self.nickname();
        let username = self
            .username()
            .map(|username| format!("!{username}"))
            .unwrap_or_default();
        let hostname = self
            .hostname()
            .map(|hostname| format!("@{hostname}"))
            .unwrap_or_default();

        format!("{access_levels}{nickname}{username}{hostname}")
            .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for User {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;

        User::parse(&value, None, None).map_err(|_| {
            serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(&value),
                &"{access_levels}{nickname}{username}{hostname}",
            )
        })
    }
}

impl From<Nick> for User {
    fn from(nickname: Nick) -> Self {
        User {
            nickname,
            username: None,
            hostname: None,
            accountname: None,
            access_levels: BTreeSet::default(),
            away: false,
        }
    }
}

impl User {
    pub fn from_proto_user(
        user: proto::User,
        casemapping: isupport::CaseMap,
    ) -> Self {
        User {
            nickname: Nick::from_string(user.nickname, casemapping),
            username: user.username,
            hostname: user.hostname,
            accountname: None,
            access_levels: BTreeSet::default(),
            away: false,
        }
    }

    fn key(&'_ self) -> (Reverse<AccessLevel>, NickRef<'_>) {
        (
            Reverse(self.highest_access_level()),
            self.nickname.as_nickref(),
        )
    }

    pub fn seed(&self) -> &str {
        self.as_str()
    }

    pub fn display(
        &self,
        with_access_levels: bool,
        truncate: Option<u16>,
    ) -> String {
        let mut nickname = if with_access_levels {
            format!("{}{}", self.highest_access_level(), self.nickname())
        } else {
            self.nickname().to_string()
        };

        if let Some(len) = truncate {
            nickname = nickname.chars().take(len as usize).collect();
        }

        nickname
    }

    pub fn as_str(&self) -> &str {
        self.nickname.raw.as_ref()
    }

    pub fn as_normalized_str(&self) -> &str {
        self.nickname.normalized.as_ref()
    }

    pub fn is_away(&self) -> bool {
        self.away
    }

    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    pub fn nickname(&self) -> NickRef<'_> {
        self.nickname.as_nickref()
    }

    pub fn hostname(&self) -> Option<&str> {
        self.hostname.as_deref()
    }

    pub fn accountname(&self) -> Option<&str> {
        self.accountname.as_deref()
    }

    pub fn with_nickname(self, nickname: Nick) -> Self {
        Self { nickname, ..self }
    }

    pub fn with_username_and_hostname(
        self,
        username: String,
        hostname: String,
    ) -> Self {
        Self {
            username: Some(username),
            hostname: Some(hostname),
            ..self
        }
    }

    pub fn with_accountname(self, accountname: &str) -> Self {
        let accountname = if accountname == "*" || accountname == "0" {
            None
        } else {
            Some(accountname.to_string())
        };

        Self {
            accountname,
            ..self
        }
    }

    pub fn highest_access_level(&self) -> AccessLevel {
        self.access_levels
            // BTreeSet::last is the maximum element.
            .last()
            .copied()
            .unwrap_or(AccessLevel::Member)
    }

    pub fn has_access_level(&self, access_level: AccessLevel) -> bool {
        self.access_levels.contains(&access_level)
    }

    pub fn update_access_level(
        &mut self,
        operation: mode::Operation,
        mode: mode::Channel,
    ) {
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
            UsernameFormat::Mask => format!(
                "{}{}{}",
                self.nickname(),
                self.username()
                    .map(|username| format!("!{username}"))
                    .unwrap_or_default(),
                self.hostname()
                    .map(|hostname| format!("@{hostname}"))
                    .unwrap_or_default()
            ),
        }
    }

    /// Check if this user matches any of the provided mask patterns (regex).
    /// The user is converted to a mask string format (nickname!username@hostname) for comparison.
    pub fn matches_masks(&self, masks: &[String]) -> bool {
        use fancy_regex::Regex;

        let user_mask = self.formatted(UsernameFormat::Mask);

        masks.iter().any(|mask_pattern| {
            if let Ok(regex) = Regex::new(mask_pattern) {
                regex.is_match(&user_mask).unwrap_or(false)
            } else {
                false
            }
        })
    }

    pub fn parse(
        value: &str,
        casemapping: Option<isupport::CaseMap>,
        prefix: Option<&[isupport::PrefixMap]>,
    ) -> Result<Self, ParseUserError> {
        if value.is_empty() {
            return Err(ParseUserError::NicknameEmpty);
        }

        let index = if let Some(prefix) = prefix {
            value.find(|c: char| {
                prefix.iter().all(|prefix_map| c != prefix_map.prefix)
            })
        } else {
            value.find(|c: char| AccessLevel::try_from(c).is_err())
        };

        let Some(index) = index else {
            return Err(ParseUserError::NicknameEmpty);
        };

        let (access_levels, rest) = (&value[..index], &value[index..]);

        let access_levels = access_levels
            .chars()
            .filter_map(|c| AccessLevel::try_from(c).ok())
            .collect::<BTreeSet<_>>();

        let (nickname, username, hostname) =
            match (rest.find('!'), rest.find('@')) {
                (None, None) => (rest, None, None),
                (Some(i), None) => {
                    (&rest[..i], Some(rest[i + 1..].to_string()), None)
                }
                (None, Some(i)) => {
                    (&rest[..i], None, Some(rest[i + 1..].to_string()))
                }
                (Some(i), Some(j)) => {
                    if i < j {
                        (
                            &rest[..i],
                            Some(rest[i + 1..j].to_string()),
                            Some(rest[j + 1..].to_string()),
                        )
                    } else if let Some(k) = rest[i + 1..].find('@') {
                        (
                            &rest[..i],
                            Some(rest[i + 1..i + k + 1].to_string()),
                            Some(rest[i + k + 2..].to_string()),
                        )
                    } else {
                        (&rest[..i], Some(rest[i + 1..].to_string()), None)
                    }
                }
            };

        Ok(User {
            nickname: Nick::from_str(nickname, casemapping.unwrap_or_default()),
            username,
            hostname,
            accountname: None,
            access_levels,
            away: false,
        })
    }
}

#[derive(Error, Debug)]
pub enum ParseUserError {
    #[error("nickname can't be empty")]
    NicknameEmpty,
}

#[derive(Debug, Clone)]
pub struct NickColor {
    pub seed: Option<String>,
    pub color: iced_core::Color,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nick {
    raw: String,
    normalized: String,
}

impl PartialEq for Nick {
    fn eq(&self, other: &Self) -> bool {
        self.normalized.eq(&other.normalized)
    }
}

impl Eq for Nick {}

impl PartialOrd for Nick {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Nick {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.normalized.cmp(&other.normalized)
    }
}

impl Hash for Nick {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.normalized.hash(state);
    }
}

impl fmt::Display for Nick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.raw.fmt(f)
    }
}

impl From<NickRef<'_>> for Nick {
    fn from(nickref: NickRef) -> Self {
        Nick {
            raw: nickref.raw.to_string(),
            normalized: nickref.normalized.to_string(),
        }
    }
}

impl From<User> for Nick {
    fn from(user: User) -> Self {
        user.nickname
    }
}

impl From<Nick> for (String, String) {
    fn from(nick: Nick) -> Self {
        (nick.raw, nick.normalized)
    }
}

impl<'a> Nick {
    pub fn from_string(nick: String, casemapping: isupport::CaseMap) -> Self {
        Nick {
            normalized: casemapping.normalize(&nick),
            raw: nick,
        }
    }

    pub fn from_str(nick: &'a str, casemapping: isupport::CaseMap) -> Self {
        Nick {
            normalized: casemapping.normalize(nick),
            raw: nick.to_string(),
        }
    }

    pub fn as_nickref(&'a self) -> NickRef<'a> {
        NickRef {
            raw: self.raw.as_ref(),
            normalized: self.normalized.as_ref(),
        }
    }

    pub fn as_str(&self) -> &str {
        self.raw.as_ref()
    }

    pub fn as_normalized_str(&self) -> &str {
        self.normalized.as_ref()
    }

    pub fn renormalize(&mut self, casemapping: isupport::CaseMap) {
        self.normalized = casemapping.normalize(self.raw.as_str());
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NickRef<'a> {
    raw: &'a str,
    normalized: &'a str,
}

impl<'a> From<&'a Nick> for NickRef<'a> {
    fn from(nick: &'a Nick) -> Self {
        NickRef {
            raw: nick.raw.as_str(),
            normalized: nick.normalized.as_str(),
        }
    }
}

impl NickRef<'_> {
    pub fn to_owned(self) -> Nick {
        Nick {
            raw: self.raw.to_string(),
            normalized: self.normalized.to_string(),
        }
    }

    pub fn as_str(&self) -> &str {
        self.raw
    }

    pub fn as_normalized_str(&self) -> &str {
        self.normalized
    }
}

impl Equivalent<User> for NickRef<'_> {
    fn equivalent(&self, user: &User) -> bool {
        self.eq(&user.nickname.as_nickref())
    }
}

impl Hash for NickRef<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.normalized.hash(state);
    }
}

impl fmt::Display for NickRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.raw.fmt(f)
    }
}

impl PartialOrd for NickRef<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NickRef<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.normalized.cmp(other.normalized)
    }
}

impl PartialEq for NickRef<'_> {
    fn eq(&self, other: &NickRef) -> bool {
        self.normalized.eq(other.normalized)
    }
}

impl Eq for NickRef<'_> {}

impl PartialEq<Nick> for NickRef<'_> {
    fn eq(&self, other: &Nick) -> bool {
        self.normalized.eq(other.normalized.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AccessLevel {
    Member,
    Voice,
    HalfOp,
    Oper,
    Protected(ProtectedPrefix),
    Founder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProtectedPrefix {
    Standard,
    Alternative,
}

impl std::fmt::Display for AccessLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let access_level = match self {
            AccessLevel::Founder => Some(proto::FOUNDER_PREFIX),
            AccessLevel::Protected(prefix) => match prefix {
                ProtectedPrefix::Standard => Some(proto::PROTECTED_PREFIX_STD),
                ProtectedPrefix::Alternative => {
                    Some(proto::PROTECTED_PREFIX_ALT)
                }
            },
            AccessLevel::Oper => Some(proto::OPERATOR_PREFIX),
            AccessLevel::HalfOp => Some(proto::HALF_OPERATOR_PREFIX),
            AccessLevel::Voice => Some(proto::VOICED_PREFIX),
            AccessLevel::Member => None,
        };

        if let Some(access_level) = access_level {
            write!(f, "{access_level}")
        } else {
            write!(f, "")
        }
    }
}

impl TryFrom<char> for AccessLevel {
    type Error = ();

    fn try_from(c: char) -> Result<AccessLevel, ()> {
        match c {
            proto::FOUNDER_PREFIX => Ok(AccessLevel::Founder),
            proto::PROTECTED_PREFIX_STD => {
                Ok(AccessLevel::Protected(ProtectedPrefix::Standard))
            }
            proto::PROTECTED_PREFIX_ALT => {
                Ok(AccessLevel::Protected(ProtectedPrefix::Alternative))
            }
            proto::OPERATOR_PREFIX => Ok(AccessLevel::Oper),
            proto::HALF_OPERATOR_PREFIX => Ok(AccessLevel::HalfOp),
            proto::VOICED_PREFIX => Ok(AccessLevel::Voice),
            _ => Err(()),
        }
    }
}

impl TryFrom<mode::Channel> for AccessLevel {
    type Error = ();

    fn try_from(mode: mode::Channel) -> Result<Self, Self::Error> {
        Ok(match mode {
            mode::Channel::Founder => Self::Founder,
            mode::Channel::Protected(prefix) => Self::Protected(prefix),
            mode::Channel::Oper => Self::Oper,
            mode::Channel::HalfOp => Self::HalfOp,
            mode::Channel::Voice => Self::Voice,
            _ => return Err(()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_serde() {
        use serde_test::{Token, assert_tokens};

        let tests = [
            (
                User {
                    nickname: Nick::from_str(
                        "dan",
                        isupport::CaseMap::default(),
                    ),
                    username: None,
                    hostname: None,
                    accountname: None,
                    access_levels: BTreeSet::<AccessLevel>::from([
                        AccessLevel::Oper,
                        AccessLevel::Voice,
                    ]),
                    away: false,
                },
                [Token::String("+@dan")],
            ),
            (
                User {
                    nickname: Nick::from_str(
                        "dan",
                        isupport::CaseMap::default(),
                    ),
                    username: Some("d".into()),
                    hostname: Some("localhost".into()),
                    accountname: None,
                    access_levels: BTreeSet::<AccessLevel>::new(),
                    away: false,
                },
                [Token::String("dan!d@localhost")],
            ),
            (
                User {
                    nickname: Nick::from_str(
                        "d@n",
                        isupport::CaseMap::default(),
                    ),
                    username: Some("d".into()),
                    hostname: Some("localhost".into()),
                    accountname: None,
                    access_levels: BTreeSet::<AccessLevel>::from([
                        AccessLevel::Oper,
                    ]),
                    away: false,
                },
                [Token::String("@d@n!d@localhost")],
            ),
            (
                User {
                    nickname: Nick::from_str(
                        "foobar",
                        isupport::CaseMap::default(),
                    ),
                    username: None,
                    hostname: None,
                    accountname: None,
                    access_levels: BTreeSet::<AccessLevel>::new(),
                    away: false,
                },
                [Token::String("foobar")],
            ),
            (
                User {
                    nickname: Nick::from_str(
                        "foobar",
                        isupport::CaseMap::default(),
                    ),
                    username: Some("8a027a9a4a".into()),
                    hostname: Some(
                        "2201:12f1:2:1162:1242:1fg:he11:abde".into(),
                    ),
                    accountname: None,
                    access_levels: BTreeSet::<AccessLevel>::new(),
                    away: false,
                },
                [Token::String(
                    "foobar!8a027a9a4a@2201:12f1:2:1162:1242:1fg:he11:abde",
                )],
            ),
            (
                User {
                    nickname: Nick::from_str(
                        "foobar",
                        isupport::CaseMap::default(),
                    ),
                    username: Some("~foobar".into()),
                    hostname: Some("12.521.212.521".into()),
                    accountname: None,
                    access_levels: BTreeSet::<AccessLevel>::from([
                        AccessLevel::Oper,
                        AccessLevel::Voice,
                    ]),
                    away: false,
                },
                [Token::String("+@foobar!~foobar@12.521.212.521")],
            ),
            (
                User {
                    nickname: Nick::from_str(
                        "H1N5",
                        isupport::CaseMap::default(),
                    ),
                    username: Some("the.flu".into()),
                    hostname: Some("in.you".into()),
                    accountname: None,
                    access_levels: BTreeSet::<AccessLevel>::from([
                        AccessLevel::Oper,
                    ]),
                    away: false,
                },
                [Token::String("@H1N5!the.flu@in.you")],
            ),
            (
                User {
                    nickname: Nick::from_str(
                        "*status",
                        isupport::CaseMap::default(),
                    ),
                    username: None,
                    hostname: None,
                    accountname: None,
                    access_levels: BTreeSet::<AccessLevel>::new(),
                    away: false,
                },
                [Token::String("*status")],
            ),
            (
                User {
                    nickname: Nick::from_str(
                        "714user",
                        isupport::CaseMap::default(),
                    ),
                    username: None,
                    hostname: None,
                    accountname: None,
                    access_levels: BTreeSet::<AccessLevel>::from([
                        AccessLevel::Oper,
                    ]),
                    away: false,
                },
                [Token::String("@714user")],
            ),
        ];

        for (user, expected) in tests {
            assert_tokens(&user, &expected);
        }
    }

    #[test]
    fn matches_masks() {
        let user = User {
            nickname: Nick::from_str("alice", isupport::CaseMap::default()),
            username: Some("alice".into()),
            hostname: Some("example.com".into()),
            accountname: None,
            access_levels: BTreeSet::<AccessLevel>::new(),
            away: false,
        };

        // Test exact match
        assert!(user.matches_masks(&["alice!alice@example.com".to_string()]));

        // Test wildcard patterns
        assert!(user.matches_masks(&[".*@example.com".to_string()]));
        assert!(user.matches_masks(&["alice!.*@.*".to_string()]));
        assert!(user.matches_masks(&[".*!.*@example.com".to_string()]));

        // Test non-matching patterns
        assert!(!user.matches_masks(&["bob!bob@example.com".to_string()]));
        assert!(!user.matches_masks(&[".*@other.com".to_string()]));

        // Test multiple patterns (should match if any pattern matches)
        let patterns = vec![
            "bob!bob@example.com".to_string(),
            ".*@example.com".to_string(),
            "charlie!charlie@other.com".to_string(),
        ];
        assert!(user.matches_masks(&patterns));

        // Test empty patterns (should not match)
        assert!(!user.matches_masks(&[]));

        // Test invalid regex (should not match)
        assert!(!user.matches_masks(&["[invalid".to_string()]));
    }

    #[test]
    fn chanmap() {
        let users = &[
            ("dan", "+@dan"),
            ("d@n", "@d@n!d@localhost"),
            ("foobar", "+@foobar!~foobar@12.521.212.521"),
            ("*status", "*status"),
        ]
        .map(|(a, b)| {
            (
                Nick::from_str(a, isupport::CaseMap::default()),
                User::parse(b, None, None).unwrap(),
            )
        });
        let channel_users: ChannelUsers =
            users.iter().map(|(_, u)| u).cloned().collect();
        for (nick, user) in users {
            assert_eq!(
                channel_users.get_by_nick(nick.as_nickref()),
                Some(user)
            );
        }
    }
}
