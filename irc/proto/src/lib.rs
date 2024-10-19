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
pub const CHANNEL_PREFIXES: &[char] = &['#', '&', '+', '!'];

/// Reference: https://defs.ircdocs.horse/defs/chantypes
///
/// Channel types which should be used if the CHANTYPES ISUPPORT is not returned
pub const DEFAULT_CHANNEL_PREFIXES: &[char] = &['#', '&'];

/// https://modern.ircdocs.horse/#channels
///
/// Channel names are strings (beginning with specified prefix characters). Apart from the requirement of
/// the first character being a valid channel type prefix character; the only restriction on a channel name
/// is that it may not contain any spaces (' ', 0x20), a control G / BELL ('^G', 0x07), or a comma (',', 0x2C)
/// (which is used as a list item separator by the protocol).
pub const CHANNEL_BLACKLIST_CHARS: &[char] = &[',', '\u{07}', ','];

pub fn is_channel(target: &str, chantypes: &[char]) -> bool {
    target.starts_with(chantypes) && !target.contains(CHANNEL_BLACKLIST_CHARS)
}

// Reference: https://defs.ircdocs.horse/defs/chanmembers
pub const CHANNEL_MEMBERSHIP_PREFIXES: &[char] = &['~', '&', '!', '@', '%', '+'];

/// https://modern.ircdocs.horse/#channels
///
/// Given a target, split it into a channel name (beginning with a character in `chantypes`) and a
/// possible list of prefixes (given in `statusmsg_prefixes`). If these two lists overlap, the
/// behaviour is unspecified.
pub fn parse_channel_from_target(
    target: &str,
    chantypes: &[char],
    statusmsg_prefixes: &[char],
) -> Option<(Vec<char>, String)> {
    // We parse the target by finding the first character in chantypes, and returing (even if that
    // character is in statusmsg_prefixes)
    // If the characters before the first chantypes character are all valid prefixes, then we have
    // a valid channel name with those prefixes.    let chan_index = target.find(chantypes)?;
    let chan_index = target.find(chantypes)?;

    // will not panic, since `find` always returns a valid codepoint index
    let (prefix, chan) = target.split_at(chan_index);
    if prefix.chars().all(|ref c| statusmsg_prefixes.contains(c)) {
        Some((prefix.chars().collect(), chan.to_owned()))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_channel_correct() {
        let chantypes = DEFAULT_CHANNEL_PREFIXES;
        assert!(is_channel("#foo", chantypes));
        assert!(is_channel("&foo", chantypes));
        assert!(!is_channel("foo", chantypes));
    }

    #[test]
    fn empty_chantypes() {
        assert!(!is_channel("#foo", &[]));
        assert!(!is_channel("&foo", &[]));
    }

    #[test]
    fn parse_channel() {
        let chantypes = DEFAULT_CHANNEL_PREFIXES;
        let prefixes = CHANNEL_MEMBERSHIP_PREFIXES;
        assert_eq!(
            parse_channel_from_target("#foo", chantypes, prefixes),
            Some((vec![], "#foo".to_owned()))
        );
        assert_eq!(
            parse_channel_from_target("+%#foo", chantypes, prefixes),
            Some((vec!['+', '%'], "#foo".to_owned()))
        );
        assert_eq!(
            parse_channel_from_target("&+%foo", chantypes, prefixes),
            Some((vec![], "&+%foo".to_owned()))
        );
    }

    #[test]
    fn invalid_channels() {
        let chantypes = DEFAULT_CHANNEL_PREFIXES;
        let prefixes = CHANNEL_MEMBERSHIP_PREFIXES;
        assert!(parse_channel_from_target("+%foo", chantypes, prefixes).is_none());
    }
}
