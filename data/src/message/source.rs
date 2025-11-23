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

    use crate::isupport;
    use crate::user::Nick;

    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(untagged)]
    pub enum Server {
        #[deprecated(note = "use Server::Details")]
        Kind(Kind),
        Details(Details),
    }

    impl Server {
        pub fn new(
            kind: Kind,
            nick: Option<Nick>,
            change: Option<Change>,
        ) -> Self {
            Self::Details(Details { kind, nick, change })
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

        pub fn change(&self) -> Option<&Change> {
            match self {
                Server::Kind(_) => None,
                Server::Details(details) => details.change.as_ref(),
            }
        }

        pub fn renormalize(&mut self, casemapping: isupport::CaseMap) {
            if let Server::Details(Details { nick, change, .. }) = self {
                if let Some(nick) = nick {
                    nick.renormalize(casemapping);
                }

                if let Some(Change::Nick(nick)) = change {
                    nick.renormalize(casemapping);
                }
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
        #[strum(serialize = "wallops")]
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
    pub enum Change {
        Nick(Nick),
        Host(String, String),
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Details {
        pub kind: Kind,
        pub nick: Option<Nick>,
        pub change: Option<Change>,
    }
}
