use icu_locale::Locale;
use jiff::fmt::rfc2822;
use jiff::tz::TimeZone;
use jiff::{self, Timestamp};

pub const RFC3339_WITH_MILLIS: &str = "%Y-%m-%dT%H:%M:%S%.3fZ";

/// Returns None if the time cannot be represented in the RFC 2822 format (e.g.
/// a negative year)
pub fn rfc2822_from_seconds(
    seconds: i64,
    time_zone: TimeZone,
) -> Result<String, jiff::Error> {
    Timestamp::from_second(seconds)
        .and_then(|timestamp| rfc2822_from_timestamp(timestamp, time_zone))
}

/// Returns None if the time cannot be represented in the RFC 2822 format (e.g.
/// a negative year)
pub fn rfc2822_from_timestamp(
    timestamp: Timestamp,
    time_zone: TimeZone,
) -> Result<String, jiff::Error> {
    rfc2822::to_string(&timestamp.to_zoned(time_zone))
}

pub fn system_locale() -> Locale {
    sys_locale::get_locale()
        .and_then(|locale_string| {
            Locale::try_from_str(&locale_string.replace("_", "-")).ok()
        })
        .unwrap_or(Locale::UNKNOWN)
}
