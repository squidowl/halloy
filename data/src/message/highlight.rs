use crate::{Message, User, target};

#[derive(Debug, Clone)]
pub struct Highlight {
    pub kind: Kind,
    pub channel: target::Channel,
    pub user: User,
    pub message: Message,
}

#[derive(Debug, Clone)]
pub enum Kind {
    Nick {
        sounds: Vec<String>,
    },
    Match {
        matching: String,
        sounds: Vec<String>,
    },
}
