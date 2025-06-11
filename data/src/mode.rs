use std::fmt;

use irc::proto;

use crate::isupport;
use crate::user::ProtectedPrefix;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode<T> {
    Add(T, Option<String>),
    Remove(T, Option<String>),
    NoPrefix(T),
}

impl<T> Mode<T> {
    pub fn value(&self) -> &T {
        match self {
            Mode::Add(value, _) => value,
            Mode::Remove(value, _) => value,
            Mode::NoPrefix(value) => value,
        }
    }

    pub fn operation(&self) -> Option<Operation> {
        match self {
            Mode::Add(_, _) => Some(Operation::Add),
            Mode::Remove(_, _) => Some(Operation::Remove),
            Mode::NoPrefix(_) => None,
        }
    }

    pub fn arg(&self) -> Option<&str> {
        match self {
            Mode::Add(_, arg) => arg.as_deref(),
            Mode::Remove(_, arg) => arg.as_deref(),
            Mode::NoPrefix(_) => None,
        }
    }
}

pub enum Operation {
    Add,
    Remove,
}

pub trait Parser: Copy {
    fn from_char(c: char) -> Self;
}

// Reference: https://defs.ircdocs.horse/defs/chanmodes

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Admin,
    Ban,
    BlockCaps,
    NoCTCP,
    DelayJoins,
    BanException,
    ChanFilter,
    StripBadWords,
    History,
    InviteOnly,
    InviteException,
    JoinThrottle,
    KickNoRejoin,
    KeyLock,
    NoKnock,
    Limit,
    Moderated,
    NoExternalMessages,
    NoNickChange,
    Permanent,
    RegisteredOnly,
    Secret,
    ProtectedTopic,
    NoNotice,
    NoInvite,
    AutoOp,
    ExemptChanOps,
    OperPrefix,
    OJoin,
    Founder,
    Protected(ProtectedPrefix),
    Oper,
    HalfOp,
    Voice,
    Unknown(char),
}

impl From<char> for Channel {
    fn from(c: char) -> Self {
        use Channel::*;

        match c {
            'a' => Admin,
            'b' => Ban,
            'B' => BlockCaps,
            'C' => NoCTCP,
            'D' => DelayJoins,
            'e' => BanException,
            'g' => ChanFilter,
            'G' => StripBadWords,
            'H' => History,
            'i' => InviteOnly,
            'I' => InviteException,
            'j' => JoinThrottle,
            'J' => KickNoRejoin,
            'k' => KeyLock,
            'K' => NoKnock,
            'l' => Limit,
            'm' => Moderated,
            'n' => NoExternalMessages,
            'N' => NoNickChange,
            'P' => Permanent,
            'r' => RegisteredOnly,
            's' => Secret,
            't' => ProtectedTopic,
            'T' => NoNotice,
            'V' => NoInvite,
            'w' => AutoOp,
            'X' => ExemptChanOps,
            'y' => OperPrefix,
            'Y' => OJoin,
            proto::FOUNDER_PREFIX => Founder,
            proto::PROTECTED_PREFIX_STD => Protected(ProtectedPrefix::Standard),
            proto::PROTECTED_PREFIX_ALT => {
                Protected(ProtectedPrefix::Alternative)
            }
            proto::OPERATOR_PREFIX => Oper,
            proto::HALF_OPERATOR_PREFIX => HalfOp,
            proto::VOICED_PREFIX => Voice,
            _ => Unknown(c),
        }
    }
}

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Channel::*;

        match self {
            Admin => write!(f, "Admin"),
            Ban => write!(f, "Ban"),
            BlockCaps => write!(f, "Block Caps"),
            NoCTCP => write!(f, "No CTCP"),
            DelayJoins => write!(f, "Delay Joins"),
            BanException => write!(f, "Ban Exception"),
            ChanFilter => write!(f, "Channel Filter"),
            StripBadWords => write!(f, "Strip Bad Words"),
            History => write!(f, "History"),
            InviteOnly => write!(f, "Invite Only"),
            InviteException => write!(f, "Invite Exception"),
            JoinThrottle => write!(f, "Join Throttle"),
            KickNoRejoin => write!(f, "Kick No-Rejoin"),
            KeyLock => write!(f, "Key Lock"),
            NoKnock => write!(f, "No Knock"),
            Limit => write!(f, "Limit"),
            Moderated => write!(f, "Moderated"),
            NoExternalMessages => write!(f, "No External Messages"),
            NoNickChange => write!(f, "No Nick Change"),
            Permanent => write!(f, "Permanent"),
            RegisteredOnly => write!(f, "Registered Only"),
            Secret => write!(f, "Secret"),
            ProtectedTopic => write!(f, "Protected Topic"),
            NoNotice => write!(f, "No Notice"),
            NoInvite => write!(f, "No Invite"),
            AutoOp => write!(f, "Automatic Channel Membership"),
            ExemptChanOps => write!(f, "Exempt Automatic Channel Membership"),
            OperPrefix => write!(f, "Operator Prefix"),
            OJoin => write!(f, "Operator with Prefix"),
            Founder => write!(f, "Founder"),
            Protected(_) => write!(f, "Protected"),
            Oper => write!(f, "Operator"),
            HalfOp => write!(f, "Half Operator"),
            Voice => write!(f, "Voice"),
            Unknown(_) => write!(f, "Unknown Mode"),
        }
    }
}

