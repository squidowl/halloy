use std::collections::VecDeque;

use tokio::time::{Duration, Instant};

pub struct BackoffInterval {
    duration: Duration,
    previous: Duration,
    minimum: Duration,
    mode: BackoffMode,
}

enum BackoffMode {
    Set,
    BackingOff(usize),
    EasingOn,
}

impl From<Duration> for BackoffInterval {
    fn from(duration: Duration) -> Self {
        BackoffInterval {
            duration,
            previous: duration,
            minimum: duration,
            mode: BackoffMode::Set,
        }
    }
}

impl BackoffInterval {
    pub fn duration(&self) -> Duration {
        self.duration
    }

    pub fn long_enough(&mut self) {
        match &mut self.mode {
            BackoffMode::Set => (),
            BackoffMode::EasingOn => {
                self.previous = self.duration;
                self.duration =
                    std::cmp::max(self.duration.mul_f64(0.9), self.minimum);

                if self.previous == self.duration {
                    self.mode = BackoffMode::Set;
                }
            }
            BackoffMode::BackingOff(count) => {
                *count += 1;

                if *count > 8 {
                    self.mode = BackoffMode::EasingOn;
                }
            }
        }
    }

    pub fn too_short(&mut self) {
        match &mut self.mode {
            BackoffMode::EasingOn => {
                self.duration = self.previous;
                self.mode = BackoffMode::Set;
            }
            _ => {
                self.mode = BackoffMode::BackingOff(0);
                self.duration = std::cmp::min(
                    self.duration.saturating_mul(2),
                    self.max_duration(),
                );
            }
        }
    }

    pub fn set_min(&mut self, duration: Duration) {
        // If the interval shows no sign of having had to back off, then attempt
        // to ease on
        if self.duration == self.minimum
            && matches!(self.mode, BackoffMode::Set)
        {
            self.mode = BackoffMode::EasingOn;
        }

        self.minimum = duration;
        self.duration = std::cmp::min(self.duration, self.max_duration());
        self.duration = std::cmp::max(self.duration, self.minimum);
    }

    fn max_duration(&self) -> Duration {
        self.minimum.saturating_mul(256)
    }
}

pub struct TokenBucket<T> {
    duration: Duration,
    capacity: usize,
    available_permits: usize,
    last: Option<Instant>,
    user_tokens: VecDeque<T>,
    high_priority_tokens: VecDeque<T>,
    low_priority_tokens: VecDeque<T>,
}

impl<T> TokenBucket<T> {
    pub fn new(duration: Duration, capacity: usize) -> Self {
        Self {
            duration,
            capacity,
            available_permits: capacity,
            last: Some(Instant::now()),
            user_tokens: VecDeque::new(),
            high_priority_tokens: VecDeque::new(),
            low_priority_tokens: VecDeque::new(),
        }
    }

    pub fn add_permit(&mut self, now: Instant) {
        if self.available_permits < self.capacity {
            if let Some(last) = self.last {
                if now.duration_since(last) >= self.duration {
                    self.available_permits += 1;
                    self.last = Some(now);
                }
            } else {
                self.last = Some(now);
            }
        } else {
            self.last = None;
        }
    }

    pub fn add_token(&mut self, token: T, token_priority: TokenPriority) {
        match token_priority {
            TokenPriority::User => self.user_tokens.push_back(token),
            TokenPriority::High => self.high_priority_tokens.push_back(token),
            TokenPriority::Low => self.low_priority_tokens.push_back(token),
        }
    }

    // Returns as many available tokens as are permitted
    pub fn acquire_tokens(&mut self) -> impl Iterator<Item = T> {
        let number_of_user_tokens =
            self.available_permits.min(self.user_tokens.len());

        self.available_permits =
            self.available_permits.saturating_sub(number_of_user_tokens);

        let number_of_high_priority_tokens =
            self.available_permits.min(self.high_priority_tokens.len());

        self.available_permits = self
            .available_permits
            .saturating_sub(number_of_high_priority_tokens);

        let number_of_low_priority_tokens =
            self.available_permits.min(self.low_priority_tokens.len());

        self.available_permits = self
            .available_permits
            .saturating_sub(number_of_low_priority_tokens);

        self.user_tokens
            .drain(..number_of_user_tokens)
            .chain(
                self.high_priority_tokens
                    .drain(..number_of_high_priority_tokens),
            )
            .chain(
                self.low_priority_tokens
                    .drain(..number_of_low_priority_tokens),
            )
    }

    // Returns all tokens, regardless of permit status
    pub fn drain_tokens(&mut self) -> impl Iterator<Item = T> {
        self.user_tokens
            .drain(..)
            .chain(self.high_priority_tokens.drain(..))
            .chain(self.low_priority_tokens.drain(..))
    }
}

pub enum TokenPriority {
    Low,  // Polls & other automated messages for retrieving metadata
    High, // Most automated messages
    User, // Messages that the user triggers directly
}
