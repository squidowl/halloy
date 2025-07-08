use std::collections::HashSet;
use std::fmt;
use std::hash::Hash;

use irc::proto;
use itertools::sorted;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config::buffer::UsernameFormat;
use crate::mode;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(into = "String")]
#[serde(try_from = "String")]
pub struct User {
    nickname: Nick,
    username: Option<String>,
    hostname: Option<String>,
    accountname: Option<String>,
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

#[derive(Error, Debug)]
pub enum TryFromUserError {
    #[error("nickname can't be empty")]
    NicknameEmpty,
    #[error("nickname must start with alphabetic or [ \\ ] ^ _ ` {{ | }} *")]
    NicknameInvalidCharacter,
}

impl TryFrom<String> for User {
    type Error = TryFromUserError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl<'a> TryFrom<&'a str> for User {
    type Error = TryFromUserError;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(Self::Error::NicknameEmpty);
        }

        let Some(index) = value.find(|c: char| {
            c.is_alphabetic() || "[\\]^_`{|}*".find(c).is_some()
        }) else {
            return Err(Self::Error::NicknameInvalidCharacter);
        };

        let (access_levels, rest) = (&value[..index], &value[index..]);

        let access_levels = access_levels
            .chars()
            .filter_map(|c| AccessLevel::try_from(c).ok())
            .collect::<HashSet<_>>();

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
            nickname: Nick::from(nickname),
            username,
            hostname,
            accountname: None,
            access_levels,
            away: false,
        })
    }
}

impl From<User> for String {
    fn from(user: User) -> Self {
        let access_levels: String = sorted(user.access_levels.iter())
            .map(ToString::to_string)
            .collect();
        let nickname = user.nickname();
        let username = user
            .username()
            .map(|username| format!("!{username}"))
            .unwrap_or_default();
        let hostname = user
            .hostname()
            .map(|hostname| format!("@{hostname}"))
            .unwrap_or_default();

        format!("{access_levels}{nickname}{username}{hostname}")
    }
}

impl From<Nick> for User {
    fn from(nickname: Nick) -> Self {
        User {
            nickname,
            username: None,
            hostname: None,
            accountname: None,
            access_levels: HashSet::default(),
            away: false,
        }
    }
}

impl User {
    pub fn seed(&self) -> &str {
        self.as_str()
    }

    pub fn display(&self, with_access_levels: bool) -> String {
        match with_access_levels {
            true => {
                format!("{}{}", self.highest_access_level(), self.nickname())
            }
            false => self.nickname().to_string(),
        }
    }

    pub fn as_str(&self) -> &str {
        self.nickname.as_ref()
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
            .iter()
            .max()
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
        }
    }

    /// Check if this user matches any of the provided mask patterns (regex).
    /// The user is converted to a mask string format (nickname!username@hostname) for comparison.
    pub fn matches_masks(&self, masks: &[String]) -> bool {
        use fancy_regex::Regex;

        let user_mask = String::from(self.clone());
        println!("user_mask: {user_mask}");

        masks.iter().any(|mask_pattern| {
            if let Ok(regex) = Regex::new(mask_pattern) {
                regex.is_match(&user_mask).unwrap_or(false)
            } else {
                false
            }
        })
    }
}

impl From<proto::User> for User {
    fn from(user: proto::User) -> Self {
        User {
            nickname: Nick::from(user.nickname),
            username: user.username,
            hostname: user.hostname,
            accountname: None,
            access_levels: HashSet::default(),
            away: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NickColor {
    pub seed: Option<String>,
    pub color: iced_core::Color,
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
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

impl NickRef<'_> {
    pub fn to_owned(self) -> Nick {
        Nick(self.0.to_string())
    }
}

impl fmt::Display for NickRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for NickRef<'_> {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl PartialOrd for NickRef<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.0.to_lowercase().cmp(&other.0.to_lowercase()))
    }
}

impl Ord for NickRef<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.to_lowercase().cmp(&other.0.to_lowercase())
    }
}

