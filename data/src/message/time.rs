use std::cmp::Ordering;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Time {
    pub utc: DateTime<Utc>,
    pub source: Source,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum Source {
    Server,
    Client,
}

impl Time {
    pub fn server(date_time: Option<DateTime<Utc>>) -> Self {
        if let Some(date_time) = date_time {
            Self {
                utc: date_time,
                source: Source::Server,
            }
        } else {
            Self {
                utc: Utc::now(),
                source: Source::Client,
            }
        }
    }

    pub fn client(date_time: DateTime<Utc>) -> Self {
        Self {
            utc: date_time,
            source: Source::Client,
        }
    }

    pub fn try_into_server_time(&self) -> Option<DateTime<Utc>> {
        matches!(self.source, Source::Server).then_some(self.utc)
    }
}

impl PartialEq for Time {
    fn eq(&self, other: &Self) -> bool {
        self.utc == other.utc
    }
}

impl PartialEq<DateTime<Utc>> for Time {
    fn eq(&self, other: &DateTime<Utc>) -> bool {
        self.utc.eq(other)
    }
}

impl PartialEq<Time> for DateTime<Utc> {
    fn eq(&self, other: &Time) -> bool {
        self.eq(&other.utc)
    }
}

impl PartialEq<Option<DateTime<Utc>>> for Time {
    fn eq(&self, other: &Option<DateTime<Utc>>) -> bool {
        other
            .as_ref()
            .is_some_and(|date_time| self.utc.eq(date_time))
    }
}

impl PartialEq<Time> for Option<DateTime<Utc>> {
    fn eq(&self, other: &Time) -> bool {
        self.as_ref()
            .is_some_and(|date_time| date_time.eq(&other.utc))
    }
}

impl Eq for Time {}

impl Ord for Time {
    fn cmp(&self, other: &Self) -> Ordering {
        self.utc.cmp(&other.utc)
    }
}

impl PartialOrd for Time {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialOrd<DateTime<Utc>> for Time {
    fn partial_cmp(&self, other: &DateTime<Utc>) -> Option<Ordering> {
        Some(self.utc.cmp(other))
    }
}

impl PartialOrd<Time> for DateTime<Utc> {
    fn partial_cmp(&self, other: &Time) -> Option<Ordering> {
        Some(self.cmp(&other.utc))
    }
}

impl PartialOrd<Option<DateTime<Utc>>> for Time {
    fn partial_cmp(&self, other: &Option<DateTime<Utc>>) -> Option<Ordering> {
        if let Some(date_time) = other.as_ref() {
            Some(self.utc.cmp(date_time))
        } else {
            Some(Ordering::Greater)
        }
    }
}

impl PartialOrd<Time> for Option<DateTime<Utc>> {
    fn partial_cmp(&self, other: &Time) -> Option<Ordering> {
        if let Some(date_time) = self.as_ref() {
            Some(date_time.cmp(&other.utc))
        } else {
            Some(Ordering::Less)
        }
    }
}