impl Parser for Channel {
    fn from_char(c: char) -> Self {
        Self::from(c)
    }
}

// Reference: https://defs.ircdocs.horse/defs/usermodes

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum User {
    ServerAdmin,
    Bot,
    CoAdmin,
    Deaf,
    External,
    RemoteClientConns,
    HideOper,
    Invisible,
    HideChans,
    Rej,
    NetworkAdmin,
    Spambots,
    GlobalOperator,
    LocalOperator,
    Registered,
    ServerNotices,
    UnAuth,
    WebTV,
    WAllOps,
    HostHiding,
    Unknown(char),
}

impl From<char> for User {
    fn from(c: char) -> Self {
        use User::*;

        match c {
            'A' => ServerAdmin,
            'B' => Bot,
            'C' => CoAdmin,
            'D' => Deaf,
            'e' => External,
            'F' => RemoteClientConns,
            'H' => HideOper,
            'i' => Invisible,
            'I' => HideChans,
            'j' => Rej,
            'N' => NetworkAdmin,
            'm' => Spambots,
            'o' => GlobalOperator,
            'O' => LocalOperator,
            'r' | 'R' => Registered,
            's' => ServerNotices,
            'u' => UnAuth,
            'V' => WebTV,
            'w' => WAllOps,
            'x' => HostHiding,
            _ => Unknown(c),
        }
    }
}

impl fmt::Display for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use User::*;

        match self {
            ServerAdmin => write!(f, "Server Administrator"),
            Bot => write!(f, "Bot"),
            CoAdmin => write!(f, "Co-Administrator"),
            Deaf => write!(f, "Deaf"),
            External => write!(f, "Receives Server Connection Information"),
            RemoteClientConns => {
                write!(f, "Receives Remote Client Connection Information")
            }
            HideOper => write!(f, "Hide Operator Membership"),
            Invisible => write!(f, "Invisible"),
            HideChans => write!(f, "Hide Channels in WHOIS"),
            Rej => write!(f, "Receives Rejected Client Information"),
            NetworkAdmin => write!(f, "Network Administrator"),
            Spambots => write!(f, "Receives Spambot Information"),
            GlobalOperator => write!(f, "Network-Wide Operator"),
            LocalOperator => write!(f, "Server-Wide Operator"),
            Registered => write!(f, "Registered"),
            ServerNotices => write!(f, "Receives Server Notices"),
            UnAuth => {
                write!(f, "Receives Unauthorized Client Connection Information")
            }
            WebTV => write!(f, "WebTV Client"),
            WAllOps => write!(f, "Receives WALLOPS Messages"),
            HostHiding => write!(f, "Hidden Host"),
            Unknown(_) => write!(f, "Unknown Mode"),
        }
    }
}

