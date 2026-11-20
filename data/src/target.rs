use std::hash::Hash;
use std::{cmp, fmt};

use ecow::{EcoVec, eco_vec};
use equivalent::Equivalent;
use irc::proto;
use irc::string::CaseMapped;
use serde::{Deserialize, Serialize};

use crate::user::{Nick, User};
use crate::{isupport, message};

#[derive(Debug, Clone, Copy)]
pub enum TargetRef<'a> {
    Channel(&'a Channel),
    Query(&'a Query),
}

impl PartialOrd for TargetRef<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TargetRef<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (
                TargetRef::Channel(channel),
                TargetRef::Channel(other_channel),
            ) => channel.cmp(other_channel),
            (TargetRef::Channel(_), TargetRef::Query(_)) => cmp::Ordering::Less,
            (TargetRef::Query(query), TargetRef::Query(other_query)) => {
                query.cmp(other_query)
            }
            (TargetRef::Query(_), TargetRef::Channel(_)) => {
                cmp::Ordering::Greater
            }
        }
    }
}

impl PartialEq for TargetRef<'_> {
    fn eq(&self, other: &TargetRef) -> bool {
        match (self, other) {
            (
                TargetRef::Channel(channel),
                TargetRef::Channel(other_channel),
            ) => channel.eq(other_channel),
            (TargetRef::Query(query), TargetRef::Query(other_query)) => {
                query.eq(other_query)
            }
            _ => false,
        }
    }
}

impl Eq for TargetRef<'_> {}

impl Hash for TargetRef<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            TargetRef::Channel(channel) => channel.hash(state),
            TargetRef::Query(query) => query.hash(state),
        }
    }
}

impl PartialEq<Target> for TargetRef<'_> {
    fn eq(&self, other: &Target) -> bool {
        match (self, other) {
            (TargetRef::Channel(channel), Target::Channel(other_channel)) => {
                (*channel).eq(other_channel)
            }
            (TargetRef::Query(query), Target::Query(other_query)) => {
                (*query).eq(other_query)
            }
            _ => false,
        }
    }
}

impl<'a> TargetRef<'a> {
    pub fn as_channel(&self) -> Option<&'a Channel> {
        match self {
            TargetRef::Channel(channel) => Some(channel),
            TargetRef::Query(_) => None,
        }
    }

    pub fn to_owned(self) -> Target {
        match self {
            TargetRef::Channel(channel) => Target::Channel(channel.clone()),
            TargetRef::Query(query) => Target::Query(query.clone()),
        }
    }
}

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

    pub fn to_channel(self) -> Option<Channel> {
        match self {
            Target::Channel(channel) => Some(channel),
            Target::Query(_) => None,
        }
    }

    pub fn as_query(&self) -> Option<&Query> {
        match self {
            Target::Channel(_) => None,
            Target::Query(query) => Some(query),
        }
    }

    pub fn to_query(self) -> Option<Query> {
        match self {
            Target::Channel(_) => None,
            Target::Query(query) => Some(query),
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

    pub fn as_targetref<'a>(&'a self) -> TargetRef<'a> {
        match self {
            Target::Channel(channel) => channel.as_targetref(),
            Target::Query(query) => query.as_targetref(),
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
                casemapped: CaseMapped::from_string(channel, casemapping),
            })
        } else {
            Target::Query(Query(CaseMapped::from_string(target, casemapping)))
        }
    }
}

impl PartialEq for Target {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Target::Channel(channel), Target::Channel(other_channel)) => {
                channel.eq(other_channel)
            }
            (Target::Query(query), Target::Query(other_query)) => {
                query.eq(other_query)
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
                channel.cmp(other_channel)
            }
            (Target::Channel(_), Target::Query(_)) => cmp::Ordering::Less,
            (Target::Query(query), Target::Query(other_query)) => {
                query.cmp(other_query)
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

impl From<Channel> for Target {
    fn from(channel: Channel) -> Self {
        Target::Channel(channel)
    }
}

impl From<Query> for Target {
    fn from(query: Query) -> Self {
        Target::Query(query)
    }
}

impl From<User> for Target {
    fn from(user: User) -> Self {
        Target::Query(Query::from(user))
    }
}

impl From<&User> for Target {
    fn from(user: &User) -> Self {
        Target::Query(Query::from(user))
    }
}

impl From<Nick> for Target {
    fn from(nick: Nick) -> Self {
        Target::Query(Query::from(nick))
    }
}

impl From<&Nick> for Target {
    fn from(nick: &Nick) -> Self {
        Target::Query(Query::from(nick))
    }
}

impl TryFrom<message::Target> for Target {
    type Error = ();

    fn try_from(target: message::Target) -> Result<Self, Self::Error> {
        Ok(match target {
            message::Target::Channel { channel } => Target::Channel(channel),
            message::Target::Query { query } => Target::Query(query),
            _ => return Err(()),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Channel {
    prefixes: EcoVec<char>,
    #[serde(flatten)]
    casemapped: CaseMapped,
}

impl Channel {
    pub fn as_normalized_str(&self) -> &str {
        self.casemapped.as_normalized_str()
    }

    pub fn as_str(&self) -> &str {
        self.casemapped.as_str()
    }

    pub fn from_str(
        target: &str,
        chantypes: &[char],
        casemapping: isupport::CaseMap,
    ) -> Self {
        if let Some(index) = target.find(chantypes) {
            // This will not panic, since `find` always returns a valid codepoint index.
            // We call `find` -> `split_at` because it is an _inclusive_ split, which includes the match.
            let (prefixes, channel) = target.split_at(index);

            Channel {
                prefixes: prefixes.chars().collect(),
                casemapped: CaseMapped::from_string(channel, casemapping),
            }
        } else {
            Channel {
                prefixes: eco_vec![],
                casemapped: CaseMapped::from_string(target, casemapping),
            }
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
                casemapped: CaseMapped::from_string(channel, casemapping),
            })
        } else {
            Err(ParseError::InvalidChannel(target.to_string()))
        }
    }

    pub fn renormalize(&mut self, casemapping: isupport::CaseMap) {
        self.casemapped.renormalize(casemapping);
    }

    pub fn prefixes(&self) -> &[char] {
        &self.prefixes
    }

    pub fn into_target(self) -> Target {
        Target::Channel(self)
    }

    pub fn to_target(&self) -> Target {
        Target::Channel(self.clone())
    }

    pub fn as_targetref<'a>(&'a self) -> TargetRef<'a> {
        TargetRef::Channel(self)
    }
}

impl PartialEq for Channel {
    fn eq(&self, other: &Self) -> bool {
        self.casemapped.eq(&other.casemapped)
    }
}

impl Eq for Channel {}

impl Hash for Channel {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.casemapped.hash(state);
    }
}

