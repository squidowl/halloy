use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::message::{
    Message, MessageReferences, MessageWithContext, Temporal, Time,
};
use crate::time::RFC3339_WITH_MILLIS;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Metadata {
    pub read_marker: Option<ReadMarker>,
    pub latest: Option<Timestamp>,
    pub latest_triggers_unread: Option<Timestamp>,
    pub latest_triggers_highlight: Option<Timestamp>,
    pub latest_chathistory_references: Option<(Time, MessageReferences)>,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    Deserialize,
    Serialize,
)]
pub struct ReadMarker(Timestamp);

impl From<Timestamp> for ReadMarker {
    fn from(timestamp: Timestamp) -> Self {
        Self(timestamp)
    }
}

impl From<&Time> for ReadMarker {
    fn from(time: &Time) -> Self {
        Self::from(time.utc)
    }
}

impl PartialEq<ReadMarker> for Time {
    fn eq(&self, other: &ReadMarker) -> bool {
        self.utc.eq(&other.0)
    }
}

impl PartialEq<Time> for ReadMarker {
    fn eq(&self, other: &Time) -> bool {
        self.0.eq(&other.utc)
    }
}

impl PartialOrd<ReadMarker> for Time {
    fn partial_cmp(&self, other: &ReadMarker) -> Option<Ordering> {
        Some(self.utc.cmp(&other.0))
    }
}

impl PartialOrd<Time> for ReadMarker {
    fn partial_cmp(&self, other: &Time) -> Option<Ordering> {
        Some(self.0.cmp(&other.utc))
    }
}

impl From<&Message> for ReadMarker {
    fn from(message: &Message) -> Self {
        Self::from(&message.time)
    }
}

impl From<&MessageWithContext> for ReadMarker {
    fn from(message_with_context: &MessageWithContext) -> Self {
        Self::from(message_with_context.time())
    }
}

impl ReadMarker {
    pub fn as_microsecond(&self) -> i64 {
        self.0.as_microsecond()
    }
}

impl FromStr for ReadMarker {
    type Err = jiff::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl fmt::Display for ReadMarker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Timestamp::strftime(&self.0, RFC3339_WITH_MILLIS).fmt(f)
    }
}

impl PartialEq<Timestamp> for ReadMarker {
    fn eq(&self, other: &Timestamp) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<ReadMarker> for Timestamp {
    fn eq(&self, other: &ReadMarker) -> bool {
        self.eq(&other.0)
    }
}

impl PartialOrd<Timestamp> for ReadMarker {
    fn partial_cmp(&self, other: &Timestamp) -> Option<Ordering> {
        Some(self.0.cmp(other))
    }
}

impl PartialOrd<ReadMarker> for Timestamp {
    fn partial_cmp(&self, other: &ReadMarker) -> Option<Ordering> {
        Some(self.cmp(&other.0))
    }
}