impl PartialEq<Nick> for NickRef<'_> {
    fn eq(&self, other: &Nick) -> bool {
        self.0.eq(other.0.as_str())
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
)]
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

        write!(f, "{access_level}")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_try_from() {
        let tests = [
            (
                User {
                    nickname: "dan".into(),
                    username: None,
                    hostname: None,
                    accountname: None,
                    access_levels: HashSet::<AccessLevel>::from([
                        AccessLevel::Oper,
                        AccessLevel::Voice,
                    ]),
                    away: false,
                },
                "+@dan",
            ),
            (
                User {
                    nickname: "d@n".into(),
                    username: Some("d".into()),
                    hostname: Some("localhost".into()),
                    accountname: None,
                    access_levels: HashSet::<AccessLevel>::from([
                        AccessLevel::Oper,
                    ]),
                    away: false,
                },
                "@d@n!d@localhost",
            ),
            (
                User {
                    nickname: "foobar".into(),
                    username: None,
                    hostname: None,
                    accountname: None,
                    access_levels: HashSet::<AccessLevel>::new(),
                    away: false,
                },
                "foobar",
            ),
            (
                User {
                    nickname: "foobar".into(),
                    username: Some("8a027a9a4a".into()),
                    hostname: Some(
                        "2201:12f1:2:1162:1242:1fg:he11:abde".into(),
                    ),
                    accountname: None,
                    access_levels: HashSet::<AccessLevel>::new(),
                    away: false,
                },
                "foobar!8a027a9a4a@2201:12f1:2:1162:1242:1fg:he11:abde",
            ),
            (
                User {
                    nickname: "foobar".into(),
                    username: Some("~foobar".into()),
                    hostname: Some("12.521.212.521".into()),
                    accountname: None,
                    access_levels: HashSet::<AccessLevel>::from([
                        AccessLevel::Oper,
                        AccessLevel::Voice,
                    ]),
                    away: false,
                },
                "+@foobar!~foobar@12.521.212.521",
            ),
        ];

        for (test, expected) in tests {
            let user = String::from(test);
            assert_eq!(user, expected);
        }
    }

    #[test]
    fn user_try_from() {
        let tests = [
            (
                "dan!d@localhost",
                User {
                    nickname: "dan".into(),
                    username: Some("d".into()),
                    hostname: Some("localhost".into()),
                    accountname: None,
                    access_levels: HashSet::<AccessLevel>::new(),
                    away: false,
                },
            ),
            (
                "$@H1N5!the.flu@in.you",
                User {
                    nickname: "H1N5".into(),
                    username: Some("the.flu".into()),
                    hostname: Some("in.you".into()),
                    accountname: None,
                    access_levels: HashSet::<AccessLevel>::from([
                        AccessLevel::Oper,
                    ]),
                    away: false,
                },
            ),
            (
                "d@n!d@localhost",
                User {
                    nickname: "d@n".into(),
                    username: Some("d".into()),
                    hostname: Some("localhost".into()),
                    accountname: None,
                    access_levels: HashSet::<AccessLevel>::new(),
                    away: false,
                },
            ),
            (
                "d@n!d",
                User {
                    nickname: "d@n".into(),
                    username: Some("d".into()),
                    hostname: None,
                    accountname: None,
                    access_levels: HashSet::<AccessLevel>::new(),
                    away: false,
                },
            ),
            (
                "*status",
                User {
                    nickname: "*status".into(),
                    username: None,
                    hostname: None,
                    accountname: None,
                    access_levels: HashSet::<AccessLevel>::new(),
                    away: false,
                },
            ),
        ];

        for (test, expected) in tests {
            let user = super::User::try_from(test).unwrap();

            assert_eq!(
                (
                    user.nickname,
                    user.username,
                    user.hostname,
                    user.access_levels
                ),
                (
                    expected.nickname,
                    expected.username,
                    expected.hostname,
                    expected.access_levels
                )
            );
        }
    }

    #[test]
    fn matches_masks() {
        let user = super::User::try_from("alice!alice@example.com ").unwrap();

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
}
