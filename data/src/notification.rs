use crate::target::Channel;
use crate::user::Nick;
use crate::{User, isupport};

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Notification {
    Connected,
    Disconnected,
    Reconnected,
    DirectMessage {
        user: User,
        casemapping: isupport::CaseMap,
        message: String,
    },
    Highlight {
        user: User,
        channel: Channel,
        casemapping: isupport::CaseMap,
        message: String,
        description: String,
        sound: Option<String>,
    },
    FileTransferRequest {
        nick: Nick,
        casemapping: isupport::CaseMap,
        filename: String,
    },
    MonitoredOnline(Vec<User>),
    MonitoredOffline(Vec<Nick>),
    Channel {
        user: User,
        channel: Channel,
        casemapping: isupport::CaseMap,
        message: String,
    },
}
