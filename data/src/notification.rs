use crate::User;
use crate::target::Channel;
use crate::user::Nick;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Notification {
    Connected,
    Disconnected,
    Reconnected,
    DirectMessage(User),
    Highlight { user: User, channel: Channel },
    FileTransferRequest(Nick),
    MonitoredOnline(Vec<User>),
    MonitoredOffline(Vec<Nick>),
}
