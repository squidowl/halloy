pub use self::command::Command;

pub mod command;
pub mod format;
pub mod parse;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub tags: Vec<Tag>,
    pub source: Option<Source>,
    pub command: Command,
}

impl From<Command> for Message {
    fn from(command: Command) -> Self {
        Self {
            tags: vec![],
            source: None,
            command,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub key: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Server(String),
    User(User),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub nickname: String,
    pub username: Option<String>,
    pub hostname: Option<String>,
}

pub fn command(command: &str, parameters: Vec<String>) -> Message {
    Message {
        tags: vec![],
        source: None,
        command: Command::new(command, parameters),
    }
}

pub const CHANNEL_PREFIXES: [char; 4] = ['#', '&', '+', '!'];

pub fn is_channel(target: &str) -> bool {
    target.starts_with(CHANNEL_PREFIXES)
}

pub const CHANNEL_MEMBERSHIP_PREFIXES: [char; 5] = ['~', '&', '@', '%', '+' ];

#[macro_export]
macro_rules! command {
    ($c:expr) => (
        $crate::command($c, vec![])
    );
    ($c:expr, $($p:expr),+ $(,)?) => (
        $crate::command($c, vec![$($p.into(),)*])
    );
}
