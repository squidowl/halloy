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

/// Reference: https://defs.ircdocs.horse/defs/chantypes
pub const CHANNEL_PREFIXES: [char; 4] = ['#', '&', '+', '!'];
/// https://modern.ircdocs.horse/#channels
///
/// Channel names are strings (beginning with specified prefix characters). Apart from the requirement of
/// the first character being a valid channel type prefix character; the only restriction on a channel name
/// is that it may not contain any spaces (' ', 0x20), a control G / BELL ('^G', 0x07), or a comma (',', 0x2C)
/// (which is used as a list item separator by the protocol).
pub const CHANNEL_BLACKLIST_CHARS: [char; 3] = [',', '\u{07}', ','];

pub fn is_channel(target: &str) -> bool {
    target.starts_with(CHANNEL_PREFIXES) && !target.contains(CHANNEL_BLACKLIST_CHARS)
}

// Reference: https://defs.ircdocs.horse/defs/chanmembers
pub const CHANNEL_MEMBERSHIP_PREFIXES: [char; 6] = ['~', '&', '!', '@', '%', '+'];

pub fn parse_channel_from_target(target: &str) -> Option<(Option<char>, String)> {
    if target.starts_with(CHANNEL_MEMBERSHIP_PREFIXES) {
        let channel = target.strip_prefix(CHANNEL_MEMBERSHIP_PREFIXES)?;

        if is_channel(channel) {
            return Some((target.chars().next(), channel.to_string()));
        }
    }

    if is_channel(target) {
        Some((None, target.to_string()))
    } else {
        None
    }
}

#[macro_export]
macro_rules! command {
    ($c:expr) => (
        $crate::command($c, vec![])
    );
    ($c:expr, $($p:expr),+ $(,)?) => (
        $crate::command($c, vec![$($p.into(),)*])
    );
}
