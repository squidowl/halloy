use std::hash::Hash;
use std::{cmp, fmt};

use irc::proto;
use serde::{Deserialize, Serialize};

use crate::isupport;
use crate::user::User;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Target {
    Channel(Channel),
    Query(Query),
}

impl Target {
    pub fn as_channel(&self) -> Option<&Channel> {
        match self {
            Target::Channel(channel) => Some(channel),
            Target::Query(_) => None,
        }
    }

    pub fn as_normalized_str(&self) -> &str {
        match self {
            Target::Channel(channel) => channel.as_normalized_str(),
            Target::Query(query) => query.as_normalized_str(),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Target::Channel(channel) => channel.as_str(),
            Target::Query(query) => query.as_str(),
        }
    }

    pub fn parse(
        target: &str,
        chantypes: &[char],
        statusmsg: &[char],
        casemapping: isupport::CaseMap,
    ) -> Self {
        if let Some((prefixes, channel)) =
            proto::parse_channel_from_target(target, chantypes, statusmsg)
        {
            Target::Channel(Channel {
                prefixes,
                normalized: casemapping.normalize(&channel),
                raw: target.to_string(),
            })
        } else {
            Target::Query(Query {
                normalized: casemapping.normalize(target),
                raw: target.to_string(),
            })
        }
    }
}

impl PartialEq for Target {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Target::Channel(channel), Target::Channel(other_channel)) => {
                channel.normalized.eq(&other_channel.normalized)
            }
            (Target::Query(query), Target::Query(other_query)) => {
                query.normalized.eq(&other_query.normalized)
            }
            _ => false,
        }
    }
}

impl Eq for Target {}

impl Hash for Target {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Target::Channel(channel) => channel.hash(state),
            Target::Query(query) => query.hash(state),
        }
    }
}

impl Ord for Target {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        match (self, other) {
            (Target::Channel(channel), Target::Channel(other_channel)) => {
                channel.normalized.cmp(&other_channel.normalized)
            }
            (Target::Channel(_), Target::Query(_)) => cmp::Ordering::Less,
            (Target::Query(query), Target::Query(other_query)) => {
                query.normalized.cmp(&other_query.normalized)
            }
            (Target::Query(_), Target::Channel(_)) => cmp::Ordering::Greater,
        }
    }
}

impl PartialOrd for Target {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Target::Channel(channel) => channel.fmt(f),
            Target::Query(query) => query.fmt(f),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Channel {
    prefixes: Vec<char>,
    normalized: String,
    raw: String,
}

impl Channel {
    pub fn as_normalized_str(&self) -> &str {
        self.normalized.as_ref()
    }

    pub fn as_str(&self) -> &str {
        self.raw.as_ref()
    }

    pub fn from_str(target: &str, casemapping: isupport::CaseMap) -> Self {
        if let Some(index) = target.find(proto::DEFAULT_CHANNEL_PREFIXES) {
            // This will not panic, since `find` always returns a valid codepoint index.
            // We call `find` -> `split_at` because it is an _inclusive_ split, which includes the match.
            let (prefixes, channel) = target.split_at(index);

            return Channel {
                prefixes: prefixes.chars().collect(),
                normalized: casemapping.normalize(channel),
                raw: target.to_string(),
            };
        }

        Channel {
            prefixes: vec![],
            normalized: casemapping.normalize(target),
            raw: target.to_string(),
        }
    }

    pub fn parse(
        target: &str,
        chantypes: &[char],
        statusmsg: &[char],
        casemapping: isupport::CaseMap,
    ) -> Result<Self, ParseError> {
        if let Some((prefixes, channel)) =
            proto::parse_channel_from_target(target, chantypes, statusmsg)
        {
            Ok(Channel {
                prefixes,
                normalized: casemapping.normalize(&channel),
                raw: target.to_string(),
            })
        } else {
            Err(ParseError::InvalidChannel(target.to_string()))
        }
    }

    pub fn prefixes(&self) -> &[char] {
        &self.prefixes
    }

    pub fn to_target(&self) -> Target {
        Target::Channel(self.clone())
    }
}

impl PartialEq for Channel {
    fn eq(&self, other: &Self) -> bool {
        self.normalized.eq(&other.normalized)
    }
}

impl Eq for Channel {}

impl Hash for Channel {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.normalized.hash(state);
    }
}

impl Ord for Channel {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.normalized.cmp(&other.normalized)
    }
}

impl PartialOrd for Channel {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.raw.fmt(f)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Query {
    normalized: String,
    raw: String,
}

impl Query {
    pub fn as_normalized_str(&self) -> &str {
        self.normalized.as_ref()
    }

    pub fn as_str(&self) -> &str {
        self.raw.as_ref()
    }

    pub fn from_user(user: &User, casemapping: isupport::CaseMap) -> Self {
        Query {
            normalized: casemapping.normalize(user.as_str()),
            raw: user.as_str().to_string(),
        }
    }

    pub fn parse(
        target: &str,
        chantypes: &[char],
        statusmsg: &[char],
        casemapping: isupport::CaseMap,
    ) -> Result<Self, ParseError> {
        if let Some((_, _)) =
            proto::parse_channel_from_target(target, chantypes, statusmsg)
        {
            Err(ParseError::InvalidQuery(target.to_string()))
        } else {
            Ok(Query {
                normalized: casemapping.normalize(target),
                raw: target.to_string(),
            })
        }
    }

    pub fn to_target(&self) -> Target {
        Target::Query(self.clone())
    }
}

impl PartialEq for Query {
    fn eq(&self, other: &Self) -> bool {
        self.normalized.eq(&other.normalized)
    }
}

impl Eq for Query {}

impl Hash for Query {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.normalized.hash(state);
    }
}

impl Ord for Query {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.normalized.cmp(&other.normalized)
    }
}

impl PartialOrd for Query {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Query {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.raw.fmt(f)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("unable to parse channel from {0}")]
    InvalidChannel(String),
    #[error("unable to parse query from {0}")]
    InvalidQuery(String),
}
