#[derive(Debug, Clone)]
pub struct User(irc::client::data::User);

impl User {
    pub fn nickname(&self) -> &str {
        self.0.get_nickname()
    }

    pub fn highest_access_level(&self) -> AccessLevel {
        AccessLevel(self.0.highest_access_level())
    }
}

impl From<irc::client::data::User> for User {
    fn from(user: irc::client::data::User) -> Self {
        Self(user)
    }
}

#[derive(Debug, Clone)]
pub struct AccessLevel(irc::client::data::AccessLevel);

impl std::fmt::Display for AccessLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let access_level = match self.0 {
            irc::client::data::AccessLevel::Owner => "~",
            irc::client::data::AccessLevel::Admin => "&",
            irc::client::data::AccessLevel::Oper => "@",
            irc::client::data::AccessLevel::HalfOp => "%",
            irc::client::data::AccessLevel::Voice => "+",
            irc::client::data::AccessLevel::Member => "",
        };

        write!(f, "{}", access_level)
    }
}

impl From<irc::client::data::AccessLevel> for AccessLevel {
    fn from(access_level: irc::client::data::AccessLevel) -> Self {
        Self(access_level)
    }
}
