use serde::{Deserialize, Serialize};

use crate::User;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Source {
    User(User),
    Server(Option<Server>),
    Action,
    Internal(Internal),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Server {
    Join,
    Part,
    Quit,
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
