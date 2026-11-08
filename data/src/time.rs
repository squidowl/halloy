use icu_locale::{Locale, locale};
use jiff::fmt::rfc2822;
use jiff::tz::TimeZone;
use jiff::{self, Timestamp};

#[cfg(target_os = "macos")]
mod apple;
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
mod unix;
#[cfg(target_os = "windows")]
mod windows;

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
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    let locale = crate::time::unix::get_time_locale();

    #[cfg(target_os = "macos")]
    let locale = crate::time::apple::get_locale();

    #[cfg(target_os = "windows")]
    let locale = crate::time::windows::get_locale();

    locale
        .and_then(|locale| Locale::try_from_str(&locale).ok())
        .unwrap_or(locale!("en"))
}

pub fn posix_to_bcp47(locale: &str) -> String {
    locale
        .chars()
        .take_while(|&c| c != '.' && c != '@')
        .map(|c| if c == '_' { '-' } else { c })
        .collect()
}
