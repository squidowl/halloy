use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Posix(u64);

impl Posix {
    pub fn now() -> Self {
        let nanos_since_epoch = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("valid unix timestamp")
            .as_nanos() as u64;

        Self(nanos_since_epoch)
    }
}