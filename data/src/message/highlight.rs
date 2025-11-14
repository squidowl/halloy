use crate::{User, Message, target};

#[derive(Debug, Clone)]
pub struct Highlight {
    pub kind: Kind,
    pub channel: target::Channel,
    pub user: User,
    pub message: Message,
}

#[derive(Debug, Clone)]
pub enum Kind {
    Nick,
    Match {
        matching: String,
        sound: Option<String>,
    },
}