impl Ord for Channel {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.casemapped.cmp(&other.casemapped)
    }
}

impl PartialOrd for Channel {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.casemapped.fmt(f)
    }
}

#[derive(
    Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash, Ord, PartialOrd,
)]
pub struct Query(CaseMapped);

impl From<CaseMapped> for Query {
    fn from(inner: CaseMapped) -> Self {
        Query(inner)
    }
}

impl From<User> for Query {
    fn from(user: User) -> Self {
        Query::from(<User as std::convert::Into<Nick>>::into(user))
    }
}

impl From<&User> for Query {
    fn from(user: &User) -> Self {
        Query::from(user.nickname())
    }
}

impl From<&Query> for CaseMapped {
    fn from(query: &Query) -> Self {
        query.0.clone()
    }
}

impl From<Query> for CaseMapped {
    fn from(query: Query) -> Self {
        query.0
    }
}

impl From<Nick> for Query {
    fn from(nick: Nick) -> Self {
        Query(nick.into())
    }
}

impl From<&Nick> for Query {
    fn from(nick: &Nick) -> Self {
        Query(nick.into())
    }
}
impl Equivalent<User> for Query {
    fn equivalent(&self, user: &User) -> bool {
        self.as_normalized_str().eq(user.as_normalized_str())
    }
}

impl Query {
    pub fn as_normalized_str(&self) -> &str {
        self.0.as_normalized_str()
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
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
            Ok(Query(CaseMapped::from_string(target, casemapping)))
        }
    }

    pub fn renormalize(&mut self, casemapping: isupport::CaseMap) {
        self.0.renormalize(casemapping);
    }

    pub fn into_target(self) -> Target {
        Target::Query(self)
    }

    pub fn to_target(&self) -> Target {
        Target::Query(self.clone())
    }

    pub fn as_targetref<'a>(&'a self) -> TargetRef<'a> {
        TargetRef::Query(self)
    }
}

impl fmt::Display for Query {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("unable to parse channel from {0}")]
    InvalidChannel(String),
    #[error("unable to parse query from {0}")]
    InvalidQuery(String),
}

#[cfg(test)]
mod tests {
    use irc::proto::{CHANNEL_MEMBERSHIP_PREFIXES, DEFAULT_CHANNEL_PREFIXES};

    use super::*;
    use crate::isupport::CaseMap;

    #[test]
    fn parse_channel() {
        let chantypes = DEFAULT_CHANNEL_PREFIXES;
        let prefixes = CHANNEL_MEMBERSHIP_PREFIXES;
        let cm = CaseMap::default();
        assert_eq!(
            Channel::parse("#foo", chantypes, prefixes, cm).unwrap(),
            Channel {
                prefixes: eco_vec![],
                casemapped: CaseMapped::from_string("#foo", cm)
            }
        );
        assert_eq!(
            Channel::parse("+%#foo", chantypes, prefixes, cm).unwrap(),
            Channel {
                prefixes: eco_vec!['+', '%'],
                casemapped: CaseMapped::from_string("#foo", cm)
            }
        );
    }

    #[test]
    fn test_renormalize() {
        let chantypes = DEFAULT_CHANNEL_PREFIXES;
        let prefixes = CHANNEL_MEMBERSHIP_PREFIXES;
        let c =
            Channel::parse("+%#foo", chantypes, prefixes, CaseMap::default())
                .unwrap();
        let mut c2 = c.clone();
        c2.renormalize(CaseMap::default());
        assert_eq!(c, c2);
    }
}
