use std::sync::Arc;

use tokio::sync::Semaphore;
use tokio::time::{Duration, Instant};

pub struct BackoffInterval {
    duration: Duration,
    previous: Duration,
    original: Duration,
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
            original: duration,
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
                    std::cmp::max(self.duration.mul_f64(0.9), self.original);
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
                    self.duration.mul_f64(2.0),
                    256 * self.original,
                );
            }
        }
    }
}

#[derive(Clone)]
pub struct TokenBucket {
    duration: Duration,
    capacity: usize,
    semaphore: Arc<Semaphore>,
    last: Option<Instant>,
}

impl TokenBucket {
    pub fn new(duration: Duration, capacity: usize) -> Self {
        Self {
            duration,
            capacity,
            semaphore: Arc::new(Semaphore::new(capacity)),
            last: Some(Instant::now()),
        }
    }

    pub fn add_permit(&mut self, now: Instant) {
        if self.semaphore.available_permits() < self.capacity {
            if let Some(last) = self.last {
                if now.duration_since(last) >= self.duration {
                    self.semaphore.add_permits(1);
                    self.last = Some(now);
                }
            } else {
                self.last = Some(now);
            }
        } else {
            self.last = None;
        }
    }

    pub async fn acquire_permit(&self) {
        if let Ok(permit) = self.semaphore.acquire().await {
            // Do not release permit back to semaphore, forget it in order to
            // reduce the number of available tokens until they are refilled.
            permit.forget();
        }
    }
}
