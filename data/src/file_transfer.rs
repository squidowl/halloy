use std::path::PathBuf;
use std::time::Duration;

use chrono::{DateTime, Utc};

pub use self::manager::Manager;
pub use self::task::Task;
use crate::user::Nick;
use crate::{Server, dcc, server};

pub mod manager;
pub mod task;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id(u16);

impl From<u16> for Id {
    fn from(value: u16) -> Self {
        Id(value)
    }
}

impl From<Id> for u16 {
    fn from(id: Id) -> Self {
        id.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTransfer {
    pub id: Id,
    pub server: Server,
    pub created_at: DateTime<Utc>,
    pub direction: Direction,
    pub remote_user: Nick,
    pub filename: String,
    pub size: u64,
    pub status: Status,
}

impl FileTransfer {
    pub fn progress(&self) -> f64 {
        match self.status {
            Status::Active { transferred, .. } => {
                transferred as f64 / self.size as f64
            }
            Status::Completed { .. } => 1.0,
            _ => 0.0,
        }
    }
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
    /// Pending approval
    PendingApproval,
    /// Pending reverse confirmation
    PendingReverseConfirmation,
    /// Queued (needs an open port to begin)
    Queued,
    /// Ready (waiting for remote user to connect)
    Ready,
    /// Transfer is actively sending / receiving
    Active { transferred: u64, elapsed: Duration },
    /// Transfer is complete
    Completed { elapsed: Duration, sha256: String },
    /// An error occurred
    Failed { error: String },
}

#[derive(Debug, Clone)]
pub struct ReceiveRequest {
    pub from: Nick,
    pub dcc_send: dcc::Send,
    pub server: Server,
    pub server_handle: server::Handle,
}

#[derive(Debug)]
pub struct SendRequest {
    pub to: Nick,
    pub path: PathBuf,
    pub server: Server,
    pub server_handle: server::Handle,
}
