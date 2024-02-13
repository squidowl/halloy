use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};

use crate::user::Nick;
use crate::User;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Source {
    User(User),
    Server(Option<Server>),
    Action,
    Internal(Internal),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub enum Server {
    #[serde(rename = "joinwithnick")]
    Join(Option<Nick>),
    #[serde(rename = "partwithnick")]
    Part(Option<Nick>),
    #[serde(rename = "quitwithnick")]
    Quit(Option<Nick>),
}

impl<'de> Deserialize<'de> for Server {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Debug, Deserialize)]
        enum Mapping {
            #[serde(rename = "join")]
            JoinWithoutNick,
            #[serde(rename = "joinwithnick")]
            Join(Option<Nick>),
            #[serde(rename = "part")]
            PartWithoutNick,
            #[serde(rename = "partwithnick")]
            Part(Option<Nick>),
            #[serde(rename = "quit")]
            QuitWithoutNick,
            #[serde(rename = "quitwithnick")]
            Quit(Option<Nick>),
        }

        if let Ok(mapping) = Mapping::deserialize(deserializer) {
            match mapping {
                Mapping::JoinWithoutNick => Ok(Server::Join(None)),
                Mapping::Join(nick) => Ok(Server::Join(nick)),
                Mapping::PartWithoutNick => Ok(Server::Part(None)),
                Mapping::Part(nick) => Ok(Server::Part(nick)),
                Mapping::QuitWithoutNick => Ok(Server::Quit(None)),
                Mapping::Quit(nick) => Ok(Server::Quit(nick)),
            }
        } else {
            Err(D::Error::custom("could not map to Server enum"))
        }
    }
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
