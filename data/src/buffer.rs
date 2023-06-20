use crate::user::Nick;
use crate::Server;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Buffer {
    Server(Server),
    Channel(Server, String),
    Query(Server, Nick),
}
