use std::fmt;
use std::ops::Deref;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Name(String);

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Server {
    pub name: Name,
    pub hostname: String,
}

impl Server {
    pub fn new(name: impl ToString, hostname: impl ToString) -> Self {
        Self {
            name: Name(name.to_string()),
            hostname: hostname.to_string(),
        }
    }
}

impl fmt::Display for Server {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.name.0.fmt(f)
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Config(irc::client::data::Config);

impl From<irc::client::data::Config> for Config {
    fn from(config: irc::client::data::Config) -> Self {
        Self(config)
    }
}

impl Deref for Config {
    type Target = irc::client::data::Config;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
