use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Utc};

pub use self::manager::Manager;
pub use self::task::Task;
use crate::{Server, User, dcc, server};

pub mod manager;
pub mod task;

const FALLBACK_FILENAME: &str = "dcc_transfer";

pub fn sanitize_filename(raw: &str) -> String {
    let trimmed = raw.trim().trim_matches('"');
    let candidate = last_path_component(trimmed);

    if matches!(candidate, "" | "." | "..") {
        return FALLBACK_FILENAME.to_string();
    }

    replace_control_chars(candidate)
}

// Keep only the final path component so path traversal segments are ignored.
fn last_path_component(input: &str) -> &str {
    input
        .rsplit(['/', '\\'])
        .find(|segment| !segment.is_empty())
        .unwrap_or_default()
}

// Replace control characters to avoid problematic filenames.
fn replace_control_chars(input: &str) -> String {
    input
        .chars()
        .map(|c| if c.is_control() { '_' } else { c })
        .collect()
}

pub fn receive_save_path(save_directory: &Path, filename: &str) -> PathBuf {
    save_directory.join(sanitize_filename(filename))
}

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
    pub remote_user: User,
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
            .then_with(|| {
                self.remote_user
                    .nickname()
                    .cmp(&other.remote_user.nickname())
            })
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
    pub from: User,
    pub dcc_send: dcc::Send,
    pub server: Server,
    pub server_handle: server::Handle,
}

#[derive(Debug)]
pub struct SendRequest {
    pub to: User,
    pub path: PathBuf,
    pub server: Server,
    pub server_handle: server::Handle,
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{receive_save_path, sanitize_filename};

    #[test]
    fn sanitize_filename_strips_traversal_components() {
        assert_eq!(
            sanitize_filename("../../.ssh/authorized_keys"),
            "authorized_keys"
        );
        assert_eq!(sanitize_filename("..\\..\\Startup\\evil.exe"), "evil.exe");
    }

    #[test]
    fn sanitize_filename_replaces_invalid_or_empty_values() {
        assert_eq!(sanitize_filename(".."), "dcc_transfer");
        assert_eq!(sanitize_filename(""), "dcc_transfer");
        assert_eq!(
            sanitize_filename("name\u{0}with\u{1f}controls"),
            "name_with_controls"
        );
    }

    #[test]
    fn receive_save_path_stays_in_configured_directory() {
        let save_path = receive_save_path(
            Path::new("/home/victim/Downloads"),
            "../../../tmp/pwned",
        );

        assert_eq!(
            save_path,
            Path::new("/home/victim/Downloads").join("pwned")
        );
    }
}
