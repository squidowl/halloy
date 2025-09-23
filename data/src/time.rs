use std::time::SystemTime;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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

    pub fn from_datetime(datetime: DateTime<Utc>) -> Self {
        if let Some(nanos_since_epoch) = datetime.timestamp_nanos_opt() {
            Self(u64::try_from(nanos_since_epoch).unwrap_or(0))
        } else {
            let micros_since_epoch =
                u64::try_from(datetime.timestamp_micros()).unwrap_or(0);

            Self(micros_since_epoch * 1_000)
        }
    }

    // This will saturate starting 2262-04-11T23:47:16.854775807
    pub fn datetime(&self) -> DateTime<Utc> {
        DateTime::from_timestamp_nanos(
            i64::try_from(self.0).unwrap_or(i64::MAX),
        )
    }
}
