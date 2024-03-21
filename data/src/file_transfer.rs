use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::user::Nick;
use crate::{dcc, server, Server};

pub use self::manager::Manager;
pub use self::task::Task;

pub mod manager;
pub mod task;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id(u32);

impl From<u32> for Id {
    fn from(value: u32) -> Self {
        Id(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTransfer {
    pub server: Server,
    pub created_at: DateTime<Utc>,
    pub direction: Direction,
    pub remote_user: Nick,
    pub secure: bool,
    pub filename: String,
    pub size: u64,
    pub status: Status,
}

impl PartialOrd for FileTransfer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FileTransfer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.created_at
            .cmp(&other.created_at)
            .reverse()
            .then_with(|| self.direction.cmp(&other.direction))
            .then_with(|| self.remote_user.cmp(&other.remote_user))
            .then_with(|| self.secure.cmp(&other.secure))
            .then_with(|| self.filename.cmp(&other.filename))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Direction {
    Sent,
    Received,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    // Waiting to get processed by
    Pending,
    Queued,
    Active { transferred: u64, elapsed: Duration },
    Completed { elapsed: Duration, sha256: String },
    Failed { error: String },
}

#[derive(Debug)]
pub struct ReceiveRequest {
    pub from: Nick,
    pub dcc_send: dcc::Send,
    pub server: Server,
    pub server_handle: server::Handle,
}
