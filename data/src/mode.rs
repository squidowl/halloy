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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Ban,
    Exception,
    Limit,
    InviteOnly,
    InviteException,
    Key,
    Moderated,
    RegisteredOnly,
    Secret,
    ProtectedTopic,
    NoExternalMessages,
    Founder,
    Admin,
    Oper,
    Halfop,
    Voice,
    Unknown(char),
}

impl From<char> for Channel {
    fn from(c: char) -> Self {
        use Channel::*;

        match c {
            'b' => Ban,
            'e' => Exception,
            'l' => Limit,
            'i' => InviteOnly,
            'I' => InviteException,
            'k' => Key,
            'm' => Moderated,
            'r' => RegisteredOnly,
            's' => Secret,
            't' => ProtectedTopic,
            'n' => NoExternalMessages,
            'q' => Founder,
            'a' => Admin,
            'o' => Oper,
            'h' => Halfop,
            'v' => Voice,
            _ => Unknown(c),
        }
    }
}

impl Parser for Channel {
    fn takes_arg(self) -> bool {
        use Channel::*;

        matches!(
            self,
            Ban | Exception
                | Limit
                | InviteException
                | Key
                | Founder
                | Admin
                | Oper
                | Halfop
                | Voice
        )
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
