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

impl Source {
    pub fn user(&self) -> Option<&User> {
        match self {
            Source::User(user) | Source::Action(Some(user)) => Some(user),
            Source::Server(_) | Source::Action(None) | Source::Internal(_) => {
                None
            }
        }
    }
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
    use serde::{Deserialize, Serialize};

    use crate::isupport;
    use crate::user::Nick;

    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Server {
        pub kind: Kind,
        pub nick: Option<Nick>,
        pub change: Option<Change>,
    }

    impl Server {
        pub fn new(
            kind: Kind,
            nick: Option<Nick>,
            change: Option<Change>,
        ) -> Self {
            Self { kind, nick, change }
        }

        pub fn renormalize(&mut self, casemapping: isupport::CaseMap) {
            if let Some(nick) = self.nick.as_mut() {
                nick.renormalize(casemapping);
            }

            if let Some(Change::Nick(nick)) = self.change.as_mut() {
                nick.renormalize(casemapping);
            }
        }

        pub fn can_reference(&self) -> bool {
            match self.kind {
                Kind::Join
                | Kind::Part
                | Kind::Quit
                | Kind::ChangeHost
                | Kind::ChangeMode
                | Kind::ChangeNick
                | Kind::WAllOps
                | Kind::Kick
                | Kind::ChangeTopic => true,
                Kind::JoinTopic
                | Kind::MonitoredOnline
                | Kind::MonitoredOffline
                | Kind::StandardReply(_)
                | Kind::Away
                | Kind::Invite
                | Kind::RequestTopic => false,
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
        #[serde(rename = "replytopic")]
        JoinTopic,
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
        Away,
        Invite,
        RequestTopic,
    }

    impl Kind {
        pub fn is_action(&self) -> bool {
            match self {
                Kind::Join
                | Kind::Part
                | Kind::Quit
                | Kind::JoinTopic
                | Kind::ChangeHost
                | Kind::ChangeNick
                | Kind::Away => false,
                Kind::MonitoredOnline
                | Kind::MonitoredOffline
                | Kind::StandardReply(_)
                | Kind::WAllOps
                | Kind::Kick
                | Kind::ChangeMode
                | Kind::ChangeTopic
                | Kind::Invite
                | Kind::RequestTopic => true,
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
}
