use serde::{Deserialize, Serialize};

use crate::User;

pub use self::server::Server;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Source {
    User(User),
    Server(Option<Server>),
    Action,
    Internal(Internal),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Internal {
    Status(Status),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Status {
    Success,
    Error,
}

pub mod server {
    #![allow(deprecated)]
    use serde::{Deserialize, Serialize};

    use crate::time::Posix;
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
            text: Option<String>,
            time: Option<Posix>,
        ) -> Self {
            Self::Details(Details {
                kind,
                nick,
                text,
                time,
            })
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

        pub fn text(&self) -> Option<&str> {
            match self {
                Server::Kind(_) => None,
                Server::Details(details) => {
                    if let Some(text) = &details.text {
                        Some(text.as_str())
                    } else {
                        None
                    }
                }
            }
        }

        pub fn time(&self) -> Option<&Posix> {
            match self {
                Server::Kind(_) => None,
                Server::Details(details) => details.time.as_ref(),
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum Kind {
        Join,
        Part,
        Quit,
        ReplyTopic,
        ReplyTopicWhoTime,
        Topic,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Details {
        pub kind: Kind,
        pub nick: Option<Nick>,
        pub text: Option<String>,
        pub time: Option<Posix>,
    }
}
