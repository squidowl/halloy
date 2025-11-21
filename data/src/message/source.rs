use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use self::server::Server;
use crate::{User, log};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Source {
    User(User),
    Server(Option<Server>),
    Action(Option<User>),
    Internal(Internal),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Internal {
    Status(Status),
    Logs(log::Level),
    Condensed(DateTime<Utc>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Status {
    Success,
    Error,
}

pub mod server {
    #![allow(deprecated)]
    use serde::{Deserialize, Serialize};

    use crate::user::Nick;

    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(untagged)]
    pub enum Server {
        #[deprecated(note = "use Server::Details")]
        Kind(Kind),
        Details(Details),
    }

    impl Server {
        pub fn new(kind: Kind, nick: Option<Nick>) -> Self {
            Self::Details(Details { kind, nick })
        }

        pub fn kind(&self) -> Kind {
            match self {
                Server::Kind(kind) => *kind,
                Server::Details(details) => details.kind,
            }
        }

        pub fn nick(&self) -> Option<&Nick> {
            match self {
                Server::Kind(_) => None,
                Server::Details(details) => details.nick.as_ref(),
            }
        }
    }

    #[derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        Hash,
        Serialize,
        Deserialize,
        strum::Display,
    )]
    #[serde(rename_all = "lowercase")]
    #[strum(serialize_all = "kebab-case")]
    pub enum Kind {
        Join,
        Part,
        Quit,
        #[strum(serialize = "topic")]
        ReplyTopic,
        ChangeHost,
        ChangeMode,
        ChangeNick,
        MonitoredOnline,
        MonitoredOffline,
        #[strum(to_string = "standard-reply-{0}")]
        StandardReply(StandardReply),
        WAllOps,
        Kick,
        ChangeTopic,
    }

    #[derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        Hash,
        Serialize,
        Deserialize,
        strum::Display,
    )]
    #[strum(serialize_all = "kebab-case")]
    pub enum StandardReply {
        Fail,
        Warn,
        Note,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Details {
        pub kind: Kind,
        pub nick: Option<Nick>,
    }
}
