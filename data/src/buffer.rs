use crate::{Server, User};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Buffer {
    Server(Server),
    Channel(Server, String),
    Query(Server, User),
}