impl Parser for User {
    fn from_char(c: char) -> Self {
        Self::from(c)
    }
}

enum ModeSet<'a> {
    Plus(&'a str),
    Minus(&'a str),
    None(&'a str),
}

pub fn parse<T>(
    encoded: &str,
    args: &[String],
    chanmodes: &[isupport::ModeKind],
    prefix: &[isupport::PrefixMap],
) -> Vec<Mode<T>>
where
    T: Parser,
{
    let mut args = args.iter();
    let mut parsed = vec![];

    let mode_sets = match (encoded.find('+'), encoded.find('-')) {
        (None, None) => vec![ModeSet::None(encoded)],
        (None, Some(i)) => vec![ModeSet::Minus(&encoded[i + 1..])],
        (Some(i), None) => vec![ModeSet::Plus(&encoded[i + 1..])],
        (Some(p), Some(m)) => {
            let end_plus = if p > m { encoded.len() } else { m };
            let end_minus = if m > p { encoded.len() } else { p };

            vec![
                ModeSet::Plus(&encoded[p + 1..end_plus]),
                ModeSet::Minus(&encoded[m + 1..end_minus]),
            ]
        }
    };

    for mode_set in mode_sets {
        let modes = match mode_set {
            ModeSet::Plus(s) => s,
            ModeSet::Minus(s) => s,
            ModeSet::None(s) => s,
        };

        for c in modes.chars() {
            let value = T::from_char(
                prefix
                    .iter()
                    .find_map(|prefix_map| {
                        (prefix_map.mode == c).then_some(prefix_map.prefix)
                    })
                    .unwrap_or(c),
            );
            let arg = if takes_arg(c, &mode_set, chanmodes, prefix) {
                args.next().cloned()
            } else {
                None
            };

            let mode = match mode_set {
                ModeSet::Plus(_) => Mode::Add(value, arg),
                ModeSet::Minus(_) => Mode::Remove(value, arg),
                ModeSet::None(_) => Mode::NoPrefix(value),
            };

            parsed.push(mode);
        }
    }

    parsed
}

fn takes_arg(
    mode: char,
    mode_set: &ModeSet,
    chanmodes: &[isupport::ModeKind],
    prefix: &[isupport::PrefixMap],
) -> bool {
    let known = if let Some(kind) = chanmodes.iter().find_map(|chanmode| {
        if chanmode.modes.chars().any(|m| m == mode) {
            Some(chanmode.kind)
        } else {
            None
        }
    }) {
        match kind {
            'A' => Some(!matches!(mode_set, ModeSet::None(_))),
            'B' => Some(true),
            'C' => Some(matches!(mode_set, ModeSet::Plus(_))),
            'D' => Some(false),
            _ => None,
        }
    } else {
        prefix.iter().find_map(|prefix_map| {
            if prefix_map.mode == mode {
                Some(true)
            } else {
                None
            }
        })
    };

    known.unwrap_or(false)
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn channel() {
        let tests = [
            ("+r", vec![], vec![Mode::Add(Channel::RegisteredOnly, None)]),
            (
                "-rb+i",
                vec!["*@192.168.0.1".into()],
                // Adds are parsed first
                vec![
                    Mode::Add(Channel::InviteOnly, None),
                    Mode::Remove(Channel::RegisteredOnly, None),
                    Mode::Remove(Channel::Ban, Some("*@192.168.0.1".into())),
                ],
            ),
            ("b", vec![], vec![Mode::NoPrefix(Channel::Ban)]),
        ];

        let isupport = HashMap::<isupport::Kind, isupport::Parameter>::new();

        for (modes, args, expected) in tests {
            let modes = parse::<Channel>(
                modes,
                &args,
                isupport::get_chanmodes(&isupport),
                isupport::get_prefix(&isupport),
            );
            assert_eq!(modes, expected);
        }
    }
}
