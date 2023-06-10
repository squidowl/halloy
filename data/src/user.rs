use irc::client::data;

use crate::config;

#[derive(Debug, Clone)]
pub struct User(data::User);

impl User {
    pub fn new(nick: &str, user: Option<&str>, host: Option<&str>) -> Self {
        let formatted = match (user, host) {
            (None, None) => nick.to_string(),
            (None, Some(host)) => format!("{nick}@{host}"),
            (Some(user), None) => format!("{nick}!{user}"),
            (Some(user), Some(host)) => format!("{nick}!{user}@{host}"),
        };

        Self(data::User::new(&formatted))
    }

    pub fn color_seed(&self, user_colors: &config::UserColor) -> Option<String> {
        match user_colors {
            config::UserColor::Solid => None,
            config::UserColor::Unique => {
                Some(self.hostname().unwrap_or(self.nickname()).to_string())
            }
        }
    }

    pub fn username(&self) -> Option<&str> {
        self.0.get_username()
    }

    pub fn nickname(&self) -> &str {
        self.0.get_nickname()
    }

    pub fn hostname(&self) -> Option<&str> {
        self.0.get_hostname()
    }

    pub fn highest_access_level(&self) -> AccessLevel {
        AccessLevel(self.0.highest_access_level())
    }
}

impl From<data::User> for User {
    fn from(user: data::User) -> Self {
        Self(user)
    }
}

#[derive(Debug, Clone)]
pub struct AccessLevel(data::AccessLevel);

impl std::fmt::Display for AccessLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let access_level = match self.0 {
            data::AccessLevel::Owner => "~",
            data::AccessLevel::Admin => "&",
            data::AccessLevel::Oper => "@",
            data::AccessLevel::HalfOp => "%",
            data::AccessLevel::Voice => "+",
            data::AccessLevel::Member => "",
        };

        write!(f, "{}", access_level)
    }
}

impl From<data::AccessLevel> for AccessLevel {
    fn from(access_level: data::AccessLevel) -> Self {
        Self(access_level)
    }
}
