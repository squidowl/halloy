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
    fn takes_arg(self) -> bool;
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
    Halfop,
    History,
    InviteOnly,
    InviteException,
    JoinThrottle,
    KickNoRejoin,
    Key,
    NoKnock,
    Limit,
    Moderated,
    NoExternalMessages,
    NoNickChange,
    Oper,
    Permanent,
    Founder,
    RegisteredOnly,
    Secret,
    ProtectedTopic,
    NoNotice,
    Voice,
    NoInvite,
    AutoOp,
    ExemptChanOps,
    OperPrefix,
    OJoin,
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
            'h' => Halfop,
            'H' => History,
            'i' => InviteOnly,
            'I' => InviteException,
            'j' => JoinThrottle,
            'J' => KickNoRejoin,
            'k' => Key,
            'K' => NoKnock,
            'l' => Limit,
            'm' => Moderated,
            'n' => NoExternalMessages,
            'N' => NoNickChange,
            'o' => Oper,
            'P' => Permanent,
            'q' => Founder,
            'r' => RegisteredOnly,
            's' => Secret,
            't' => ProtectedTopic,
            'T' => NoNotice,
            'v' => Voice,
            'V' => NoInvite,
            'w' => AutoOp,
            'X' => ExemptChanOps,
            'y' => OperPrefix,
            'Y' => OJoin,
            _ => Unknown(c),
        }
    }
}

impl Parser for Channel {
    fn takes_arg(self) -> bool {
        use Channel::*;

        matches!(
            self,
            Admin
                | Ban
                | BanException
                | ChanFilter
                | Halfop
                | History
                | JoinThrottle
                | InviteException
                | KickNoRejoin
                | Key
                | Limit
                | Oper
                | Founder
                | Voice
                | AutoOp
                | ExemptChanOps
        )
    }

    fn from_char(c: char) -> Self {
        Self::from(c)
    }
}

// Reference: https://defs.ircdocs.horse/defs/chanmodes

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

impl Parser for User {
    fn takes_arg(self) -> bool {
        use User::*;

        matches!(self, ServerNotices)
    }

    fn from_char(c: char) -> Self {
        Self::from(c)
    }
}

pub fn parse<T>(encoded: &str, args: &[String]) -> Vec<Mode<T>>
where
    T: Parser,
{
    enum Mod<'a> {
        Plus(&'a str),
        Minus(&'a str),
        None(&'a str),
    }

    let mut args = args.iter();
    let mut parsed = vec![];

    let mods = match (encoded.find('+'), encoded.find('-')) {
        (None, None) => vec![Mod::None(encoded)],
        (None, Some(i)) => vec![Mod::Minus(&encoded[i + 1..])],
        (Some(i), None) => vec![Mod::Plus(&encoded[i + 1..])],
        (Some(p), Some(m)) => {
            let end_plus = if p > m { encoded.len() } else { m };
            let end_minus = if m > p { encoded.len() } else { p };

            vec![
                Mod::Plus(&encoded[p + 1..end_plus]),
                Mod::Minus(&encoded[m + 1..end_minus]),
            ]
        }
    };

    for _mod in mods {
        let modes = match _mod {
            Mod::Plus(s) => s,
            Mod::Minus(s) => s,
            Mod::None(s) => s,
        };

        for c in modes.chars() {
            let value = T::from_char(c);
            let arg = if value.takes_arg() {
                args.next().cloned()
            } else {
                None
            };
            let mode = match _mod {
                Mod::Plus(_) => Mode::Add(value, arg),
                Mod::Minus(_) => Mode::Remove(value, arg),
                Mod::None(_) => Mode::NoPrefix(value),
            };
            parsed.push(mode);
        }
    }

    parsed
}

#[cfg(test)]
mod test {
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

        for (modes, args, expected) in tests {
            let modes = parse::<Channel>(modes, &args);
            assert_eq!(modes, expected);
        }
    }
}
