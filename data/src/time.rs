use std::time::SystemTime;

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Posix(u64);

impl Posix {
    pub fn now() -> Self {
        let nanos_since_epoch = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("valid unix timestamp")
            .as_nanos() as u64;

        Self(nanos_since_epoch)
    }

    pub fn from_seconds(seconds: u64) -> Self {
        Self(seconds * 1_000_000_000)
    }

    pub fn as_nanos(&self) -> u64 {
        self.0
    }

    pub fn datetime(&self) -> Option<DateTime<Utc>> {
        let seconds = (self.0 / 1_000_000_000) as i64;
        let nanos = (self.0 % 1_000_000_000) as u32;

        NaiveDateTime::from_timestamp_opt(seconds, nanos)
            .map(|datetime| Utc.from_utc_datetime(&datetime))
    }
}
