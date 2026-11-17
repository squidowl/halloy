use std::cmp::Ordering;
use std::hash::Hash;

use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Time {
    pub utc: Timestamp,
    pub source: Source,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum Source {
    Server,
    Client,
}

impl Time {
    pub fn maybe_server(timestamp: Option<Timestamp>) -> Self {
        if let Some(timestamp) = timestamp {
            Self {
                utc: timestamp,
                source: Source::Server,
            }
        } else {
            Self {
                utc: Timestamp::now(),
                source: Source::Client,
            }
        }
    }

    pub fn client(timestamp: Timestamp) -> Self {
        Self {
            utc: timestamp,
            source: Source::Client,
        }
    }

    pub fn try_into_server_time(&self) -> Option<Timestamp> {
        matches!(self.source, Source::Server).then_some(self.utc)
    }

    pub fn system_date(&self) -> Date {
        self.utc.to_zoned(TimeZone::system()).date()
    }
}

impl PartialEq for Time {
    fn eq(&self, other: &Self) -> bool {
        self.utc == other.utc
    }
}

impl Eq for Time {}

impl PartialOrd for Time {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Time {
    fn cmp(&self, other: &Self) -> Ordering {
        self.utc.cmp(&other.utc)
    }
}

impl Hash for Time {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.utc.hash(state);
    }
}

impl PartialEq<Timestamp> for Time {
    fn eq(&self, other: &Timestamp) -> bool {
        self.utc.eq(other)
    }
}

impl PartialEq<Time> for Timestamp {
    fn eq(&self, other: &Time) -> bool {
        self.eq(&other.utc)
    }
}

impl PartialOrd<Timestamp> for Time {
    fn partial_cmp(&self, other: &Timestamp) -> Option<Ordering> {
        Some(self.utc.cmp(other))
    }
}

impl PartialOrd<Time> for Timestamp {
    fn partial_cmp(&self, other: &Time) -> Option<Ordering> {
        Some(self.cmp(&other.utc))
    }
}

impl PartialEq<Option<Timestamp>> for Time {
    fn eq(&self, other: &Option<Timestamp>) -> bool {
        other
            .as_ref()
            .is_some_and(|date_time| self.utc.eq(date_time))
    }
}

impl PartialEq<Time> for Option<Timestamp> {
    fn eq(&self, other: &Time) -> bool {
        self.as_ref()
            .is_some_and(|date_time| date_time.eq(&other.utc))
    }
}

impl PartialOrd<Option<Timestamp>> for Time {
    fn partial_cmp(&self, other: &Option<Timestamp>) -> Option<Ordering> {
        if let Some(date_time) = other.as_ref() {
            Some(self.utc.cmp(date_time))
        } else {
            Some(Ordering::Greater)
        }
    }
}

impl PartialOrd<Time> for Option<Timestamp> {
    fn partial_cmp(&self, other: &Time) -> Option<Ordering> {
        if let Some(date_time) = self.as_ref() {
            Some(date_time.cmp(&other.utc))
        } else {
            Some(Ordering::Less)
        }
    }
}
