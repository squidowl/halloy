use std::fmt;

use crate::{isupport, user};

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
    fn from_char(
        c: char,
        prefix: &[isupport::PrefixMap],
        bot_mode_char: Option<char>,
        deaf_mode_char: Option<char>,
        invex_mode_char: Option<char>,
    ) -> Self;
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
    AccessLevel(user::AccessLevel),
    Unknown(char),
}

// These mappings, where not supplied via ISUPPORT, are from typical/common
// mode letters and cannot be relied upon when certainty is necessary.

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
            AccessLevel(access_level) => match access_level {
                user::AccessLevel::Member => write!(f, "Member"),
                user::AccessLevel::Founder => write!(f, "Founder"),
                user::AccessLevel::Protected(_) => write!(f, "Protected"),
                user::AccessLevel::Oper => write!(f, "Operator"),
                user::AccessLevel::HalfOp => write!(f, "Half Operator"),
                user::AccessLevel::Voice => write!(f, "Voice"),
                user::AccessLevel::Unknown(prefix) => {
                    write!(f, "Unknown Prefix ({prefix})")
                }
            },
            Unknown(_) => write!(f, "Unknown Mode"),
        }
    }
}

impl Parser for Channel {
    fn from_char(
        c: char,
        prefix: &[isupport::PrefixMap],
        _bot_mode_char: Option<char>,
        _deaf_mode_char: Option<char>,
        invex_mode_char: Option<char>,
    ) -> Self {
        use Channel::*;

        if invex_mode_char.is_some_and(|invex_mode_char| c == invex_mode_char) {
            InviteException
        } else {
            user::AccessLevel::try_from_mode_char(c, prefix).map_or(
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
                    _ => Unknown(c),
                },
                AccessLevel,
            )
        }
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

// These mappings, where not supplied via ISUPPORT, are from typical/common
// mode letters and cannot be relied upon when certainty is necessary.

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
    fn from_char(
        c: char,
        _prefix: &[isupport::PrefixMap],
        bot_mode_char: Option<char>,
        deaf_mode_char: Option<char>,
        _invex_mode_char: Option<char>,
    ) -> Self {
        use User::*;

        if bot_mode_char.is_some_and(|bot_mode_char| c == bot_mode_char) {
            Bot
        } else if deaf_mode_char
            .is_some_and(|deaf_mode_char| c == deaf_mode_char)
        {
            Deaf
        } else {
            match c {
                'A' => ServerAdmin,
                'C' => CoAdmin,
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
}

#[derive(Clone, Copy, PartialEq)]
enum Sign {
    Plus,
    Minus,
    None,
}

pub fn parse<T>(
    encoded: &str,
    args: &[String],
    chanmodes: &[isupport::ModeKind],
    prefix: &[isupport::PrefixMap],
    bot_mode_char: Option<char>,
    deaf_mode_char: Option<char>,
    invex_mode_char: Option<char>,
) -> Vec<Mode<T>>
where
    T: Parser,
{
    let mut args = args.iter();
    let mut parsed = vec![];

    // Each sign applies to the mode letters that follow it until the next
    // sign, and arguments are consumed in the same order the modes are
    // given (e.g. `-o+v foo bar` de-ops foo and voices bar).
    let mut sign = Sign::None;

    for c in encoded.chars() {
        match c {
            '+' => sign = Sign::Plus,
            '-' => sign = Sign::Minus,
            _ => {
                let value = T::from_char(
                    c,
                    prefix,
                    bot_mode_char,
                    deaf_mode_char,
                    invex_mode_char,
                );

                let arg = if takes_arg(c, sign, chanmodes, prefix) {
                    args.next().cloned()
                } else {
                    None
                };

                let mode = match sign {
                    Sign::Plus => Mode::Add(value, arg),
                    Sign::Minus => Mode::Remove(value, arg),
                    Sign::None => Mode::NoPrefix(value),
                };

                parsed.push(mode);
            }
        }
    }

    parsed
}

fn takes_arg(
    mode: char,
    sign: Sign,
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
            'A' => Some(sign != Sign::None),
            'B' => Some(true),
            'C' => Some(sign == Sign::Plus),
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
    use hashbrown::HashMap;

    use super::*;

    #[test]
    fn channel() {
        let tests = [
            ("+r", vec![], vec![Mode::Add(Channel::RegisteredOnly, None)]),
            (
                "-rb+i",
                vec!["*@192.168.0.1".into()],
                vec![
                    Mode::Remove(Channel::RegisteredOnly, None),
                    Mode::Remove(Channel::Ban, Some("*@192.168.0.1".into())),
                    Mode::Add(Channel::InviteOnly, None),
                ],
            ),
            ("b", vec![], vec![Mode::NoPrefix(Channel::Ban)]),
            // Args must pair up with modes in the order they are given
            (
                "-o+v",
                vec!["alice".into(), "bob".into()],
                vec![
                    Mode::Remove(
                        Channel::AccessLevel(user::AccessLevel::Oper),
                        Some("alice".into()),
                    ),
                    Mode::Add(
                        Channel::AccessLevel(user::AccessLevel::Voice),
                        Some("bob".into()),
                    ),
                ],
            ),
            // Signs may repeat multiple times
            (
                "+o-v+t",
                vec!["alice".into(), "bob".into()],
                vec![
                    Mode::Add(
                        Channel::AccessLevel(user::AccessLevel::Oper),
                        Some("alice".into()),
                    ),
                    Mode::Remove(
                        Channel::AccessLevel(user::AccessLevel::Voice),
                        Some("bob".into()),
                    ),
                    Mode::Add(Channel::ProtectedTopic, None),
                ],
            ),
            // Type B modes always take an arg, type C only when added,
            // type A only when signed
            (
                "+kl-bl",
                vec!["hunter2".into(), "10".into(), "*!*@*".into()],
                vec![
                    Mode::Add(Channel::KeyLock, Some("hunter2".into())),
                    Mode::Add(Channel::Limit, Some("10".into())),
                    Mode::Remove(Channel::Ban, Some("*!*@*".into())),
                    Mode::Remove(Channel::Limit, None),
                ],
            ),
        ];

        let isupport = HashMap::<isupport::Kind, isupport::Parameter>::new();

        for (modes, args, expected) in tests {
            let modes = parse::<Channel>(
                modes,
                &args,
                isupport::get_chanmodes_or_default(&isupport),
                isupport::get_prefix_or_default(&isupport),
                None,
                None,
                None,
            );
            assert_eq!(modes, expected);
        }
    }
}
