use icu_locale::{Locale, locale};
use jiff::fmt::rfc2822;
use jiff::tz::TimeZone;
use jiff::{self, SignedDuration, Timestamp};

#[cfg(target_os = "macos")]
mod apple;
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
mod unix;
#[cfg(target_os = "windows")]
mod windows;

/// This format is for use with IRCv3 features that utilize timestamps (e.g.
/// chathistory) and is expected to be used directly with Timestamp (or UTC
/// Zoned).
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

pub const OFFSET_WINDOW_LEN: usize = 16;

#[derive(Debug, Clone)]
pub struct ServerTimeTracking {
    latest_received: Option<Timestamp>,
    offset_window: Vec<SignedDuration>,
    offset_window_index: usize,
    median_offset: Option<SignedDuration>,
    latest_estimate: Option<(Timestamp, SignedDuration)>,
}

impl ServerTimeTracking {
    pub fn new(server_time: Timestamp) -> Self {
        let mut offset_window = Vec::with_capacity(OFFSET_WINDOW_LEN);

        let offset = Timestamp::now().duration_until(server_time);

        offset_window.push(offset);

        Self {
            latest_received: Some(server_time),
            offset_window,
            offset_window_index: 0,
            median_offset: Some(offset),
            latest_estimate: None,
        }
    }

    pub fn update(&mut self, server_time: Timestamp) {
        // Only update if the server-time is later than the latest recorded, to
        // avoid updating with any unmarked historical messages.
        if self
            .latest_received
            .is_none_or(|latest_received| server_time > latest_received)
        {
            self.latest_received = Some(server_time);

            let offset = Timestamp::now().duration_until(server_time);

            if self.offset_window.len() < OFFSET_WINDOW_LEN {
                self.offset_window.push(offset);
            } else {
                self.offset_window[self.offset_window_index] = offset;
                self.offset_window_index =
                    (self.offset_window_index + 1) % OFFSET_WINDOW_LEN;
            }

            self.median_offset = None;
        }
    }

    fn estimate_server_time(&mut self) -> Timestamp {
        // Use median offset within window in order to filter out transient lag
        // spikes.
        let (estimate, estimate_offset) = if let Some(latest_received) =
            self.latest_received
            && let Some(median_offset) = self.median_offset.or_else(|| {
                let mut offset_window = self.offset_window.clone();

                offset_window.sort();

                offset_window.into_iter().nth(self.offset_window.len() / 2)
            }) {
            let server_time =
                (Timestamp::now() + median_offset).max(latest_received);

            // If the estimated server time would before the previous estimate,
            // then re-estimate based on previous estimate instead (which we can
            // expect will be later than the previous estimate).
            if let Some((estimate, estimate_offset)) = self.latest_estimate
                && estimate > server_time
            {
                (Timestamp::now() + estimate_offset, estimate_offset)
            } else {
                self.median_offset = Some(median_offset);

                (server_time, median_offset)
            }
        } else {
            // No server-time has been received, use local clock.
            (Timestamp::now(), SignedDuration::new(0, 0))
        };

        self.latest_estimate = Some((estimate, estimate_offset));

        estimate
    }
}

/// Estimate server-time based on the median offset between server-time and the
/// local time it was received and last server time estimate.
pub fn estimate_server_time(
    server_time_tracking: Option<&mut ServerTimeTracking>,
) -> Timestamp {
    if let Some(server_time_tracking) = server_time_tracking {
        server_time_tracking.estimate_server_time()
    } else {
        Timestamp::now()
    }
}
