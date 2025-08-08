use crate::User;
use crate::target::Channel;
use crate::user::Nick;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Notification {
    Connected,
    Disconnected,
    Reconnected,
    DirectMessage {
        user: User,
        message: String,
    },
    Highlight {
        user: User,
        channel: Channel,
        message: String,
        description: String,
    },
    FileTransferRequest {
        nick: Nick,
        filename: String,
    },
    MonitoredOnline(Vec<User>),
    MonitoredOffline(Vec<Nick>),
}
