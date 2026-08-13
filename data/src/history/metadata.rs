use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

use chrono::format::SecondsFormat;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::message::{
    Message, MessageReferences, MessageWithContext, Temporal, Time,
};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Metadata {
    pub read_marker: Option<ReadMarker>,
    pub latest: Option<DateTime<Utc>>,
    pub latest_triggers_unread: Option<DateTime<Utc>>,
    pub latest_triggers_highlight: Option<DateTime<Utc>>,
    pub chathistory_references: Option<MessageReferences>,
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
pub struct ReadMarker(DateTime<Utc>);

impl From<DateTime<Utc>> for ReadMarker {
    fn from(date_time: DateTime<Utc>) -> Self {
        Self(date_time)
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
    pub fn as_date_time(&self) -> &DateTime<Utc> {
        &self.0
    }
}

impl FromStr for ReadMarker {
    type Err = chrono::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.with_timezone(&Utc))
            .map(Self)
    }
}

impl fmt::Display for ReadMarker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.to_rfc3339_opts(SecondsFormat::Millis, true).fmt(f)
    }
}

impl PartialEq<DateTime<Utc>> for ReadMarker {
    fn eq(&self, other: &DateTime<Utc>) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<ReadMarker> for DateTime<Utc> {
    fn eq(&self, other: &ReadMarker) -> bool {
        self.eq(&other.0)
    }
}

impl PartialOrd<DateTime<Utc>> for ReadMarker {
    fn partial_cmp(&self, other: &DateTime<Utc>) -> Option<Ordering> {
        Some(self.0.cmp(other))
    }
}

impl PartialOrd<ReadMarker> for DateTime<Utc> {
    fn partial_cmp(&self, other: &ReadMarker) -> Option<Ordering> {
        Some(self.cmp(&other.0))
    }
}
